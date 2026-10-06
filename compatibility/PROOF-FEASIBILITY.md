# Smaller-proof feasibility — October 6, 2026

Decision: retain the measured consensus verifier; investigate aggregated Bulletproofs-family proofs in a separate experimental adapter. Do not raise the block work limit or substitute cryptography based on these estimates. No replacement has been benchmarked or integrated into consensus.

| Candidate | Evidence | Fit and unresolved work |
|---|---|---|
| Current secp256k1 range proofs | Locally measured; two 4,166-byte proofs per confidential output plus complement | Compatible baseline; large bandwidth and cold verification cost |
| Classic aggregated Bulletproofs | Transparent setup and logarithmic proof sizes; published implementations | Useful comparison target. Dalek uses Ristretto, so it cannot directly verify our secp256k1 commitments |
| Bulletproofs+ | Paper reports 576-byte single 64-bit proof under its encoding | Size improvement candidate; select and audit a compatible implementation before any integration |
| Bulletproofs++ | Paper reports 416-byte single 64-bit proof and secp256k1 benchmarks | Strong secp256k1-oriented research candidate; paper points to proof-of-concept/WIP implementations. Those claims are not our measurements or audit evidence |

Sources: [original Bulletproofs research](https://crypto.stanford.edu/bulletproofs/), [Dalek API and encoding](https://doc.dalek.rs/bulletproofs/struct.RangeProof.html), [Bulletproofs+ paper](https://eprint.iacr.org/2020/735), [Bulletproofs++ paper](https://eprint.iacr.org/2022/510). The [current secp256k1-zkp repository](https://github.com/BlockstreamResearch/secp256k1-zkp) describes experimental APIs; its general README does not establish a reviewed drop-in Bulletproofs++ module for our pinned Rust dependency.

## Preserve the statement, not just nonnegativity

A generic 64-bit range proof is insufficient for this protocol: MAX_MONEY is smaller than 2^64. The proposed adapter must prove BOTH v and MAX_MONEY-v are in range and retain the existing complement commitment tally. Aggregate 2m values for m outputs, padding to a specified power of two when required. With this small input/output bound, integer sums remain far below the secp256k1 group order; this reasoning still depends on commitment binding and soundness. Do not remove complements without a reviewed exact-bound replacement.

The full ordered commitment list, network/protocol version, transaction statement, range width, generator derivation, aggregation count and padding convention must be bound into the transcript. Reject noncanonical points/scalars, infinity where forbidden, mismatched counts, omitted complements, reordered proofs, trailing bytes and cross-transaction reuse. Wallet receiver derivation and encrypted metadata recovery should remain unchanged only if the same group, generator orientation and commitment encoding are preserved and tested.

A new proof system needs an explicit new encoding/version and activation specification. Existing CNU2 transactions must not silently change meaning. Keep the existing excess/ownership checks until separately reviewed. Aggregating output ranges does not itself supply a conservation proof, a signature, or sender anonymity.

## Reproducible size/fee scenarios

Run `python compatibility/economics.py`; economics.json now uses measured isolated secp adapter proof sizes (803/932/1060 bytes) for the integration scenario. The earlier illustrative 33-byte-point formula is retained in JSON for comparison; the historical library uses a different encoding. Actual transaction framing and a versioned consensus format are still undefined, so these are not measured candidate transactions.

| Confidential outputs | Measured vbytes | Hypothetical vbytes after aggregating both bounds | Current fee at 5 sat/vbyte | Hypothetical fee at 5 sat/vbyte |
|---|---:|---:|---:|---:|
| 2 | 4,704 | 739 | 23,520 sats | 3,695 sats |
| 8 | 17,890 | 1,459 | 89,450 sats | 7,295 sats |
| 24 | 53,050 | 3,323 | 265,250 sats | 16,615 sats |

All other current overhead is retained in these scenarios. No CPU speedup is assumed; 36 one-input/two-output payments per block remains the current work-bound result. Published benchmark ratios cannot set our consensus limits. Current RPC timings include host/transport overhead and some concurrent compilation, so they are not cryptographic microbenchmarks.

## Adapter acceptance gate

1. Pin a candidate implementation and document its license, maintenance status, audit coverage and applicable assumptions. Review source rather than assuming a paper implies implementation readiness.
2. Build an isolated adapter exposing prove/verify over our existing commitments. Do not connect it to node consensus yet.
3. Run independent vectors for zero, MAX_MONEY, MAX_MONEY+1, negative-equivalent openings, altered generators/transcripts, aggregation reorder/padding and corrupted last proofs. Compare accepted statements with the existing exact-bound verifier.
4. Measure proof bytes, prover time, cold/warm verifier time, peak allocation and malformed-proof cost for 1/2/8/24 outputs on multiple machines. Record dependency commits and compiler flags.
5. Obtain independent cryptographic composition review; only then propose versioned consensus integration and a recalibrated resource schedule.

SNARK/STARK alternatives are deferred: they would require a separately reviewed statement and implementation with additional engineering/assumption choices. Their possible compactness does not justify importing an unrelated proof system into this prototype now.


## Executed comparator (separate from consensus)

An isolated Ristretto implementation is now under experiments/bulletproofs with pinned dependencies, source and results. It is not linked into the Bitcoin verifier. Generator precomputation is excluded and verification has no result cache. The 24-output case pads 48 actual values/complements to 64.

| Outputs | Measured aggregate proof bytes | Median verification ms (5 samples) |
|---|---:|---:|
| 1 | 736 | 1.444 |
| 2 | 800 | 2.400 |
| 8 | 928 | 7.420 |
| 24 | 1056 | 26.454 |

These are proof-only Ristretto measurements, not the measured secp256k1 proofs now used in the fee scenario and not directly comparable to native-node RPC timings. Altered commitments, a wrong transcript domain and corrupted proofs were rejected. A valid 64-bit proof for MAX_MONEY+1 and a wrapped complement was accepted by the generic range verifier but rejected by the complement tally. This demonstrates why exact-bound enforcement cannot be dropped.

The new secp256k1 experiment below advances implementation evidence; full transaction binding and independent vectors remain open. No consensus proof-format change or work-budget increase is justified by this comparator alone.

## Executed secp256k1 adapter (2026-10-06)

[Source and reproduction](../experiments/secp256k1-bulletproofs/README.md), [measurements](../experiments/secp256k1-bulletproofs/results.json), and [scratch cleanup finding](../experiments/secp256k1-bulletproofs/SCRATCH-CLEANUP.md). This historical candidate uses the exact baseline H/G orientation and byte-identical Pedersen commitments. Pinned upstream: b247e1ec8ed62b9abf123dc83189d253e17d488d, plus two explicit local scratch cleanup insertions. Initial malformed-proof tests crashed the unpatched build; this is not a ready-to-deploy library.

| Outputs | Proof bytes | Adapter median verify ms |
|---|---:|---:|
| 1 | 739 | 8.751 |
| 2 | 803 | 14.633 |
| 8 | 932 | 46.247 |
| 24 | 1060 | 156.972 |

Local host: AMD Ryzen 7 5800X, Windows, Rust 1.98.1/MSVC, release with C VERIFY enabled and portable 10x26/8x32 field/scalar backend. Five verification samples per size, no result cache, generator setup excluded. Verification includes modern-library complement tallies. These timings are not directly comparable to the original RPC measurements or to an optimized alternative C backend. Scratch limit is 64 MiB; peak memory remains unmeasured.

Boundary, transcript, commitment ordering, padding, count, malformed/trailing/truncated proof checks pass, including one bit flip at every byte of a valid proof on a reused context. Generic 64-bit verification accepts the oversized-value test while the adapter's monetary tally rejects it. The test corpus is finite and self-generated, not independent vectors or sustained fuzzing.

**Decision: continue isolated qualification; do not integrate into consensus yet.** The owner approved MIT for original code. External composition review, historical-library/patch review, memory-safety tooling, representative hostile workloads, complete transaction transcript design, versioned activation and outside node reproduction remain gates. No change to the 256-unit block budget or 36-payment result was made.
