//! In-process consensus boundary. No RPC, filesystem, seed or wallet access.
use confidential_tx::{decode_outputs, validate, Transaction, MAX_TX_BYTES};

pub fn verify(
    payload: &[u8],
    expected_base: &[u8],
    previous: &[u8],
    height: u32,
) -> Result<(), &'static str> {
    let tx = Transaction::decode(payload)?;
    if tx.base_bytes() != expected_base {
        return Err("Core/payload mismatch");
    }
    let prev = decode_outputs(previous)?;
    validate(&tx, &prev, height)
}

/// Returns 1 for valid, 0 for invalid, -1 for an internal panic. Core fails closed.
/// # Safety
/// Each non-null pointer must reference its supplied readable length for this call.
#[no_mangle]
pub unsafe extern "C" fn cnu_verify_v0(
    payload: *const u8,
    payload_len: usize,
    expected: *const u8,
    expected_len: usize,
    previous: *const u8,
    previous_len: usize,
    height: u32,
) -> i32 {
    if payload.is_null()
        || expected.is_null()
        || previous.is_null()
        || payload_len > MAX_TX_BYTES
        || expected_len > MAX_TX_BYTES
        || previous_len > MAX_TX_BYTES
    {
        return 0;
    }
    std::panic::catch_unwind(|| {
        // SAFETY: the caller contract supplies live arrays; lengths were bounded above.
        let p = unsafe { std::slice::from_raw_parts(payload, payload_len) };
        let e = unsafe { std::slice::from_raw_parts(expected, expected_len) };
        let v = unsafe { std::slice::from_raw_parts(previous, previous_len) };
        i32::from(verify(p, e, v, height).is_ok())
    })
    .unwrap_or(-1)
}

pub const CORE_VERSION: u32 = 0x43554e30;
pub fn compact(b: &mut Vec<u8>, n: usize) {
    if n < 253 {
        b.push(n as u8);
    } else if n <= 65535 {
        b.push(253);
        b.extend((n as u16).to_le_bytes());
    } else {
        b.push(254);
        b.extend((n as u32).to_le_bytes());
    }
}
/// New native transaction encoding understood only by the isolated patched Core build.
pub fn core_bytes(tx: &Transaction) -> Vec<u8> {
    use confidential_tx::Value;
    let mut b = CORE_VERSION.to_le_bytes().to_vec();
    compact(&mut b, tx.inputs.len());
    for i in &tx.inputs {
        b.extend(i.bytes());
        b.push(0);
        b.extend(u32::MAX.to_le_bytes());
    }
    compact(&mut b, tx.outputs.len());
    for o in &tx.outputs {
        match &o.value {
            Value::Transparent(v) => {
                b.extend((*v as i64).to_le_bytes());
                b.extend([34, 0x51, 32]);
                b.extend(o.owner);
            }
            Value::Confidential {
                commitment,
                encrypted,
            } => {
                b.extend((-2i64).to_le_bytes());
                b.extend([34, 0x52, 32]);
                b.extend(o.owner);
                b.extend(commitment);
                b.extend(encrypted);
            }
        }
    }
    b.extend(tx.lock_height.to_le_bytes());
    b.extend(tx.fee.to_le_bytes());
    let payload = tx.bytes();
    compact(&mut b, payload.len());
    b.extend(payload);
    b
}
pub fn core_txid(tx: &Transaction) -> [u8; 32] {
    keys::sha256(&keys::sha256(&core_bytes(tx)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn deterministic_malformed_ffi_corpus() {
        // Independent byte generation, not wallet-produced fixtures. This is a
        // bounded fuzz smoke test, not coverage-guided fuzzing or an audit.
        let mut state = 0x243f6a8885a308d3u64;
        for size in 0..4096 {
            let mut payload = vec![0u8; size];
            for byte in &mut payload {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *byte = state as u8;
            }
            if size >= 8 && size % 2 == 0 {
                payload[..4].copy_from_slice(b"CNU0");
                payload[4..8].copy_from_slice(&keys::NETWORK.to_le_bytes());
            }
            let previous = [0u8; 4];
            let result = unsafe {
                super::cnu_verify_v0(
                    payload.as_ptr(),
                    payload.len(),
                    payload.as_ptr(),
                    payload.len(),
                    previous.as_ptr(),
                    previous.len(),
                    105,
                )
            };
            assert_eq!(result, 0, "malformed corpus size {size}");
        }
    }
    #[test]
    fn ffi_fails_closed_on_bad_bytes() {
        assert_eq!(
            unsafe {
                super::cnu_verify_v0(
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    1,
                )
            },
            0
        );
        assert!(super::verify(&[], &[], &[], 1).is_err());
    }
    #[test]
    fn native_statement_and_previous_coin_binding() {
        use confidential_tx::{build, OutPoint, Output, Payment, Spend, Value};
        let a = keys::Account::derive(&[7; 32], 0).unwrap();
        let output = Output {
            owner: keys::public(&a.spend).x_only_public_key().0.serialize(),
            value: Value::Transparent(100000),
        };
        let spend = Spend {
            outpoint: OutPoint {
                txid: [4; 32],
                vout: 1,
            },
            output: output.clone(),
            value: 100000,
            blind: [0; 32],
            secret: a.spend,
        };
        let tx = build(
            &[spend],
            &[Payment::Confidential(
                address::Address::from_account(&a),
                99000,
            )],
            1000,
            &mut rand::rngs::OsRng,
        )
        .unwrap();
        let mut prev = 1u32.to_le_bytes().to_vec();
        prev.extend(output.bytes());
        assert!(super::verify(&tx.bytes(), &tx.base_bytes(), &prev, 1).is_ok());
        let mut wrong = tx.base_bytes();
        wrong[10] ^= 1;
        assert!(super::verify(&tx.bytes(), &wrong, &prev, 1).is_err());
        prev[5] ^= 1;
        assert!(super::verify(&tx.bytes(), &tx.base_bytes(), &prev, 1).is_err());
    }
}
