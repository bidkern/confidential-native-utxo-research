//! Established rangeproof API; no wallet data is available to verification.
use commitments::{commit, generator, parse, tally, Commitment, MAX_MONEY, RANGE_END};
use keys::Result;
use secp256k1_zkp::{RangeProof, Secp256k1, SecretKey, Tweak};
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Mutex, OnceLock},
};
pub const MAX_PROOF: usize = 5134;

// Store complete verification statements: cache hits do not add a hash-collision
// assumption. Arc shares bytes between the FIFO and set. Only successes enter.
#[derive(Hash, PartialEq, Eq)]
struct Statement {
    c: Commitment,
    proof: Vec<u8>,
    bind: [u8; 32],
    exact: bool,
}
struct ProofCache {
    entries: HashSet<Arc<Statement>>,
    fifo: VecDeque<Arc<Statement>>,
    capacity: usize,
}
impl ProofCache {
    fn new(capacity: usize) -> Self {
        Self {
            entries: HashSet::new(),
            fifo: VecDeque::new(),
            capacity,
        }
    }
    fn insert(&mut self, statement: Statement) {
        if self.capacity == 0 || self.entries.contains(&statement) {
            return;
        }
        while self.entries.len() >= self.capacity {
            if let Some(old) = self.fifo.pop_front() {
                self.entries.remove(&old);
            }
        }
        let statement = Arc::new(statement);
        self.entries.insert(statement.clone());
        self.fifo.push_back(statement);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub complement: Commitment,
    pub lower: Vec<u8>,
    pub upper: Vec<u8>,
}
pub fn prove_one(
    value: u64,
    blind: [u8; 32],
    c: Commitment,
    bind: &[u8; 32],
    nonce: SecretKey,
    exact: bool,
) -> Result<Vec<u8>> {
    RangeProof::new(
        &Secp256k1::new(),
        0,
        parse(&c)?,
        value,
        Tweak::from_slice(&blind).map_err(|_| "blind")?,
        &[],
        bind,
        nonce,
        if exact { -1 } else { 0 },
        if exact { 0 } else { 52 },
        generator(),
    )
    .map(|p| p.serialize())
    .map_err(|_| "range proof generation")
}
pub fn verify_one(c: Commitment, p: &[u8], bind: &[u8; 32], exact: bool) -> Result<()> {
    static CACHE: OnceLock<Mutex<ProofCache>> = OnceLock::new();
    verify_cached(
        c,
        p,
        bind,
        exact,
        CACHE.get_or_init(|| Mutex::new(ProofCache::new(1024))),
    )
}
fn verify_cached(
    c: Commitment,
    p: &[u8],
    bind: &[u8; 32],
    exact: bool,
    cache: &Mutex<ProofCache>,
) -> Result<()> {
    if p.is_empty() || p.len() > MAX_PROOF {
        return Err("proof size");
    }
    // Pin the v0 library encoding: zero exponent, 52-bit mantissa and zero minimum.
    // The 0.11 wrapper computes an exclusive upper bound with max+1. Reject a
    // malicious 64-bit header BEFORE calling it, avoiding debug overflow panics.
    if exact {
        if p.len() != 65 || p[0] != 0 {
            return Err("exact-zero proof header");
        }
    } else if p.len() != 4166 || p[..2] != [64, 51] {
        return Err("52-bit proof header");
    }
    let statement = Statement {
        c,
        proof: p.to_vec(),
        bind: *bind,
        exact,
    };
    // Poisoning disables the optimization, never consensus validation. Do not
    // hold this mutex across expensive cryptography (duplicate work is safe).
    if cache
        .lock()
        .is_ok_and(|cache| cache.entries.contains(&statement))
    {
        return Ok(());
    }
    verify_uncached(c, p, bind, exact)?;
    if let Ok(mut cache) = cache.lock() {
        cache.insert(statement);
    }
    Ok(())
}
fn verify_uncached(c: Commitment, p: &[u8], bind: &[u8; 32], exact: bool) -> Result<()> {
    let proof = RangeProof::from_slice(p).map_err(|_| "proof parse")?;
    let range = proof
        .verify(&Secp256k1::new(), parse(&c)?, bind, generator())
        .map_err(|_| "invalid range proof")?;
    if range.start != 0 || range.end != if exact { 1 } else { RANGE_END } {
        return Err("noncanonical public range");
    }
    Ok(())
}

pub fn prove(
    value: u64,
    blind: [u8; 32],
    bind: &[u8; 32],
    nonces: [SecretKey; 2],
) -> Result<Bounds> {
    let rest = MAX_MONEY
        .checked_sub(value)
        .ok_or("value above MAX_MONEY")?;
    if blind == [0; 32] {
        return Err("zero output blind");
    }
    let neg = keys::negate(blind)?;
    let c = commit(value, blind)?;
    let complement = commit(rest, neg)?;
    Ok(Bounds {
        complement,
        lower: prove_one(value, blind, c, bind, nonces[0], false)?,
        upper: prove_one(rest, neg, complement, bind, nonces[1], false)?,
    })
}
pub fn verify(c: Commitment, p: &Bounds, bind: &[u8; 32]) -> Result<()> {
    tally(&[c, p.complement], &[commit(MAX_MONEY, [0; 32])?])?;
    verify_one(c, &p.lower, bind, false)?;
    verify_one(p.complement, &p.upper, bind, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_preserves_statement_checks_and_bounds() {
        let cache = Mutex::new(ProofCache::new(2));
        let blind = [1; 32];
        let bind = [7; 32];
        let c = commit(42, blind).unwrap();
        let p = prove_one(
            42,
            blind,
            c,
            &bind,
            SecretKey::from_slice(&[2; 32]).unwrap(),
            false,
        )
        .unwrap();
        for _ in 0..2 {
            verify_cached(c, &p, &bind, false, &cache).unwrap();
        }
        let mut bad = p.clone();
        bad[30] ^= 1;
        assert!(verify_cached(c, &bad, &bind, false, &cache).is_err());
        assert!(verify_cached(commit(43, blind).unwrap(), &p, &bind, false, &cache).is_err());
        assert!(verify_cached(c, &p, &[8; 32], false, &cache).is_err());
        assert!(verify_cached(c, &p, &bind, true, &cache).is_err());
        assert_eq!(cache.lock().unwrap().entries.len(), 1);
        for value in [43, 44] {
            let c = commit(value, blind).unwrap();
            let p = prove_one(
                value,
                blind,
                c,
                &bind,
                SecretKey::from_slice(&[2; 32]).unwrap(),
                false,
            )
            .unwrap();
            verify_cached(c, &p, &bind, false, &cache).unwrap();
        }
        assert_eq!(cache.lock().unwrap().entries.len(), 2);
        assert_eq!(cache.lock().unwrap().fifo.len(), 2);
        assert!(!cache.lock().unwrap().entries.contains(&Statement {
            c,
            proof: p.clone(),
            bind,
            exact: false
        }));
        verify_cached(c, &p, &bind, false, &cache).unwrap();
        let disabled = Mutex::new(ProofCache::new(0));
        verify_cached(c, &p, &bind, false, &disabled).unwrap();
        assert!(disabled.lock().unwrap().entries.is_empty());
    }
    #[test]
    fn concurrent_and_poisoned_cache_preserves_validation() {
        let cache = Arc::new(Mutex::new(ProofCache::new(2)));
        let blind = [1; 32];
        let bind = [9; 32];
        let c = commit(0, blind).unwrap();
        let p = prove_one(
            0,
            blind,
            c,
            &bind,
            SecretKey::from_slice(&[3; 32]).unwrap(),
            true,
        )
        .unwrap();
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let cache = &cache;
                let p = &p;
                scope.spawn(move || verify_cached(c, p, &bind, true, cache).unwrap());
            }
        });
        assert_eq!(cache.lock().unwrap().entries.len(), 1);
        let poisoned = cache.clone();
        assert!(std::thread::spawn(move || {
            let _guard = poisoned.lock().unwrap();
            panic!("test cache poisoning");
        })
        .join()
        .is_err());
        verify_cached(c, &p, &bind, true, &cache).unwrap();
        let mut bad = p;
        bad[30] ^= 1;
        assert!(verify_cached(c, &bad, &bind, true, &cache).is_err());
    }
}
