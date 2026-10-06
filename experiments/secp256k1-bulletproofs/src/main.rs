//! Isolated research adapter. Not linked into consensus or wallets.
use commitments::{commit, tally, Commitment, MAX_MONEY};
use rand::RngCore;
use std::{ffi::c_void, time::Instant};
extern "C" {
    fn cnu_bp_new(h: *const u8) -> *mut c_void;
    fn cnu_bp_free(b: *mut c_void);
    fn cnu_bp_prove(
        b: *mut c_void,
        v: *const u64,
        r: *const u8,
        n: usize,
        bind: *const u8,
        nonce: *const u8,
        p: *mut u8,
        len: *mut usize,
        c: *mut u8,
    ) -> i32;
    fn cnu_bp_verify(
        b: *mut c_void,
        p: *const u8,
        len: usize,
        c: *const u8,
        n: usize,
        bind: *const u8,
    ) -> i32;
}
struct Adapter(*mut c_void);
impl Drop for Adapter {
    fn drop(&mut self) {
        unsafe { cnu_bp_free(self.0) }
    }
}
#[derive(Clone)]
struct Statement {
    outputs: usize,
    commits: Vec<Commitment>,
    proof: Vec<u8>,
}
fn one() -> [u8; 32] {
    let mut b = [0; 32];
    b[31] = 1;
    b
}
fn binding(s: &Statement, tx: [u8; 32]) -> [u8; 32] {
    let mut b = Vec::new();
    b.extend(keys::NETWORK.to_le_bytes());
    b.extend(tx);
    b.extend((s.outputs as u32).to_le_bytes());
    b.extend(64u32.to_le_bytes());
    b.extend(b"paired-complements;power-two-padding-zero-blind-one");
    for c in &s.commits {
        b.extend(c);
    }
    keys::hash("CNU/secp-bulletproof-adapter/experiment-v1", &b)
}
impl Adapter {
    fn new() -> Self {
        let p = unsafe { cnu_bp_new(commitments::generator().serialize().as_ptr()) };
        assert!(!p.is_null());
        Self(p)
    }
    fn prove_raw(
        &mut self,
        values: &[u64],
        blinds: &[[u8; 32]],
        outputs: usize,
        tx: [u8; 32],
    ) -> Statement {
        assert_eq!(values.len(), blinds.len());
        assert!((2..=64).contains(&values.len()) && values.len().is_power_of_two());
        let expected = values
            .iter()
            .zip(blinds)
            .map(|(v, r)| commit(*v, *r).unwrap())
            .collect::<Vec<_>>();
        let mut s = Statement {
            outputs,
            commits: expected,
            proof: vec![0; 4096],
        };
        let bind = binding(&s, tx);
        let mut nonce = [0; 32];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let mut actual = vec![[0u8; 33]; values.len()];
        let mut len = s.proof.len();
        let ok = unsafe {
            cnu_bp_prove(
                self.0,
                values.as_ptr(),
                blinds.as_ptr().cast(),
                values.len(),
                bind.as_ptr(),
                nonce.as_ptr(),
                s.proof.as_mut_ptr(),
                &mut len,
                actual.as_mut_ptr().cast(),
            )
        };
        assert_eq!(ok, 1, "upstream proving failed");
        assert!(len <= 4096);
        assert_eq!(actual, s.commits, "historical/modern commitment mismatch");
        s.proof.truncate(len);
        s
    }
    fn prove(&mut self, values: &[u64], tx: [u8; 32]) -> Statement {
        assert!(!values.is_empty() && values.len() <= 32);
        let mut vs = Vec::new();
        let mut rs = Vec::new();
        for v in values {
            assert!(*v <= MAX_MONEY);
            let r = loop {
                let mut r = [0; 32];
                rand::rngs::OsRng.fill_bytes(&mut r);
                if r != [0; 32] && keys::scalar(r).is_ok() {
                    break r;
                }
            };
            vs.extend([*v, MAX_MONEY - *v]);
            rs.extend([r, keys::negate(r).unwrap()]);
        }
        let n = vs.len().next_power_of_two();
        vs.resize(n, 0);
        rs.resize(n, one());
        self.prove_raw(&vs, &rs, values.len(), tx)
    }
    fn generic_verify(&mut self, s: &Statement, tx: [u8; 32]) -> bool {
        if s.commits.len() < 2
            || s.commits.len() > 64
            || !s.commits.len().is_power_of_two()
            || s.proof.len() > 4096
        {
            return false;
        }
        let bind = binding(s, tx);
        unsafe {
            cnu_bp_verify(
                self.0,
                s.proof.as_ptr(),
                s.proof.len(),
                s.commits.as_ptr().cast(),
                s.commits.len(),
                bind.as_ptr(),
            ) == 1
        }
    }
    fn verify(&mut self, s: &Statement, tx: [u8; 32]) -> bool {
        if s.outputs == 0
            || s.outputs > 32
            || s.commits.len() != (2 * s.outputs).next_power_of_two()
        {
            return false;
        }
        let cap = commit(MAX_MONEY, [0; 32]).unwrap();
        for pair in s.commits[..2 * s.outputs].as_chunks::<2>().0 {
            if tally(pair, &[cap]).is_err() {
                return false;
            }
        }
        let pad = commit(0, one()).unwrap();
        if s.commits[2 * s.outputs..].iter().any(|c| *c != pad) {
            return false;
        }
        self.generic_verify(s, tx)
    }
}
fn adversarial(a: &mut Adapter) {
    let tx = [7; 32];
    let s = a.prove(&[0, MAX_MONEY, 42], tx);
    assert!(a.verify(&s, tx));
    assert!(!a.verify(&s, [8; 32]));
    let mut m = s.clone();
    m.commits.swap(0, 2);
    assert!(!a.verify(&m, tx));
    let mut m = s.clone();
    m.commits[0][0] = 0;
    assert!(!a.verify(&m, tx));
    let mut m = s.clone();
    m.commits[6] = commit(1, one()).unwrap();
    assert!(!a.verify(&m, tx));
    let mut m = s.clone();
    m.outputs = usize::MAX;
    assert!(!a.verify(&m, tx));
    let mut m = s.clone();
    m.proof.push(0);
    assert!(!a.verify(&m, tx));
    let mut m = s.clone();
    m.proof = vec![0; 4097];
    assert!(!a.verify(&m, tx));
    for i in 0..s.proof.len() {
        let mut m = s.clone();
        m.proof[i] ^= 1;
        assert!(!a.verify(&m, tx));
    }
    for len in [0, 1, 32, 64, 128, s.proof.len() - 1] {
        let mut m = s.clone();
        m.proof.truncate(len);
        assert!(!a.verify(&m, tx));
    }
    let bad = a.prove_raw(
        &[MAX_MONEY + 1, u64::MAX],
        &[one(), keys::negate(one()).unwrap()],
        1,
        tx,
    );
    assert!(
        a.generic_verify(&bad, tx),
        "generic 64-bit proof should accept both values"
    );
    assert!(
        !a.verify(&bad, tx),
        "complement invariant must reject oversupply"
    );
}
fn main() {
    let setup = Instant::now();
    let mut a = Adapter::new();
    let setup_ms = setup.elapsed().as_secs_f64() * 1000.;
    adversarial(&mut a);
    let mut rows = Vec::new();
    for outputs in [1, 2, 8, 24] {
        let values = vec![123456789; outputs];
        let start = Instant::now();
        let s = a.prove(&values, [7; 32]);
        let prove_ms = start.elapsed().as_secs_f64() * 1000.;
        let mut samples = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            assert!(a.verify(&s, [7; 32]));
            samples.push(start.elapsed().as_secs_f64() * 1000.);
        }
        samples.sort_by(f64::total_cmp);
        rows.push(serde_json::json!({"outputs":outputs,"padded_commitments":s.commits.len(),"proof_bytes":s.proof.len(),"prove_ms":prove_ms,"verify_median_ms":samples[2],"verify_samples_ms":samples}));
    }
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"upstream_commit":"b247e1ec8ed62b9abf123dc83189d253e17d488d","experimental_only":true,"local_patch":"two scratch-frame cleanup insertions; VERIFY enabled","baseline_commitments_byte_equal":true,"exact_bound_and_adversarial_checks":"passed","setup_ms":setup_ms,"scratch_bytes":67108864,"verification":"includes modern complement tally; excludes generator setup; no proof result cache; not full transaction or node timings","measurements":rows})).unwrap());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_and_malformed() {
        adversarial(&mut Adapter::new());
    }
}
