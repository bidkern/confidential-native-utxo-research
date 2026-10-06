use bech32::{FromBase32, ToBase32, Variant};
use keys::{public, Account, Result};
use secp256k1_zkp::PublicKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address {
    pub scan: PublicKey,
    pub spend: PublicKey,
}
impl Address {
    pub fn from_account(a: &Account) -> Self {
        Self {
            scan: public(&a.scan),
            spend: public(&a.spend),
        }
    }
    pub fn encode(&self) -> String {
        let mut data = vec![0];
        data.extend(self.scan.serialize());
        data.extend(self.spend.serialize());
        bech32::encode("cputest", data.to_base32(), Variant::Bech32m).expect("static hrp")
    }
    pub fn decode(s: &str) -> Result<Self> {
        if s.len() != 122 {
            return Err("address length");
        }
        let (hrp, data, variant) = bech32::decode(s).map_err(|_| "address checksum")?;
        if hrp != "cputest" || variant != Variant::Bech32m {
            return Err("address network/variant");
        }
        let bytes = Vec::<u8>::from_base32(&data).map_err(|_| "address padding")?;
        if bytes.len() != 67 || bytes[0] != 0 {
            return Err("address version/length");
        }
        Ok(Self {
            scan: PublicKey::from_slice(&bytes[1..34]).map_err(|_| "scan point")?,
            spend: PublicKey::from_slice(&bytes[34..67]).map_err(|_| "spend point")?,
        })
    }
}
