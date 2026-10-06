//! Public transaction model, private construction and stateless validation.
use address::Address;
use aes_gcm_siv::{
    aead::{Aead, KeyInit, Payload},
    Aes256GcmSiv, Nonce,
};
use commitments::{commit, parse, tally, Commitment, MAX_MONEY};
use keys::{
    aggregate_public, aggregate_secret, destination, expand, hash, public, shared, Result, NETWORK,
};
use proofs::Bounds;
use rand::{CryptoRng, RngCore};
use secp256k1_zkp::{schnorr::Signature, Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey};
use std::collections::BTreeSet;

pub const MAX_IO: usize = 4096;
pub const MAX_TX_BYTES: usize = 4_000_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutPoint {
    pub txid: [u8; 32],
    pub vout: u32,
}
impl OutPoint {
    pub fn bytes(&self) -> Vec<u8> {
        let mut b = self.txid.to_vec();
        b.extend(self.vout.to_le_bytes());
        b
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Transparent(u64),
    Confidential {
        commitment: Commitment,
        encrypted: [u8; 56],
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub owner: [u8; 32],
    pub value: Value,
}
impl Output {
    pub fn bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        match &self.value {
            Value::Transparent(v) => {
                b.push(0);
                b.extend(v.to_le_bytes());
                b.extend(self.owner);
            }
            Value::Confidential {
                commitment,
                encrypted,
            } => {
                b.push(1);
                b.extend(self.owner);
                b.extend(commitment);
                b.extend(encrypted);
            }
        }
        b
    }
    pub fn commitment(&self) -> Result<Option<Commitment>> {
        match self.value {
            Value::Transparent(v) => {
                if v > MAX_MONEY {
                    return Err("transparent amount range");
                }
                if v == 0 {
                    Ok(None)
                } else {
                    Ok(Some(commit(v, [0; 32])?))
                }
            }
            Value::Confidential { commitment, .. } => {
                parse(&commitment)?;
                Ok(Some(commitment))
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Excess {
    pub commitment: Commitment,
    pub proof: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub inputs: Vec<OutPoint>,
    pub outputs: Vec<Output>,
    pub fee: u64,
    pub lock_height: u32,
    pub signatures: Vec<[u8; 64]>,
    pub ranges: Vec<Option<Bounds>>,
    pub excess: Option<Excess>,
}
impl Transaction {
    pub fn base_bytes(&self) -> Vec<u8> {
        let mut b = b"CNU0".to_vec();
        b.extend(NETWORK.to_le_bytes());
        b.extend((self.inputs.len() as u32).to_le_bytes());
        for i in &self.inputs {
            b.extend(i.bytes());
        }
        b.extend((self.outputs.len() as u32).to_le_bytes());
        for o in &self.outputs {
            b.extend(o.bytes());
        }
        b.extend(self.fee.to_le_bytes());
        b.extend(self.lock_height.to_le_bytes());
        b
    }
    pub fn txid(&self) -> [u8; 32] {
        hash("CNU/txid/v0", &self.base_bytes())
    }
    pub fn bytes(&self) -> Vec<u8> {
        let mut b = self.base_bytes();
        b.extend((self.signatures.len() as u32).to_le_bytes());
        for s in &self.signatures {
            b.extend(s);
        }
        b.extend((self.ranges.len() as u32).to_le_bytes());
        for r in &self.ranges {
            match r {
                None => b.push(0),
                Some(r) => {
                    b.push(1);
                    b.extend(r.complement);
                    put_vec(&mut b, &r.lower);
                    put_vec(&mut b, &r.upper);
                }
            }
        }
        match &self.excess {
            None => b.push(0),
            Some(e) => {
                b.push(1);
                b.extend(e.commitment);
                put_vec(&mut b, &e.proof);
            }
        }
        b
    }
    pub fn wtxid(&self) -> [u8; 32] {
        hash("CNU/wtxid/v0", &self.bytes())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_TX_BYTES {
            return Err("transaction byte limit");
        }
        let mut r = Reader { b: bytes, at: 0 };
        if r.fixed::<4>()? != *b"CNU0" || r.u32()? != NETWORK {
            return Err("domain/version");
        }
        let n = r.count(MAX_IO)?;
        let mut inputs = Vec::with_capacity(n);
        for _ in 0..n {
            inputs.push(OutPoint {
                txid: r.fixed()?,
                vout: r.u32()?,
            });
        }
        let n = r.count(MAX_IO)?;
        let mut outputs = Vec::with_capacity(n);
        for _ in 0..n {
            outputs.push(match r.byte()? {
                0 => {
                    let v = r.u64()?;
                    Output {
                        owner: r.fixed()?,
                        value: Value::Transparent(v),
                    }
                }
                1 => Output {
                    owner: r.fixed()?,
                    value: Value::Confidential {
                        commitment: r.fixed()?,
                        encrypted: r.fixed()?,
                    },
                },
                _ => return Err("output tag"),
            });
        }
        let fee = r.u64()?;
        let lock_height = r.u32()?;
        let n = r.count(MAX_IO)?;
        let mut signatures = Vec::with_capacity(n);
        for _ in 0..n {
            signatures.push(r.fixed()?);
        }
        let n = r.count(MAX_IO)?;
        let mut ranges = Vec::with_capacity(n);
        for _ in 0..n {
            ranges.push(match r.byte()? {
                0 => None,
                1 => Some(Bounds {
                    complement: r.fixed()?,
                    lower: r.vector()?,
                    upper: r.vector()?,
                }),
                _ => return Err("proof tag"),
            });
        }
        let excess = match r.byte()? {
            0 => None,
            1 => Some(Excess {
                commitment: r.fixed()?,
                proof: r.vector()?,
            }),
            _ => return Err("excess tag"),
        };
        if r.at != bytes.len() {
            return Err("trailing bytes");
        }
        Ok(Self {
            inputs,
            outputs,
            fee,
            lock_height,
            signatures,
            ranges,
            excess,
        })
    }
    pub fn sighash(&self, prev: &[Output], index: usize) -> [u8; 32] {
        let mut b = self.base_bytes();
        b.extend((prev.len() as u32).to_le_bytes());
        for o in prev {
            b.extend(o.bytes());
        }
        b.extend((index as u32).to_le_bytes());
        hash("CNU/signature/v0", &b)
    }
}
fn put_vec(b: &mut Vec<u8>, data: &[u8]) {
    b.extend((data.len() as u32).to_le_bytes());
    b.extend(data);
}

/// Bounded public coin records for the in-process Bitcoin Core verifier and wallet.
pub fn decode_outputs(bytes: &[u8]) -> Result<Vec<Output>> {
    if bytes.len() > MAX_TX_BYTES {
        return Err("coin record byte limit");
    }
    let mut r = Reader { b: bytes, at: 0 };
    let n = r.count(MAX_IO)?;
    let mut outputs = Vec::with_capacity(n);
    for _ in 0..n {
        outputs.push(match r.byte()? {
            0 => {
                let v = r.u64()?;
                Output {
                    owner: r.fixed()?,
                    value: Value::Transparent(v),
                }
            }
            1 => Output {
                owner: r.fixed()?,
                value: Value::Confidential {
                    commitment: r.fixed()?,
                    encrypted: r.fixed()?,
                },
            },
            _ => return Err("coin record tag"),
        });
    }
    if r.at != bytes.len() {
        return Err("coin record trailing data");
    }
    Ok(outputs)
}
struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self.at.checked_add(N).ok_or("length overflow")?;
        let out = self
            .b
            .get(self.at..end)
            .ok_or("truncated bytes")?
            .try_into()
            .map_err(|_| "length")?;
        self.at = end;
        Ok(out)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.fixed::<1>()?[0])
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.fixed()?))
    }
    fn count(&mut self, max: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        if n > max {
            Err("count limit")
        } else {
            Ok(n)
        }
    }
    fn vector(&mut self) -> Result<Vec<u8>> {
        let n = self.count(proofs::MAX_PROOF)?;
        let end = self.at.checked_add(n).ok_or("length overflow")?;
        let v = self.b.get(self.at..end).ok_or("truncated proof")?.to_vec();
        self.at = end;
        Ok(v)
    }
}

