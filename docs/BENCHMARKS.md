# Measured baseline

Measured 2026-09-28 on AMD Ryzen 7 5800X (8 cores/16 threads), Windows x86_64, Rust 1.98.1, release optimization, pinned Cargo.lock. Single-threaded application-level benchmarks, warm caches, no validator proof cache. Raw sample counts/min/max are in ../benchmarks/baseline.csv; CPU inventory is in ../benchmarks/machine.json.

| Operation / object | Measured median or size |
|---|---:|
| Create one commitment | 65.9 microseconds |
| Generate output's two 52-bit proofs | 11.587 ms |
| Verify output's two proofs and complement | 8.519 ms |
| One 52-bit proof | 4,166 bytes |
| Output range witness, complement and length fields | 8,373 bytes |
| Exact-zero excess proof | 65 bytes |
| One-input, two-confidential-output transaction | 17,231 bytes |
| Its unsigned base body | 308 bytes |
| Validate that transaction | 17.349 ms |
| Scan it for one account | 0.237 ms |
| Serial workload: verify 100 output-bound proofs (200 range proofs) | 852.254 ms |
| Scan synthetic 100-transaction block, one account | 23.820 ms |
| Validate synthetic 100-confidential-transaction block | 1,746.648 ms |
| Same-shaped transparent-model block validation | 35.465 ms |
| Confidential synthetic block | 1,723,572 bytes |
| Transparent synthetic block | 22,572 bytes |
| Confidential coin serialized record, excluding outpoint | 127 bytes |
| Transparent-model coin serialized record, excluding outpoint | 46 bytes |
| Additional serialized coin bytes | 81 bytes |
| Actual Rust Coin enum size on this target | 136 bytes |

The block comparison is approximately 49.3x validation time in this simulator. It is **not a comparison to Bitcoin Core**. The proof backend dominates. This baseline is too costly to present as an efficient design merely because the accounting works; a reviewed compact range-proof backend is the next substantial research step.

## Method

`cargo run --release --bin benchmark` constructs a transparent issuance fixture, matures it for 100 simulated blocks, converts it to confidential form, then measures the requested split. The block workload consists of 100 dependent transactions, each with one input, a payment and change; fees are public. Scanner trials receive resolved previous outputs, so disk reads, chain download and index construction are excluded. They include key derivation, key matching and decryption of matching outputs. The synthetic scan block's outputs belong to the scanned account; misses can cost less AEAD work.

Each timed operation gets one warmup followed by the sample count shown in CSV. Construction and verification use deterministic public fixtures; proof-generation timings reuse a fixed statement and private test nonce. Production prover RNG cost and cold-cache behavior are not represented. Context creation and the prototype's map cloning are included where called. Block timing includes cloning its starting chain and staging the block. No OS affinity or exclusive benchmark machine was used; tails may reflect background activity.

The selected Rust binding has no batch range-verification API used by this project. The batch row is explicitly **serial verification of a batch workload**, not a claimed cryptographic batch speedup. Native accelerated batching remains unmeasured/unsupported in this backend.

The +81-byte coin difference includes the 56-byte ciphertext currently retained in the coin and signature-preimage model. A leaner design could keep recovery bytes only in history while retaining an appropriate binding; that optimization is not implemented. These lengths are simulator serialization, not Core chainstate's compressed disk encoding. The enum reserves space for its largest variant, so transparent coins in this same enum also occupy 136 bytes, before BTreeMap allocator/key overhead. Total RAM/DB overhead at Bitcoin scale remains unmeasured.

## Reproduction

```powershell
./scripts/cargo.ps1 build --release --bin benchmark
./target/release/benchmark.exe | Set-Content benchmarks/baseline.csv
```

Use ordinary cargo when using an independently installed toolchain. Do not compare another run while tests or another benchmark are consuming CPU. These measurements establish a baseline only; they do not establish production security, consensus viability or acceptable full-node hardware costs.
