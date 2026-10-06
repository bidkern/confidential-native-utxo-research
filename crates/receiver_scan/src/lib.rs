use confidential_tx::{context, decrypt_opening, OutPoint, Output, Spend, Transaction, Value};
use keys::{destination, public, shared, Account, Result};
use secp256k1_zkp::{PublicKey, Scalar, SecretKey};

pub struct View {
    pub scan: SecretKey,
    pub spend: PublicKey,
}
impl View {
    pub fn from_account(a: &Account) -> Self {
        Self {
            scan: a.scan,
            spend: public(&a.spend),
        }
    }
}
pub struct Incoming {
    pub index: u32,
    pub value: u64,
    pub blind: [u8; 32],
    pub tweak: SecretKey,
}
pub fn scan(view: &View, tx: &Transaction, prev: &[Output]) -> Result<Vec<Incoming>> {
    let (ctx, a) = context(&tx.inputs, prev)?;
    let s = shared(&view.scan, &a)?;
    let mut found = Vec::new();
    for (i, o) in tx.outputs.iter().enumerate() {
        if !matches!(o.value, Value::Confidential { .. }) {
            continue;
        }
        let (t, p) = destination(&s, &ctx, i as u32, &view.spend);
        if p == o.owner {
            // A malformed payment is not allowed to abort discovery of unrelated outputs.
            if let Ok((value, blind)) = decrypt_opening(&s, &ctx, i as u32, o) {
                found.push(Incoming {
                    index: i as u32,
                    value,
                    blind,
                    tweak: t,
                });
            }
        }
    }
    Ok(found)
}
pub fn spendable(account: &Account, tx: &Transaction, found: Incoming) -> Result<Spend> {
    let secret = account
        .spend
        .add_tweak(&Scalar::from(found.tweak))
        .map_err(|_| "zero spend key")?;
    let output = tx
        .outputs
        .get(found.index as usize)
        .ok_or("scan index")?
        .clone();
    if public(&secret).x_only_public_key().0.serialize() != output.owner {
        return Err("recovered ownership mismatch");
    }
    Ok(Spend {
        outpoint: OutPoint {
            txid: tx.txid(),
            vout: found.index,
        },
        output,
        value: found.value,
        blind: found.blind,
        secret,
    })
}
