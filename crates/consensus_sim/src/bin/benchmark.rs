#[path = "../../../../tests/common/mod.rs"]
mod common;
use address::Address;
use common::*;
use confidential_tx::*;
use receiver_scan::{scan, View};
use std::{hint::black_box, time::Instant};

fn timed(name: &str, n: usize, mut f: impl FnMut()) {
    f();
    let mut samples = Vec::new();
    for _ in 0..n {
        let start = Instant::now();
        f();
        samples.push(start.elapsed().as_secs_f64() * 1e6);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "{name},us,{n},{:.3},{:.3},{:.3}",
        samples[n / 2],
        samples[0],
        samples[n - 1]
    );
}
fn metric(name: &str, v: usize) {
    println!("{name},bytes,1,{v},{v},{v}");
}
fn main() {
    println!("metric,unit,samples,median,min,max");
    let mut rng = rng();
    let blind = random_secret(&mut rng).secret_bytes();
    let bind = [7; 32];
    let commitment = commitments::commit(3 * BTC, blind).unwrap();
    let nonces = [random_secret(&mut rng), random_secret(&mut rng)];
    let p = proofs::prove(3 * BTC, blind, &bind, nonces).unwrap();
    timed("commitment_creation", 100, || {
        black_box(commitments::commit(3 * BTC, blind).unwrap());
    });
    timed("two_range_proofs_generation", 20, || {
        black_box(proofs::prove(3 * BTC, blind, &bind, nonces).unwrap());
    });
    timed("two_range_proofs_verification", 100, || {
        proofs::verify(commitment, &p, &bind).unwrap();
    });
    timed("serial_batch_100_output_bounds", 5, || {
        for _ in 0..100 {
            proofs::verify(commitment, &p, &bind).unwrap();
        }
    });
    metric("single_range_proof", p.lower.len());
    metric(
        "output_range_witness",
        33 + 8 + p.lower.len() + p.upper.len(),
    );
    let (chain, _, input) = confidential();
    let a = account();
    let addr = Address::from_account(&a);
    let view = View::from_account(&a);
    let tx = build(
        std::slice::from_ref(&input),
        &[
            Payment::Confidential(addr.clone(), 3 * BTC),
            Payment::Confidential(addr.clone(), 699_999_000),
        ],
        1000,
        &mut rng,
    )
    .unwrap();
    let prev = vec![input.output.clone()];
    metric("one_input_two_output_transaction", tx.bytes().len());
    metric("base_body", tx.base_bytes().len());
    metric("excess_proof", tx.excess.as_ref().unwrap().proof.len());
    timed("scan_one_transaction_two_outputs", 100, || {
        black_box(scan(&view, &tx, &prev).unwrap());
    });
    timed("validate_one_transaction", 20, || {
        validate(&tx, &prev, chain.height + 1).unwrap();
    });
    metric(
        "confidential_coin_serialized_excluding_outpoint",
        tx.outputs[0].bytes().len() + 5,
    );
    let transparent = Output {
        owner: tx.outputs[0].owner,
        value: Value::Transparent(3 * BTC),
    };
    metric(
        "transparent_coin_serialized_excluding_outpoint",
        transparent.bytes().len() + 5,
    );
    metric(
        "additional_serialized_utxo_bytes",
        tx.outputs[0].bytes().len() - transparent.bytes().len(),
    );
    metric("in_memory_coin_enum", std::mem::size_of::<utxo::Coin>());
    let mut current = input;
    let mut txs = Vec::new();
    let mut previous = Vec::new();
    for _ in 0..100 {
        let t = build(
            std::slice::from_ref(&current),
            &[
                Payment::Confidential(addr.clone(), 100_000),
                Payment::Confidential(addr.clone(), current.value - 101_000),
            ],
            1000,
            &mut rng,
        )
        .unwrap();
        let next = received(&a, &t, std::slice::from_ref(&current.output))
            .pop()
            .unwrap();
        previous.push(vec![current.output]);
        current = next;
        txs.push(t);
    }
    let b = block(&chain, txs);
    metric("synthetic_100_transaction_block", b.bytes_len());
    timed("scan_block_100_transactions_one_account", 10, || {
        for (t, p) in b.transactions.iter().zip(&previous) {
            black_box(scan(&view, t, p).unwrap());
        }
    });
    timed("validate_block_100_confidential_transactions", 5, || {
        let mut c = chain.clone();
        c.apply(&b).unwrap();
    });
    let (c, _, mut input) = initial();
    let owner = input.output.owner;
    let mut txs = Vec::new();
    for _ in 0..100 {
        let t = build(
            std::slice::from_ref(&input),
            &[
                Payment::Transparent(owner, 100_000),
                Payment::Transparent(owner, input.value - 101_000),
            ],
            1000,
            &mut rng,
        )
        .unwrap();
        input = Spend {
            outpoint: OutPoint {
                txid: t.txid(),
                vout: 1,
            },
            output: t.outputs[1].clone(),
            value: input.value - 101_000,
            blind: [0; 32],
            secret: input.secret,
        };
        txs.push(t);
    }
    let tb = block(&c, txs);
    metric(
        "synthetic_100_transparent_transaction_block",
        tb.bytes_len(),
    );
    timed("validate_block_100_transparent_transactions", 5, || {
        let mut copy = c.clone();
        copy.apply(&tb).unwrap();
    });
}
