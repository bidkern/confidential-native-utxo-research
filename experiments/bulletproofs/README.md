# Isolated aggregated-proof comparator

Run `cargo run --locked --release --manifest-path experiments/bulletproofs/Cargo.toml` from the project root. Dependencies are pinned in this experiment's own Cargo.lock and it is excluded from the consensus workspace.

This uses Bulletproofs 5.0.0 over Ristretto, with value/MAX_MONEY-value pairs and zero padding to a power of two. It tests endpoint values, altered commitments, transcript mismatch, corrupted proofs and an above-MAX_MONEY example that only the complement relation rejects.

results.json contains one local optimized-build run with five verification samples per case. Generator precomputation is excluded; no successful-verification cache is used. This is a range-proof-only measurement with different commitments from the node. It cannot establish secp256k1 integration speed, full transaction cost, security review or consensus readiness. The transcript is a comparator domain, not a complete transaction transcript.
