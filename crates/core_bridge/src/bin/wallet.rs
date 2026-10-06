//! Test wallet: one JSON request on stdin, one JSON response on stdout.
use address::Address;
use bech32::{u5, ToBase32, Variant};
use confidential_tx::{
    build, decode_outputs, OutPoint, Output, Payment, Spend, Transaction, Value,
};
use keys::{public, Account};
use receiver_scan::{scan, spendable, View};
use secp256k1_zkp::SecretKey;
use serde_json::{json, Value as J};
use std::io::{self, Read};

fn bytes(j: &J, key: &str) -> Vec<u8> {
    hex::decode(j[key].as_str().expect("hex field")).expect("valid hex")
}
fn account(j: &J) -> Account {
    Account::derive(&bytes(j, "seed"), j["account"].as_u64().unwrap_or(0) as u32).expect("account")
}
fn out(j: &J) -> Output {
    let mut b = 1u32.to_le_bytes().to_vec();
    b.extend(bytes(j, "output"));
    decode_outputs(&b).expect("output").remove(0)
}
fn op(j: &J) -> OutPoint {
    let mut id = bytes(j, "txid");
    id.reverse();
    OutPoint {
        txid: id.try_into().expect("txid length"),
        vout: j["vout"].as_u64().expect("vout") as u32,
    }
}
fn display(id: [u8; 32]) -> String {
    hex::encode(id.into_iter().rev().collect::<Vec<_>>())
}
fn coin(s: &Spend) -> J {
    json!({"txid":display(s.outpoint.txid),"vout":s.outpoint.vout,"output":hex::encode(s.output.bytes()),"value":s.value,"blind":hex::encode(s.blind),"secret":hex::encode(s.secret.secret_bytes())})
}
fn run(j: J) -> J {
    match j["command"].as_str().expect("command") {
        "identity" => {
            let a = account(&j);
            let owner = public(&a.spend).x_only_public_key().0.serialize();
            let mut data = vec![u5::try_from_u8(1).unwrap()];
            data.extend(owner.to_base32());
            json!({"address":Address::from_account(&a).encode(),"owner":hex::encode(owner),"mining_address":bech32::encode("bcrt",data,Variant::Bech32m).unwrap()})
        }
        "fund" => {
            let a = account(&j);
            let value = j["value"].as_u64().unwrap();
            coin(&Spend {
                outpoint: op(&j),
                output: Output {
                    owner: public(&a.spend).x_only_public_key().0.serialize(),
                    value: Value::Transparent(value),
                },
                value,
                blind: [0; 32],
                secret: a.spend,
            })
        }
        "build" => {
            let spends = j["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| Spend {
                    outpoint: op(s),
                    output: out(s),
                    value: s["value"].as_u64().unwrap(),
                    blind: bytes(s, "blind").try_into().unwrap(),
                    secret: SecretKey::from_slice(&bytes(s, "secret")).unwrap(),
                })
                .collect::<Vec<_>>();
            let pays = j["payments"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    let v = p["value"].as_u64().unwrap();
                    if let Some(a) = p["address"].as_str() {
                        Payment::Confidential(Address::decode(a).unwrap(), v)
                    } else {
                        Payment::Transparent(bytes(p, "owner").try_into().unwrap(), v)
                    }
                })
                .collect::<Vec<_>>();
            let tx = build(
                &spends,
                &pays,
                j["fee"].as_u64().unwrap(),
                &mut rand::rngs::OsRng,
            )
            .expect("construction");
            json!({"hex":hex::encode(core_bridge::core_bytes(&tx)),"txid":display(core_bridge::core_txid(&tx)),"payload":hex::encode(tx.bytes()),"outputs":tx.outputs.iter().map(|o|hex::encode(o.bytes())).collect::<Vec<_>>()})
        }
        "scan" => {
            let a = account(&j);
            let tx = Transaction::decode(&bytes(&j, "payload")).expect("payload");
            let prev = j["previous"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| out(&json!({"output":v})))
                .collect::<Vec<_>>();
            let native_id = if j["native_txid"].is_string() {
                let mut id = bytes(&j, "native_txid");
                id.reverse();
                id.try_into().expect("native txid length")
            } else {
                core_bridge::core_txid(&tx)
            };
            let output_offset = j["output_offset"].as_u64().unwrap_or(0) as u32;
            let coins = scan(&View::from_account(&a), &tx, &prev)
                .unwrap()
                .into_iter()
                .map(|v| {
                    let mut s = spendable(&a, &tx, v).unwrap();
                    s.outpoint.txid = native_id;
                    s.outpoint.vout += output_offset;
                    coin(&s)
                })
                .collect::<Vec<_>>();
            json!({"coins":coins})
        }
        "tamper_proof" => {
            let mut tx = Transaction::decode(&bytes(&j, "payload")).unwrap();
            tx.ranges.iter_mut().flatten().next().unwrap().lower[30] ^= 1;
            json!({"hex":hex::encode(core_bridge::core_bytes(&tx)),"txid":display(core_bridge::core_txid(&tx)),"payload":hex::encode(tx.bytes())})
        }
        _ => panic!("unknown command"),
    }
}
fn main() {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).unwrap();
    let j = serde_json::from_str(&s).expect("JSON request");
    println!("{}", run(j));
}
