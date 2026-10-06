# Validation and required-vector mapping

Run `cargo test --workspace` and `cargo test --release --workspace`. On the prepared Windows workspace use `scripts/cargo.ps1` in place of `cargo`. `cargo clippy --workspace --all-targets` checks the code; `cargo fmt --all --check` checks formatting.

The test runner is tests/protocol.rs, with shared public test fixtures in tests/common/mod.rs. It includes 16 integration tests; multiple assertions and attack cases occur within each test.

| Requested test | Evidence |
|---|---|
| 1: independent seed accounts | Distinct scan/spend public keys across ten hardened accounts; address roundtrip |
| 2: 100 unique payments | 100 separate transactions produce unique receiver ownership keys |
| 3: address-only observer | Wrong scan key finds no outputs; published-address API exposes no secret. This is not an impossibility proof |
| 4: recipient scanning | Correct scan key recovers each of the 100 payments |
| 5: wallet deletion/recovery | Reconstruct map from seed plus independently validated blocks; spend recovered outputs |
| 6: 10 BTC split | 300,000,000 and 699,999,000 sats plus public 1,000-sat fee validate without openings in validator |
| 7: 10->11 BTC attack | Builder creates properly signed range-proven outputs; validator rejects monetary tally |
| 8: negative/invalid range | u64::MAX equivalent commitment, wrong complement, missing/malformed proof, oversized amount, 64-bit proof header rejected |
| 9: double spend | Duplicate input, repeated transaction within block, and later spend of consumed outpoint rejected |
| 10: account linkage | No repeated scan/spend key or ancestor fingerprint in address payload; finite tests do not establish cryptographic independence |

Additional coverage: zero and MAX_MONEY endpoints; zero identity constructor rejection; wrong associated statement; altered ciphertext/signatures/fees/ownership; malformed keys; valid nonzero exact-value excess rejected; overpaid and confidential coinbase; maturity; halving schedule; withdrawal; atomic failure; parent/child block undo; bounded/truncated/trailing/malformed serialization; 1,000 deterministic random decoder inputs; explicit account-range recovery.

Frozen fixtures: account.txt records the public receiver and H; payment.txt records canonical base bytes, txid, wtxid and serialized length for the requested split. Amounts, fixture seeds and RNG are intentionally public. Reproducing a fixture detects regression, not correctness of the original derivation. Cross-implementation vectors, coverage-guided fuzzing and formal privacy/supply proofs remain future work.
