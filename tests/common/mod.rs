#![allow(dead_code)]
use address::Address;
use confidential_tx::{build, OutPoint, Output, Payment, Spend, Transaction, Value};
use keys::{public, Account};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use receiver_scan::{scan, spendable, View};
use utxo::{Block, Chain};
pub const BTC: u64 = 100_000_000;
pub fn rng() -> ChaCha20Rng {
    ChaCha20Rng::from_seed([42; 32])
}
pub fn account() -> Account {
    Account::derive(&[1; 32], 0).unwrap()
}
pub fn block(chain: &Chain, txs: Vec<Transaction>) -> Block {
    Block {
        previous: chain.tip,
        height: chain.height + 1,
        coinbase: vec![],
        transactions: txs,
    }
}
pub fn initial() -> (Chain, Vec<Block>, Spend) {
    let a = account();
    let output = Output {
        owner: public(&a.spend).x_only_public_key().0.serialize(),
        value: Value::Transparent(10 * BTC),
    };
    let b = Block {
        previous: [0; 32],
        height: 1,
        coinbase: vec![output.clone()],
        transactions: vec![],
    };
    let spend = Spend {
        outpoint: OutPoint {
            txid: b.coinbase_id(),
            vout: 0,
        },
        output,
        value: 10 * BTC,
        blind: [0; 32],
        secret: a.spend,
    };
    let mut chain = Chain::default();
    chain.apply(&b).unwrap();
    let mut blocks = vec![b];
    while chain.height < 100 {
        let b = block(&chain, vec![]);
        chain.apply(&b).unwrap();
        blocks.push(b);
    }
    (chain, blocks, spend)
}
pub fn received(a: &Account, tx: &Transaction, prev: &[Output]) -> Vec<Spend> {
    scan(&View::from_account(a), tx, prev)
        .unwrap()
        .into_iter()
        .map(|i| spendable(a, tx, i).unwrap())
        .collect()
}
pub fn confidential() -> (Chain, Vec<Block>, Spend) {
    let (mut c, mut blocks, fund) = initial();
    let a = account();
    let mut r = rng();
    let tx = build(
        std::slice::from_ref(&fund),
        &[Payment::Confidential(Address::from_account(&a), 10 * BTC)],
        0,
        &mut r,
    )
    .unwrap();
    let coin = received(&a, &tx, &[fund.output])[0].clone();
    let b = block(&c, vec![tx]);
    c.apply(&b).unwrap();
    blocks.push(b);
    (c, blocks, coin)
}
