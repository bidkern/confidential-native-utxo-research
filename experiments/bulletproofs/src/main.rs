//! Isolated Ristretto comparator; never linked to Bitcoin consensus.
use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek::scalar::Scalar;
use merlin::Transcript;
use rand::RngCore;
use std::time::Instant;
const MAX: u64 = 2_100_000_000_000_000;
fn main() {
    let pc = PedersenGens::default();
    let bp = BulletproofGens::new(64, 64);
    let mut rows = Vec::new();
    for outputs in [1usize, 2, 8, 24] {
        let k = (2 * outputs).next_power_of_two();
        let mut values = Vec::new();
        let mut blinds = Vec::new();
        for i in 0..outputs {
            let v = if i == 0 {
                0
            } else if i == 1 {
                MAX
            } else {
                123456789
            };
            let mut bytes = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut bytes);
            let blind = Scalar::from_bytes_mod_order(bytes);
            values.extend([v, MAX - v]);
            blinds.extend([blind, -blind]);
        }
        values.resize(k, 0);
        blinds.resize(k, Scalar::ZERO);
        let start = Instant::now();
        let (proof, commits) = RangeProof::prove_multiple(
            &bp,
            &pc,
            &mut Transcript::new(b"CNU/comparator/v0"),
            &values,
            &blinds,
            64,
        )
        .unwrap();
        let prove_ms = start.elapsed().as_secs_f64() * 1000.;
        for pair in commits[..2 * outputs].chunks(2) {
            assert_eq!(
                pair[0].decompress().unwrap() + pair[1].decompress().unwrap(),
                Scalar::from(MAX) * pc.B
            );
        }
        let mut samples = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            proof
                .verify_multiple(
                    &bp,
                    &pc,
                    &mut Transcript::new(b"CNU/comparator/v0"),
                    &commits,
                    64,
                )
                .unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.);
        }
        let mut altered = commits.clone();
        altered[0] = (altered[0].decompress().unwrap() + pc.B).compress();
        assert!(proof
            .verify_multiple(
                &bp,
                &pc,
                &mut Transcript::new(b"CNU/comparator/v0"),
                &altered,
                64
            )
            .is_err());
        assert!(proof
            .verify_multiple(
                &bp,
                &pc,
                &mut Transcript::new(b"wrong-domain"),
                &commits,
                64
            )
            .is_err());
        let mut corrupted = proof.to_bytes();
        corrupted[30] ^= 1;
        assert!(RangeProof::from_bytes(&corrupted)
            .and_then(|p| p.verify_multiple(
                &bp,
                &pc,
                &mut Transcript::new(b"CNU/comparator/v0"),
                &commits,
                64
            ))
            .is_err());
        rows.push(serde_json::json!({"outputs":outputs,"values_with_padding":k,"proof_bytes":proof.to_bytes().len(),"prove_ms":prove_ms,"verify_ms":samples,"altered_commitment_rejected":true,"wrong_domain_rejected":true,"corrupted_proof_rejected":true}));
    }
    // Both values fit u64, but their sum is MAX+2^64: range checking alone
    // accepts this oversized amount; the existing complement relation rejects it.
    let values = [MAX + 1, u64::MAX];
    let blinds = [Scalar::ONE, -Scalar::ONE];
    let (p, c) = RangeProof::prove_multiple(
        &bp,
        &pc,
        &mut Transcript::new(b"CNU/bound-test"),
        &values,
        &blinds,
        64,
    )
    .unwrap();
    p.verify_multiple(&bp, &pc, &mut Transcript::new(b"CNU/bound-test"), &c, 64)
        .unwrap();
    assert_ne!(
        c[0].decompress().unwrap() + c[1].decompress().unwrap(),
        Scalar::from(MAX) * pc.B
    );
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"scope":"Isolated Ristretto Bulletproofs 5.0 comparator; NOT secp256k1 or consensus integration","generator_precomputation_excluded":true,"verification_result_cache":false,"samples_per_case":5,"oversized_value_requires_complement_check":true,"cases":rows})).unwrap());
}
