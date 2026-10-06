//! Atomic block application and explicit undo, with a staged map for this research model.
use confidential_tx::{validate, OutPoint, Output, Transaction, Value, MAX_TX_BYTES};
use keys::{hash, Result};
use std::collections::BTreeMap;
const MAX_MONEY: u64 = 2_100_000_000_000_000;
pub const COINBASE_MATURITY: u32 = 100;
pub const MAX_BLOCK_BYTES: usize = 4_000_000;
pub const MAX_BLOCK_TX: usize = 4096;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coin {
    pub output: Output,
    pub height: u32,
    pub coinbase: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub previous: [u8; 32],
    pub height: u32,
    pub coinbase: Vec<Output>,
    pub transactions: Vec<Transaction>,
}
impl Block {
    pub fn coinbase_id(&self) -> [u8; 32] {
        let mut b = self.previous.to_vec();
        b.extend(self.height.to_le_bytes());
        b.extend((self.coinbase.len() as u32).to_le_bytes());
        for o in &self.coinbase {
            b.extend(o.bytes());
        }
        hash("CNU/coinbase/v0", &b)
    }
    pub fn id(&self) -> [u8; 32] {
        let mut b = self.coinbase_id().to_vec();
        b.extend((self.transactions.len() as u32).to_le_bytes());
        for t in &self.transactions {
            b.extend(t.wtxid());
        }
        hash("CNU/block/v0", &b)
    }
    pub fn bytes_len(&self) -> usize {
        72 + self.coinbase.iter().map(|o| o.bytes().len()).sum::<usize>()
            + self
                .transactions
                .iter()
                .map(|t| 4 + t.bytes().len())
                .sum::<usize>()
    }
}
pub fn subsidy(height: u32) -> u64 {
    let h = height / 210_000;
    if h >= 64 {
        0
    } else {
        5_000_000_000u64 >> h
    }
}
#[derive(Clone)]
struct Undo {
    previous: [u8; 32],
    spent: Vec<(OutPoint, Coin)>,
    created: Vec<OutPoint>,
}
#[derive(Clone, Default)]
pub struct Chain {
    coins: BTreeMap<OutPoint, Coin>,
    pub height: u32,
    pub tip: [u8; 32],
    undo: Vec<Undo>,
}
impl Chain {
    pub fn coins(&self) -> &BTreeMap<OutPoint, Coin> {
        &self.coins
    }
    pub fn apply(&mut self, b: &Block) -> Result<()> {
        if b.previous != self.tip
            || b.height != self.height.checked_add(1).ok_or("height overflow")?
        {
            return Err("block parent/height");
        }
        if b.transactions.len() > MAX_BLOCK_TX || b.coinbase.len() > 4096 {
            return Err("block count limit");
        }
        // Validate object dimensions before serializing objects from an in-process caller.
        let mut block_bytes = 72 + b.coinbase.iter().map(|o| o.bytes().len()).sum::<usize>();
        for t in &b.transactions {
            if t.inputs.len() > 4096
                || t.outputs.len() > 4096
                || t.signatures.len() > 4096
                || t.ranges.len() > 4096
            {
                return Err("transaction shape");
            }
            for p in t.ranges.iter().flatten() {
                if p.lower.len() > 5134 || p.upper.len() > 5134 {
                    return Err("proof size");
                }
            }
            if t.excess.as_ref().is_some_and(|e| e.proof.len() > 5134) {
                return Err("proof size");
            }
            let size = t.bytes().len();
            if size > MAX_TX_BYTES {
                return Err("transaction size");
            }
            block_bytes = block_bytes
                .checked_add(4 + size)
                .ok_or("block size overflow")?;
            if block_bytes > MAX_BLOCK_BYTES {
                return Err("block byte limit");
            }
        }
        if b.bytes_len() > MAX_BLOCK_BYTES {
            return Err("block byte limit");
        }
        let mut coins = self.coins.clone();
        let mut undo = Undo {
            previous: self.tip,
            spent: vec![],
            created: vec![],
        };
        let mut fees = 0u64;
        for tx in &b.transactions {
            let mut prev = Vec::new();
            for op in &tx.inputs {
                let c = coins.get(op).ok_or("missing/spent input")?;
                if c.coinbase && b.height.saturating_sub(c.height) < COINBASE_MATURITY {
                    return Err("immature coinbase");
                }
                prev.push(c.output.clone());
            }
            validate(tx, &prev, b.height)?;
            fees = fees
                .checked_add(tx.fee)
                .filter(|f| *f <= MAX_MONEY)
                .ok_or("fee overflow")?;
            let txid = tx.txid();
            for (i, _) in tx.outputs.iter().enumerate() {
                if coins.contains_key(&OutPoint {
                    txid,
                    vout: i as u32,
                }) {
                    return Err("output overwrite");
                }
            }
            for op in &tx.inputs {
                let c = coins.remove(op).ok_or("duplicate input")?;
                if self.coins.contains_key(op) {
                    undo.spent.push((*op, c));
                }
            }
            for (i, o) in tx.outputs.iter().enumerate() {
                let op = OutPoint {
                    txid,
                    vout: i as u32,
                };
                coins.insert(
                    op,
                    Coin {
                        output: o.clone(),
                        height: b.height,
                        coinbase: false,
                    },
                );
                undo.created.push(op);
            }
        }
        let mut reward = 0u64;
        for o in &b.coinbase {
            keys::lift(o.owner)?;
            let Value::Transparent(v) = o.value else {
                return Err("confidential coinbase prohibited");
            };
            reward = reward
                .checked_add(v)
                .filter(|r| *r <= MAX_MONEY)
                .ok_or("coinbase value range")?;
        }
        if reward
            > subsidy(b.height)
                .checked_add(fees)
                .ok_or("reward overflow")?
        {
            return Err("coinbase overpayment");
        }
        let cb = b.coinbase_id();
        for (i, o) in b.coinbase.iter().enumerate() {
            let op = OutPoint {
                txid: cb,
                vout: i as u32,
            };
            if coins.contains_key(&op) {
                return Err("coinbase overwrite");
            }
            coins.insert(
                op,
                Coin {
                    output: o.clone(),
                    height: b.height,
                    coinbase: true,
                },
            );
            undo.created.push(op);
        }
        self.coins = coins;
        self.height = b.height;
        self.tip = b.id();
        self.undo.push(undo);
        Ok(())
    }
    pub fn disconnect(&mut self) -> Result<()> {
        let undo = self.undo.pop().ok_or("no block undo")?;
        for op in undo.created {
            self.coins.remove(&op);
        }
        for (op, c) in undo.spent {
            self.coins.insert(op, c);
        }
        self.tip = undo.previous;
        self.height -= 1;
        Ok(())
    }
}
