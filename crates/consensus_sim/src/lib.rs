//! Research orchestration. No mainnet transport and no external balance index.
use confidential_tx::{OutPoint, Output, Spend, Value};
use keys::{public, Account, Result};
use receiver_scan::{scan, spendable, View};
use std::collections::BTreeMap;
use utxo::{Block, Chain};

/// Recover a bounded explicit account range from canonical blocks, independently validating them.
/// An explicit bound is honest: no finite routine can discover arbitrary sparse account indices.
pub fn recover(
    seed: &[u8],
    account_count: u32,
    blocks: &[Block],
) -> Result<BTreeMap<OutPoint, Spend>> {
    if account_count > 1024 {
        return Err("recovery account budget");
    }
    let accounts = (0..account_count)
        .map(|i| Account::derive(seed, i))
        .collect::<Result<Vec<_>>>()?;
    let mut chain = Chain::default();
    let mut history: BTreeMap<OutPoint, Output> = BTreeMap::new();
    let mut wallet = BTreeMap::new();
    for b in blocks {
        chain.apply(b)?;
        for tx in &b.transactions {
            let prev = tx
                .inputs
                .iter()
                .map(|p| history.get(p).cloned().ok_or("recovery missing ancestry"))
                .collect::<Result<Vec<_>>>()?;
            for a in &accounts {
                // Infinite aggregates are valid consensus transactions but cannot use this receiving protocol.
                if let Ok(found) = scan(&View::from_account(a), tx, &prev) {
                    for item in found {
                        let coin = spendable(a, tx, item)?;
                        wallet.insert(coin.outpoint, coin);
                    }
                }
            }
            for (i, o) in tx.outputs.iter().enumerate() {
                let op = OutPoint {
                    txid: tx.txid(),
                    vout: i as u32,
                };
                history.insert(op, o.clone());
                recover_transparent(&accounts, op, o, &mut wallet);
            }
            for p in &tx.inputs {
                wallet.remove(p);
                history.remove(p);
            }
        }
        for (i, o) in b.coinbase.iter().enumerate() {
            let op = OutPoint {
                txid: b.coinbase_id(),
                vout: i as u32,
            };
            history.insert(op, o.clone());
            recover_transparent(&accounts, op, o, &mut wallet);
        }
    }
    Ok(wallet)
}
fn recover_transparent(
    accounts: &[Account],
    op: OutPoint,
    o: &Output,
    wallet: &mut BTreeMap<OutPoint, Spend>,
) {
    if let Value::Transparent(value) = o.value {
        for a in accounts {
            if public(&a.spend).x_only_public_key().0.serialize() == o.owner {
                wallet.insert(
                    op,
                    Spend {
                        outpoint: op,
                        output: o.clone(),
                        value,
                        blind: [0; 32],
                        secret: a.spend,
                    },
                );
            }
        }
    }
}