pub fn context(
    inputs: &[OutPoint],
    prev: &[Output],
) -> Result<([u8; 32], secp256k1_zkp::PublicKey)> {
    if inputs.is_empty() || inputs.len() != prev.len() || inputs.len() > MAX_IO {
        return Err("input context shape");
    }
    let a = aggregate_public(&prev.iter().map(|o| o.owner).collect::<Vec<_>>())?;
    let mut b = NETWORK.to_le_bytes().to_vec();
    b.extend(0u32.to_le_bytes());
    b.extend((inputs.len() as u32).to_le_bytes());
    for i in inputs {
        b.extend(i.bytes());
    }
    b.extend(a.serialize());
    Ok((hash("CNU/input/v0", &b), a))
}
pub fn associated(ctx: &[u8; 32], index: u32, owner: [u8; 32], c: Commitment) -> Vec<u8> {
    let mut b = ctx.to_vec();
    b.extend(index.to_le_bytes());
    b.push(1);
    b.extend(owner);
    b.extend(c);
    b
}
pub fn decrypt_opening(
    s: &[u8; 33],
    ctx: &[u8; 32],
    index: u32,
    o: &Output,
) -> Result<(u64, [u8; 32])> {
    let Value::Confidential {
        commitment,
        encrypted,
    } = &o.value
    else {
        return Err("not confidential");
    };
    let key = expand::<32>(s, ctx, index, "metadata-key");
    let nonce = expand::<12>(s, ctx, index, "metadata-nonce");
    let cipher = Aes256GcmSiv::new_from_slice(&key).map_err(|_| "AEAD key")?;
    let p = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: encrypted,
                aad: &associated(ctx, index, o.owner, *commitment),
            },
        )
        .map_err(|_| "metadata authentication")?;
    if p.len() != 40 {
        return Err("metadata length");
    }
    let v = u64::from_le_bytes(p[..8].try_into().map_err(|_| "value bytes")?);
    let blind = p[8..].try_into().map_err(|_| "blind bytes")?;
    if v > MAX_MONEY || commit(v, blind)? != *commitment {
        return Err("metadata opening mismatch");
    }
    Ok((v, blind))
}
#[derive(Clone)]
pub struct Spend {
    pub outpoint: OutPoint,
    pub output: Output,
    pub value: u64,
    pub blind: [u8; 32],
    pub secret: SecretKey,
}
#[derive(Clone)]
pub enum Payment {
    Confidential(Address, u64),
    Transparent([u8; 32], u64),
}
pub fn random_secret(rng: &mut (impl RngCore + CryptoRng)) -> SecretKey {
    loop {
        let mut b = [0; 32];
        rng.fill_bytes(&mut b);
        if let Ok(s) = SecretKey::from_slice(&b) {
            return s;
        }
    }
}
pub fn make_output(
    addr: &Address,
    value: u64,
    s: &[u8; 33],
    ctx: &[u8; 32],
    index: u32,
    blind: [u8; 32],
) -> Result<Output> {
    if value > MAX_MONEY || blind == [0; 32] {
        return Err("output opening range");
    }
    let (_, owner) = destination(s, ctx, index, &addr.spend);
    let c = commit(value, blind)?;
    let mut plain = value.to_le_bytes().to_vec();
    plain.extend(blind);
    let key = expand::<32>(s, ctx, index, "metadata-key");
    let nonce = expand::<12>(s, ctx, index, "metadata-nonce");
    let cipher = Aes256GcmSiv::new_from_slice(&key).map_err(|_| "AEAD key")?;
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &plain,
                aad: &associated(ctx, index, owner, c),
            },
        )
        .map_err(|_| "encrypt")?
        .try_into()
        .map_err(|_| "cipher length")?;
    Ok(Output {
        owner,
        value: Value::Confidential {
            commitment: c,
            encrypted,
        },
    })
}

