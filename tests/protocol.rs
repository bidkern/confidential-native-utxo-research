mod common;
use address::Address;
use common::*;
use confidential_tx::*;
use keys::{public, Account};
use rand::RngCore;
use std::collections::BTreeSet;
use utxo::{subsidy, Block, Chain};

#[test]
fn independent_accounts_address_roundtrip_and_public_identifier_absence() {
    let addresses = (0..10)
        .map(|i| Address::from_account(&Account::derive(&[1; 32], i).unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(
        addresses
            .iter()
            .map(|a| a.scan.serialize())
            .collect::<BTreeSet<_>>()
            .len(),
        10
    );
    assert_eq!(
        addresses
            .iter()
            .map(|a| a.spend.serialize())
            .collect::<BTreeSet<_>>()
            .len(),
        10
    );
    for a in addresses {
        assert_eq!(Address::decode(&a.encode()).unwrap(), a);
    }
    assert!(Address::decode("cp1invalid").is_err());
    assert!(Account::derive(&[1; 32], 1 << 31).is_err());
}

#[test]
fn hundred_payments_distinct_scannable_and_seed_recoverable() {
    let (mut c, mut blocks, mut change) = initial();
    let receiver = Account::derive(&[2; 32], 0).unwrap();
    let sender = account();
    let mut rng = rng();
    let mut owners = BTreeSet::new();
    let mut receipt_ids = BTreeSet::new();
    let wrong = Account::derive(&[3; 32], 0).unwrap();
    for _ in 0..100 {
        let tx = build(
            std::slice::from_ref(&change),
            &[
                Payment::Confidential(Address::from_account(&receiver), 100_000),
                Payment::Confidential(Address::from_account(&sender), change.value - 101_000),
            ],
            1000,
            &mut rng,
        )
        .unwrap();
        let got = received(&receiver, &tx, std::slice::from_ref(&change.output));
        assert_eq!(got.len(), 1);
        assert!(owners.insert(got[0].output.owner));
        receipt_ids.insert(got[0].outpoint);
        assert!(received(&wrong, &tx, std::slice::from_ref(&change.output)).is_empty());
        change = received(&sender, &tx, &[change.output])[0].clone();
        let b = block(&c, vec![tx]);
        c.apply(&b).unwrap();
        blocks.push(b);
    }
    // Wallet database is discarded: only seed and public blocks enter recovery.
    let restored = consensus_sim::recover(&[2; 32], 1, &blocks).unwrap();
    assert_eq!(
        restored.keys().copied().collect::<BTreeSet<_>>(),
        receipt_ids
    );
    assert_eq!(restored.values().map(|s| s.value).sum::<u64>(), 10_000_000);
    let recovered_inputs = restored.values().take(2).cloned().collect::<Vec<_>>();
    let spend = build(
        &recovered_inputs,
        &[Payment::Confidential(
            Address::from_account(&receiver),
            199_000,
        )],
        1000,
        &mut rng,
    )
    .unwrap();
    c.apply(&block(&c, vec![spend])).unwrap();
    // Wrong-view test is not a cryptographic proof of unlinkability.
}

#[test]
fn ten_btc_split_fee_conserves_without_validator_openings() {
    let (mut c, _, input) = confidential();
    let a = account();
    let mut r = rng();
    let tx = build(
        std::slice::from_ref(&input),
        &[
            Payment::Confidential(Address::from_account(&a), 3 * BTC),
            Payment::Confidential(Address::from_account(&a), 699_999_000),
        ],
        1000,
        &mut r,
    )
    .unwrap();
    validate(&tx, std::slice::from_ref(&input.output), c.height + 1).unwrap();
    let round = Transaction::decode(&tx.bytes()).unwrap();
    assert_eq!(round, tx);
    assert_eq!(
        received(&a, &tx, &[input.output])
            .iter()
            .map(|s| s.value)
            .collect::<Vec<_>>(),
        vec![3 * BTC, 699_999_000]
    );
    c.apply(&block(&c, vec![tx])).unwrap();
}

#[test]
fn inflation_valid_signatures_valid_ranges_wrong_tally() {
    let (c, _, input) = confidential();
    let mut r = rng();
    let tx = build(
        std::slice::from_ref(&input),
        &[Payment::Confidential(
            Address::from_account(&account()),
            11 * BTC,
        )],
        0,
        &mut r,
    )
    .unwrap();
    assert_eq!(
        validate(&tx, &[input.output], c.height + 1),
        Err("value conservation")
    );
}

#[test]
fn range_edges_negative_equivalent_and_complement_attack() {
    let mut r = rng();
    let blind = random_secret(&mut r).secret_bytes();
    let bind = [9; 32];
    for value in [0, 1, commitments::MAX_MONEY] {
        let p = proofs::prove(
            value,
            blind,
            &bind,
            [random_secret(&mut r), random_secret(&mut r)],
        )
        .unwrap();
        let c = commitments::commit(value, blind).unwrap();
        proofs::verify(c, &p, &bind).unwrap();
        let negative_equivalent = commitments::commit(u64::MAX, blind).unwrap();
        assert!(proofs::verify(negative_equivalent, &p, &bind).is_err());
        let mut wrong = p.clone();
        wrong.complement = c;
        assert!(proofs::verify(c, &wrong, &bind).is_err());
        assert!(proofs::verify(c, &p, &[8; 32]).is_err());
    }
    assert!(proofs::prove(
        commitments::MAX_MONEY + 1,
        blind,
        &bind,
        [random_secret(&mut r), random_secret(&mut r)]
    )
    .is_err());
    assert!(commitments::parse(&[0; 33]).is_err());
    assert!(commitments::commit(0, [0; 32]).is_err());
}

#[test]
fn malformed_proof_cannot_mutate_utxo_state() {
    let (mut c, _, input) = confidential();
    let tx = build(
        &[input],
        &[Payment::Confidential(
            Address::from_account(&account()),
            10 * BTC,
        )],
        0,
        &mut rng(),
    )
    .unwrap();
    let mut bad = tx;
    bad.ranges[0].as_mut().unwrap().lower[20] ^= 1;
    let before = c.coins().clone();
    let tip = c.tip;
    assert!(c.apply(&block(&c, vec![bad])).is_err());
    assert_eq!(&before, c.coins());
    assert_eq!(tip, c.tip);
}

#[test]
fn explicit_double_spends_and_duplicate_inputs_rejected() {
    let (mut c, _, input) = confidential();
    let tx = build(
        std::slice::from_ref(&input),
        &[Payment::Confidential(
            Address::from_account(&account()),
            10 * BTC,
        )],
        0,
        &mut rng(),
    )
    .unwrap();
    let mut dup = tx.clone();
    dup.inputs.push(dup.inputs[0]);
    dup.signatures.push(dup.signatures[0]);
    assert_eq!(
        validate(&dup, &[input.output.clone(), input.output], 102),
        Err("duplicate input")
    );
    let before = c.coins().clone();
    assert!(c.apply(&block(&c, vec![tx.clone(), tx.clone()])).is_err());
    assert_eq!(&before, c.coins());
    c.apply(&block(&c, vec![tx.clone()])).unwrap();
    assert!(c.apply(&block(&c, vec![tx])).is_err());
}

#[test]
fn withdrawal_reorg_and_intrablock_undo_recover_exact_state() {
    let (mut c, blocks, input) = confidential();
    let owner = public(&account().spend).x_only_public_key().0.serialize();
    let tx = build(
        std::slice::from_ref(&input),
        &[Payment::Confidential(
            Address::from_account(&account()),
            10 * BTC,
        )],
        0,
        &mut rng(),
    )
    .unwrap();
    let next = received(&account(), &tx, &[input.output])[0].clone();
    let exit = build(
        &[next],
        &[Payment::Transparent(owner, 999_999_000)],
        1000,
        &mut rng(),
    )
    .unwrap();
    let before = c.coins().clone();
    let tip = c.tip;
    let b = block(&c, vec![tx, exit]);
    c.apply(&b).unwrap();
    c.disconnect().unwrap();
    assert_eq!(&before, c.coins());
    assert_eq!(tip, c.tip);
    let wallet = consensus_sim::recover(&[1; 32], 1, &blocks).unwrap();
    assert_eq!(wallet.len(), 1);
    c.apply(&b).unwrap();
}

#[test]
fn coinbase_issuance_maturity_and_fee_accounting() {
    let a = account();
    let owner = public(&a.spend).x_only_public_key().0.serialize();
    let mut c = Chain::default();
    let mut b = Block {
        previous: [0; 32],
        height: 1,
        coinbase: vec![Output {
            owner,
            value: Value::Transparent(subsidy(1) + 1),
        }],
        transactions: vec![],
    };
    assert!(c.apply(&b).is_err());
    b.coinbase[0].value = Value::Transparent(10 * BTC);
    c.apply(&b).unwrap();
    let input = Spend {
        outpoint: OutPoint {
            txid: b.coinbase_id(),
            vout: 0,
        },
        output: b.coinbase[0].clone(),
        value: 10 * BTC,
        blind: [0; 32],
        secret: a.spend,
    };
    let tx = build(
        &[input],
        &[Payment::Transparent(owner, 10 * BTC - 1000)],
        1000,
        &mut rng(),
    )
    .unwrap();
    assert_eq!(c.apply(&block(&c, vec![tx])), Err("immature coinbase"));
    let (mut c, _, input) = initial();
    let tx = build(
        &[input],
        &[Payment::Transparent(owner, 10 * BTC - 1000)],
        1000,
        &mut rng(),
    )
    .unwrap();
    let mut b = block(&c, vec![tx]);
    b.coinbase = vec![Output {
        owner,
        value: Value::Transparent(subsidy(b.height) + 1001),
    }];
    assert!(c.apply(&b).is_err());
    b.coinbase[0].value = Value::Transparent(subsidy(b.height) + 1000);
    c.apply(&b).unwrap();
    assert_eq!(subsidy(210000), 2_500_000_000);
    assert_eq!(subsidy(210000 * 64), 0);
}

#[test]
fn mutation_signature_binding_and_bounded_decoder() {
    let (_, _, input) = confidential();
    let tx = build(
        std::slice::from_ref(&input),
        &[Payment::Confidential(
            Address::from_account(&account()),
            10 * BTC,
        )],
        0,
        &mut rng(),
    )
    .unwrap();
    let mut bad = tx.clone();
    bad.fee = 1;
    assert!(validate(&bad, std::slice::from_ref(&input.output), 102).is_err());
    let mut bad = tx.clone();
    if let Value::Confidential {
        ref mut encrypted, ..
    } = bad.outputs[0].value
    {
        encrypted[0] ^= 1;
    }
    assert!(validate(&bad, std::slice::from_ref(&input.output), 102).is_err());
    assert!(received(&account(), &bad, &[input.output]).is_empty());
    let bytes = tx.bytes();
    for length in [0, 4, 7, 12, 30, bytes.len() - 1] {
        assert!(Transaction::decode(&bytes[..length]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(Transaction::decode(&trailing).is_err());
    let mut oversized = bytes.clone();
    oversized[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Transaction::decode(&oversized).is_err());
    let mut r = rng();
    for _ in 0..1000 {
        let mut b = vec![0; 256];
        r.fill_bytes(&mut b);
        assert!(Transaction::decode(&b).is_err());
    }
}

#[test]
fn deterministic_vectors_are_frozen() {
    let a = account();
    let addr = Address::from_account(&a);
    let frozen = include_str!("vectors/account.txt");
    assert_eq!(
        format!(
            "address={}\nscan={}\nspend={}\ngenerator={}\n",
            addr.encode(),
            hex::encode(addr.scan.serialize()),
            hex::encode(addr.spend.serialize()),
            hex::encode(commitments::generator().serialize())
        ),
        frozen
    );
}

#[test]
fn payment_wire_vector_is_frozen() {
    let (_, _, input) = confidential();
    let tx = build(
        &[input],
        &[
            Payment::Confidential(Address::from_account(&account()), 3 * BTC),
            Payment::Confidential(Address::from_account(&account()), 699_999_000),
        ],
        1000,
        &mut rng(),
    )
    .unwrap();
    assert_eq!(payment_vector(&tx), include_str!("vectors/payment.txt"));
}
fn payment_vector(tx: &Transaction) -> String {
    format!(
        "txid={}\nwtxid={}\nbase={}\nbytes={}\n",
        hex::encode(tx.txid()),
        hex::encode(tx.wtxid()),
        hex::encode(tx.base_bytes()),
        tx.bytes().len()
    )
}

#[test]
fn reject_valid_64_bit_proof_before_dependency_overflow() {
    let mut r = rng();
    let blind = random_secret(&mut r).secret_bytes();
    let bind = [1; 32];
    let c = commitments::commit(u64::MAX, blind).unwrap();
    let p = proofs::prove_one(u64::MAX, blind, c, &bind, random_secret(&mut r), false).unwrap();
    assert_eq!(
        proofs::verify_one(c, &p, &bind, false),
        Err("52-bit proof header")
    );
    let one = commitments::commit(1, blind).unwrap();
    let p = proofs::prove_one(1, blind, one, &bind, random_secret(&mut r), true).unwrap();
    assert!(proofs::verify_one(one, &p, &bind, true).is_err());
}

#[test]
fn independent_account_receipt_and_seed_recovery_range() {
    let (mut c, mut blocks, input) = initial();
    let a = Account::derive(&[2; 32], 1).unwrap();
    let tx = build(
        &[input],
        &[Payment::Confidential(Address::from_account(&a), 10 * BTC)],
        0,
        &mut rng(),
    )
    .unwrap();
    let b = block(&c, vec![tx]);
    c.apply(&b).unwrap();
    blocks.push(b);
    assert!(consensus_sim::recover(&[2; 32], 1, &blocks)
        .unwrap()
        .is_empty());
    assert_eq!(
        consensus_sim::recover(&[2; 32], 2, &blocks).unwrap().len(),
        1
    );
}

#[test]
fn malformed_excess_missing_ranges_invalid_owner_and_fee_overflow() {
    let (c, _, input) = confidential();
    let mut different_rng = rng();
    let _ = random_secret(&mut different_rng);
    let tx = build(
        std::slice::from_ref(&input),
        &[Payment::Confidential(
            Address::from_account(&account()),
            10 * BTC,
        )],
        0,
        &mut different_rng,
    )
    .unwrap();
    let check = |t: &Transaction| validate(t, std::slice::from_ref(&input.output), c.height + 1);
    let mut t = tx.clone();
    t.excess.as_mut().unwrap().proof[30] ^= 1;
    assert!(check(&t).is_err());
    let mut t = tx.clone();
    t.excess = None;
    assert_eq!(check(&t), Err("value conservation"));
    let mut t = tx.clone();
    t.ranges[0] = None;
    assert_eq!(check(&t), Err("proof/output mismatch"));
    let mut t = tx.clone();
    t.signatures[0][0] ^= 1;
    assert!(check(&t).is_err());
    let mut t = tx.clone();
    t.fee = u64::MAX;
    assert_eq!(check(&t), Err("fee/lock range"));
    let mut t = tx;
    t.outputs[0].owner = [255; 32];
    assert!(check(&t).is_err());
}

#[test]
fn transparent_negative_equivalent_and_confidential_coinbase_rejected() {
    let (mut c, _, input) = initial();
    let tx = build(
        &[input],
        &[Payment::Confidential(
            Address::from_account(&account()),
            10 * BTC,
        )],
        0,
        &mut rng(),
    )
    .unwrap();
    let mut b = block(&c, vec![]);
    b.coinbase.push(tx.outputs[0].clone());
    assert_eq!(c.apply(&b), Err("confidential coinbase prohibited"));
    b.coinbase[0].value = Value::Transparent(u64::MAX);
    assert_eq!(c.apply(&b), Err("coinbase value range"));
}
