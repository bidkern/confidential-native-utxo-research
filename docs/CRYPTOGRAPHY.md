# Cryptographic selection

EXPERIMENTAL. The user accepted explicitly documented cryptographic supply assumptions on 2026-09-28. This resolves the initial requirement gate; it does not validate the construction.

## Selected spike backend

Use secp256k1-zkp 0.11.0 Pedersen commitments and its established Borromean-style range proofs, BIP340 input signatures, BIP32 hardened derivation, SHA256/HKDF-SHA256, and AES-256-GCM-SIV metadata encryption. Pin transitive dependencies in Cargo.lock. There is no trusted setup. This is a baseline for measurement, not the final proof-system selection.

H is generated using the library's generator mapping from SHA256 of the literal ASCII `CNU/value-generator/v0`. Do not compute H by multiplying G by a known hash scalar. Generator bytes must be frozen in vectors. Commitments use the library's 33-byte encoding, which is NOT ordinary SEC1 public-key encoding. Never change its prefix to manufacture a Schnorr key.

The baseline deliberately uses two 52-bit proofs per output. Publish complement K and verify `C+K=M*H`. Prove both are within [0,2^52-1], using min_value=0, exp=0, min_bits=52. Require the returned proof range to be exactly that range, preventing amount-dependent public ranges. This enforces [0,M] because 2*(2^52-1) is far below the group order. The extra 33-byte complement avoids manual conversion or arithmetic on library-internal commitment points. Two proofs are expensive; benchmark before claiming viability.

For conservation, publish an optional excess commitment E with an **exact zero-value proof** (`min=0, exp=-1`) using the same library. Verify its returned interval is [0,1), and tally `inputs = outputs + fee*H + E`. E commits to `(0, sum r_in - sum r_out)`. Omit E only when the blind difference is zero, and still verify the tally. This establishes knowledge of a zero-value opening without inventing a proof protocol or confusing commitment and SEC1 encodings. A Schnorr excess proof is a future compact alternative, not implemented in this spike. E is a transaction witness object, never a UTXO or a nullifier.

Zero transparent values/fees contribute identity by omission; never call the library commitment constructor on (0,0), which cannot serialize infinity. Output blindings are nonzero so the ordinary zero-amount and M-amount cases have nonidentity C and K. Output range statements and excess proofs bind the canonical unsigned base-body hash through the library's additional-commitment parameter. Private prover nonce material is generated independently and never published.

AES-GCM-SIV resolves the initial ChaCha20-Poly1305 replacement nonce concern. Derive a per-output key and nonce under distinct HKDF labels; its misuse-resistance is useful if an RBF replacement reuses the same input context. Repeated identical encrypted plaintext is still recognizable; no anonymity claim covers competing replacements. See [RFC8452](https://www.rfc-editor.org/rfc/rfc8452) and [RustCrypto implementation](https://docs.rs/aes-gcm-siv/0.11.1/aes_gcm_siv/).

## Alternatives

| System | Size / verification | Assumptions and setup | Compatibility / reason |
|---|---|---|---|
| Selected rangeproof backend | Kilobytes per 52-bit proof; linear work; measure locally | Discrete-log and Fiat-Shamir assumptions, no trusted setup | secp256k1 and existing bindings; conservative baseline, high bandwidth |
| Bulletproofs | Logarithmic proof length, linear verification; aggregation possible | Discrete-log/random-oracle assumptions, no trusted setup | Authors describe range proofs; must choose and review a concrete secp implementation, not substitute Ristretto commitments |
| Bulletproofs+ / ++ | Potential smaller/faster proofs; not measured here | Additional construction-specific analysis; no blanket inheritance of original proof | Candidate optimization; no invented implementation or benchmark |
| General SNARK | Often compact, backend-dependent prover/verifier cost | Setup and curve assumptions vary | Adds circuit and cross-curve complexity without requiring hidden membership |
| Transparent general proof system | Usually larger; highly backend-dependent | Hash/security parameters, no structured setup for many systems | No selected audited secp-compatible value-accounting circuit |
| Plaintext amounts | Small, arithmetic verification | No confidential-amount proof assumption | Violates requested amount hiding |

No batch speedup is claimed for the selected binding. Benchmark a batch workload as serial verification and label it accordingly. Consensus must have an exact acceptance predicate; future randomized batch verification needs sound failure handling and review. Range verifier soundness, binding, nonce generation, FFI parsing and platform agreement are all monetary security boundaries.

Primary references: [backend source](https://github.com/BlockstreamResearch/secp256k1-zkp), [Rust rangeproof API](https://docs.rs/secp256k1-zkp/0.11.0/secp256k1_zkp/struct.RangeProof.html), [Bulletproofs authors](https://crypto.stanford.edu/bulletproofs/). Production maturity of a component is not a security audit of this composition.