pub fn build(
    spends: &[Spend],
    payments: &[Payment],
    fee: u64,
    rng: &mut (impl RngCore + CryptoRng),
) -> Result<Transaction> {
    if spends.is_empty()
        || spends.len() > MAX_IO
        || payments.is_empty()
        || payments.len() > MAX_IO
        || fee > MAX_MONEY
    {
        return Err("builder shape");
    }
    for s in spends {
        if s.value > MAX_MONEY
            || public(&s.secret).x_only_public_key().0.serialize() != s.output.owner
        {
            return Err("spend ownership/opening");
        }
        match s.output.value {
            Value::Transparent(v) if v == s.value && s.blind == [0; 32] => (),
            Value::Confidential { commitment, .. } if commit(s.value, s.blind)? == commitment => (),
            _ => return Err("input opening mismatch"),
        }
    }
    // The validator, not a builder arithmetic guard, must reject overspending.
    let prev = spends.iter().map(|s| s.output.clone()).collect::<Vec<_>>();
    let inputs = spends.iter().map(|s| s.outpoint).collect::<Vec<_>>();
    let (ctx, _) = context(&inputs, &prev)?;
    let a = aggregate_secret(&spends.iter().map(|s| s.secret).collect::<Vec<_>>())?;
    let mut outputs = Vec::new();
    let mut openings = Vec::new();
    for (i, p) in payments.iter().enumerate() {
        match p {
            Payment::Transparent(owner, v) => {
                if *v > MAX_MONEY {
                    return Err("transparent range");
                }
                keys::lift(*owner)?;
                outputs.push(Output {
                    owner: *owner,
                    value: Value::Transparent(*v),
                });
                openings.push(None);
            }
            Payment::Confidential(addr, v) => {
                let blind = random_secret(rng).secret_bytes();
                let s = shared(&a, &addr.scan)?;
                outputs.push(make_output(addr, *v, &s, &ctx, i as u32, blind)?);
                openings.push(Some((*v, blind)));
            }
        }
    }
    let mut tx = Transaction {
        inputs,
        outputs,
        fee,
        lock_height: 0,
        signatures: vec![],
        ranges: vec![],
        excess: None,
    };
    let bind = tx.txid();
    for opening in &openings {
        tx.ranges.push(match opening {
            None => None,
            Some((v, r)) => Some(proofs::prove(
                *v,
                *r,
                &bind,
                [random_secret(rng), random_secret(rng)],
            )?),
        });
    }
    let ins = spends.iter().map(|s| s.blind).collect::<Vec<_>>();
    let outs = openings
        .iter()
        .filter_map(|o| o.map(|(_, r)| r))
        .collect::<Vec<_>>();
    let x = keys::blind_sum(&ins, &outs)?;
    if x != [0; 32] {
        let c = commit(0, x)?;
        tx.excess = Some(Excess {
            commitment: c,
            proof: proofs::prove_one(0, x, c, &bind, random_secret(rng), true)?,
        });
    }
    for (i, s) in spends.iter().enumerate() {
        tx.signatures.push(
            *Secp256k1::new()
                .sign_schnorr_no_aux_rand(
                    &Message::from_digest(tx.sighash(&prev, i)),
                    &Keypair::from_secret_key(&Secp256k1::new(), &s.secret),
                )
                .as_ref(),
        );
    }
    Ok(tx)
}

