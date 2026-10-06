# Isolated secp256k1 aggregated-proof adapter

Experimental, unaudited, and NOT linked into Bitcoin Core, consensus, or wallets. This is implementation evidence, not a deployable proof upgrade. MIT applies to our adapter; upstream retains its MIT license in UPSTREAM-COPYING.

## Reproduce

Prerequisites: Git, Rust 1.98.1 and a C compiler (tested with Visual Studio Build Tools on Windows). From the repository root, in PowerShell:

```powershell
. ./experiments/secp256k1-bulletproofs/fetch.ps1
cargo run --locked --release --manifest-path experiments/secp256k1-bulletproofs/Cargo.toml
cargo test --locked --release --manifest-path experiments/secp256k1-bulletproofs/Cargo.toml
```

The script sets CNU_BP_SOURCE in the current shell. On other systems clone the source, check out the pin below, set that variable and run Cargo. Those platforms still need qualification. The build refuses the wrong pin or a dirty upstream checkout. It copies source into Cargo OUT_DIR and applies the two cleanup changes documented in SCRATCH-CLEANUP.md. VERIFY assertions are enabled.

## Statement

The baseline's serialized value generator H is imported into the historical library. Every generated commitment is compared byte-for-byte with the current baseline library. An output v is paired with MAX_MONEY-v, with opposite blindings; modern-library tally verifies their sum equals MAX_MONEY*H. A 64-bit aggregate range proof covers every member. Since the sum of two 64-bit integers is far below the secp256k1 group order, the tally cannot wrap the group order to admit inflation. Exact MAX_MONEY remains 2,100,000,000,000,000 satoshis, assuming commitment binding and proof soundness.

There are 1–32 outputs, padded to a power of two in paired values. Padding is a public zero-value commitment with blind one (not an unencodable identity); verification requires its exact bytes. The transcript binds an experiment/version tag, network, supplied 32-byte transaction binding, output count, bit width, padding convention, ordered commitments. A consensus integration must define and authenticate that transaction binding without introducing a signature/proof circularity. This harness supplies a fixed binding for tests; it does not implement that transaction format.

Fresh OS randomness supplies blinds and proof nonces. Test secrets are not wallet funds. No proof-result cache is used. Setup/generator creation is reported separately. Verification includes complement tally; timing is neither a Core RPC nor a full transaction benchmark. The configured scratch allowance is 64 MiB, not a measurement of peak process memory. Actual peak memory and worst-case work remain unqualified.

## Candidate and tests

Source: https://github.com/mimblewimble/secp256k1-zkp at b247e1ec8ed62b9abf123dc83189d253e17d488d. This historical implementation was selected for a compatibility experiment, not on a claim of maintenance or independent audit. Our local cleanup patch also requires independent review.

Tests cover zero/MAX boundaries; wrong transaction binding; reordered and malformed commitments; incorrect padding/count; oversized, truncated, trailing-byte proofs; a bit flip at each byte of a valid proof; and a valid generic 64-bit proof of MAX_MONEY+1 plus a wrapped complement that the exact monetary invariant rejects. This finite corpus is not a fuzzing campaign or cryptographic security proof. Further work: sanitizers, random parser fuzzing, deterministic cross-implementation vectors, process memory profiling, generator/transcript review, constant-time/prover review, and full transaction/node integration behind a versioned activation design.
