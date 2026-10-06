use keys::{sha256, Result};
use secp256k1_zkp::{
    verify_commitments_sum_to_equal, Generator, PedersenCommitment, Secp256k1, Tag, Tweak,
};
use std::sync::OnceLock;
pub const MAX_MONEY: u64 = 2_100_000_000_000_000;
pub const RANGE_END: u64 = 1u64 << 52;
pub type Commitment = [u8; 33];

pub fn generator() -> Generator {
    static H: OnceLock<Generator> = OnceLock::new();
    *H.get_or_init(|| {
        Generator::new_unblinded(
            &Secp256k1::new(),
            Tag::from(sha256(b"CNU/value-generator/v0")),
        )
    })
}
pub fn parse(c: &Commitment) -> Result<PedersenCommitment> {
    let p = PedersenCommitment::from_slice(c).map_err(|_| "malformed commitment")?;
    if p.serialize() != *c {
        return Err("noncanonical commitment");
    }
    Ok(p)
}
// The library's constructor cannot encode identity. No arbitrary wire bytes enter this API.
pub fn commit(value: u64, blind: [u8; 32]) -> Result<Commitment> {
    if value == 0 && blind == [0; 32] {
        return Err("identity commitment");
    }
    let t = Tweak::from_slice(&blind).map_err(|_| "invalid blind")?;
    Ok(PedersenCommitment::new(&Secp256k1::new(), value, t, generator()).serialize())
}
pub fn tally(a: &[Commitment], b: &[Commitment]) -> Result<()> {
    let aa = a.iter().map(parse).collect::<Result<Vec<_>>>()?;
    let bb = b.iter().map(parse).collect::<Result<Vec<_>>>()?;
    if !verify_commitments_sum_to_equal(&Secp256k1::new(), &aa, &bb) {
        return Err("value conservation");
    }
    Ok(())
}
