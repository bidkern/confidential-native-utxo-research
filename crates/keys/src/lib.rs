//! Wallet keys and domain-separated derivation. Never a consensus balance oracle.
use bitcoin::bip32::{ChildNumber, Xpriv};
use hkdf::Hkdf;
use k256::{elliptic_curve::PrimeField, Scalar as KScalar};
use secp256k1_zkp::{Parity, PublicKey, Scalar, Secp256k1, SecretKey, XOnlyPublicKey};
use sha2::{Digest, Sha256};

pub type Result<T> = std::result::Result<T, &'static str>;
pub const NETWORK: u32 = 0x4350_5554;

pub fn hash(tag: &str, bytes: &[u8]) -> [u8; 32] {
    let t = Sha256::digest(tag.as_bytes());
    let mut h = Sha256::new();
    h.update(t);
    h.update(t);
    h.update(bytes);
    h.finalize().into()
}
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn scalar(bytes: [u8; 32]) -> Result<KScalar> {
    Option::<KScalar>::from(KScalar::from_repr(bytes.into())).ok_or("noncanonical scalar")
}
pub fn blind_sum(positive: &[[u8; 32]], negative: &[[u8; 32]]) -> Result<[u8; 32]> {
    let mut sum = KScalar::ZERO;
    for b in positive {
        sum += scalar(*b)?;
    }
    for b in negative {
        sum -= scalar(*b)?;
    }
    Ok(sum.to_bytes().into())
}
pub fn negate(b: [u8; 32]) -> Result<[u8; 32]> {
    Ok((-scalar(b)?).to_bytes().into())
}
pub fn public(secret: &SecretKey) -> PublicKey {
    PublicKey::from_secret_key(&Secp256k1::new(), secret)
}
pub fn normalize(secret: SecretKey) -> SecretKey {
    if public(&secret).x_only_public_key().1 == Parity::Odd {
        secret.negate()
    } else {
        secret
    }
}
pub fn lift(bytes: [u8; 32]) -> Result<PublicKey> {
    Ok(XOnlyPublicKey::from_slice(&bytes)
        .map_err(|_| "invalid ownership key")?
        .public_key(Parity::Even))
}
pub fn aggregate_public(keys: &[[u8; 32]]) -> Result<PublicKey> {
    let p = keys.iter().map(|b| lift(*b)).collect::<Result<Vec<_>>>()?;
    PublicKey::combine_keys(&p.iter().collect::<Vec<_>>())
        .map_err(|_| "empty/infinite input aggregate")
}
pub fn aggregate_secret(secrets: &[SecretKey]) -> Result<SecretKey> {
    let b = secrets
        .iter()
        .map(|s| normalize(*s).secret_bytes())
        .collect::<Vec<_>>();
    SecretKey::from_slice(&blind_sum(&b, &[])?).map_err(|_| "zero input aggregate")
}

#[derive(Clone)]
pub struct Account {
    pub scan: SecretKey,
    pub spend: SecretKey,
}
impl Account {
    pub fn derive(seed: &[u8], account: u32) -> Result<Self> {
        if !(16..=64).contains(&seed.len()) || account >= (1 << 31) {
            return Err("seed/account range");
        }
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let root = Xpriv::new_master(bitcoin::Network::Regtest, seed).map_err(|_| "master key")?;
        // Fail closed on an invalid derivation rather than aliasing a neighboring account.
        let derive = |role| -> Result<SecretKey> {
            let path = [836969, 1, account, role]
                .map(|i| ChildNumber::from_hardened_idx(i).expect("bounded"));
            let child = root
                .derive_priv(&secp, &path)
                .map_err(|_| "invalid HD child")?;
            SecretKey::from_slice(&child.private_key.secret_bytes()).map_err(|_| "HD secret")
        };
        Ok(Self {
            scan: derive(0)?,
            spend: derive(1)?,
        })
    }
}

pub fn shared(secret: &SecretKey, point: &PublicKey) -> Result<[u8; 33]> {
    point
        .mul_tweak(&Secp256k1::new(), &Scalar::from(*secret))
        .map(|p| p.serialize())
        .map_err(|_| "ECDH failure")
}
pub fn expand<const N: usize>(s: &[u8; 33], ctx: &[u8; 32], index: u32, label: &str) -> [u8; N] {
    let hk = Hkdf::<Sha256>::new(Some(ctx), s);
    let mut info = b"CNU/v0/".to_vec();
    info.extend(label.as_bytes());
    info.extend(index.to_le_bytes());
    let mut out = [0; N];
    hk.expand(&info, &mut out).expect("bounded HKDF output");
    out
}
pub fn destination(
    s: &[u8; 33],
    ctx: &[u8; 32],
    index: u32,
    spend: &PublicKey,
) -> (SecretKey, [u8; 32]) {
    let material = expand::<32>(s, ctx, index, "ownership");
    for counter in 0u32.. {
        let mut preimage = material.to_vec();
        preimage.extend(counter.to_le_bytes());
        if let Ok(t) = SecretKey::from_slice(&hash("CNU/tweak/v0", &preimage)) {
            if let Ok(p) = spend.add_exp_tweak(&Secp256k1::new(), &Scalar::from(t)) {
                return (t, p.x_only_public_key().0.serialize());
            }
        }
    }
    unreachable!("scalar rejection sampling exhausted")
}