pub fn validate(tx: &Transaction, prev: &[Output], height: u32) -> Result<()> {
    if tx.inputs.is_empty()
        || tx.outputs.is_empty()
        || tx.inputs.len() > MAX_IO
        || tx.outputs.len() > MAX_IO
        || tx.inputs.len() != prev.len()
        || tx.inputs.len() != tx.signatures.len()
        || tx.outputs.len() != tx.ranges.len()
    {
        return Err("transaction shape");
    }
    if tx.fee > MAX_MONEY || tx.lock_height > height {
        return Err("fee/lock range");
    }
    let mut seen = BTreeSet::new();
    for i in &tx.inputs {
        if !seen.insert(*i) {
            return Err("duplicate input");
        }
    }
    // Preflight proof sizes before serialization or expensive verification.
    for p in tx.ranges.iter().flatten() {
        if p.lower.len() > proofs::MAX_PROOF || p.upper.len() > proofs::MAX_PROOF {
            return Err("proof size");
        }
    }
    if tx
        .excess
        .as_ref()
        .is_some_and(|e| e.proof.len() > proofs::MAX_PROOF)
    {
        return Err("proof size");
    }
    if tx.bytes().len() > MAX_TX_BYTES {
        return Err("transaction byte limit");
    }
    let bind = tx.txid();
    let mut ins = Vec::new();
    let mut outs = Vec::new();
    for (i, o) in prev.iter().enumerate() {
        if let Some(c) = o.commitment()? {
            ins.push(c);
        }
        let sig = Signature::from_slice(&tx.signatures[i]).map_err(|_| "signature encoding")?;
        let pk = XOnlyPublicKey::from_slice(&o.owner).map_err(|_| "input key")?;
        Secp256k1::new()
            .verify_schnorr(&sig, &Message::from_digest(tx.sighash(prev, i)), &pk)
            .map_err(|_| "ownership signature")?;
    }
    let mut transparent_total = 0u64;
    for (o, p) in tx.outputs.iter().zip(&tx.ranges) {
        keys::lift(o.owner)?;
        if let Some(c) = o.commitment()? {
            outs.push(c);
        }
        match (&o.value, p) {
            (Value::Transparent(v), None) => {
                transparent_total = transparent_total
                    .checked_add(*v)
                    .filter(|v| *v <= MAX_MONEY)
                    .ok_or("transparent total range")?;
            }
            (Value::Confidential { commitment, .. }, Some(p)) => {
                proofs::verify(*commitment, p, &bind)?
            }
            _ => return Err("proof/output mismatch"),
        }
    }
    if tx.fee != 0 {
        outs.push(commit(tx.fee, [0; 32])?);
    }
    if let Some(e) = &tx.excess {
        proofs::verify_one(e.commitment, &e.proof, &bind, true)?;
        outs.push(e.commitment);
    }
    tally(&ins, &outs)
}
