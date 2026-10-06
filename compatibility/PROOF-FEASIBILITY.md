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

Run `python compatibility/economics.py`; economics.json records the measured baseline and hypotheses. The illustrative classic-Bulletproof encoding uses 33-byte points, 32-byte scalars, 4+2*log2(64*k) points and five scalars, with k the padded number of values. It is arithmetic extrapolated from the documented proof layout, not a deployed secp256k1 wire format.

| Confidential outputs | Measured vbytes | Hypothetical vbytes after aggregating both bounds | Current fee at 5 sat/vbyte | Hypothetical fee at 5 sat/vbyte |
|---|---:|---:|---:|---:|
| 2 | 4,704 | 743 | 23,520 sats | 3,715 sats |
| 8 | 17,890 | 1,464 | 89,450 sats | 7,320 sats |
| 24 | 53,050 | 3,329 | 265,250 sats | 16,645 sats |

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

These are proof-only Ristretto measurements, not the hypothetical secp256k1 encodings in the fee table and not directly comparable to native-node RPC timings. Altered commitments, a wrong transcript domain and corrupted proofs were rejected. A valid 64-bit proof for MAX_MONEY+1 and a wrapped complement was accepted by the generic range verifier but rejected by the complement tally. This demonstrates why exact-bound enforcement cannot be dropped.

The next implementation gate is a separately reviewed secp256k1-compatible adapter with full transaction binding and independent vectors. No consensus proof-format change or work-budget increase is justified by this comparator alone.
