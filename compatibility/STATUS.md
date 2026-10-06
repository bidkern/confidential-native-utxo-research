# Project status — verified October 6, 2026

This is a real Bitcoin consensus-extension research implementation, not a deployable Bitcoin product. Modified nodes enforce additional rules; stock nodes accept the honest experimental blocks without enforcing confidential claims. All activity is isolated regtest.

| Requested step | Current evidence | Remaining gate |
|---|---|---|
| Independent architectural verdict | Source package, review scope and unsent review request prepared | No independent reviewers engaged or signed off |
| Activation-boundary validation | Fixed-height historical classification, same-block spends, rollback, mempool eviction, reindex and recovery pass | Production deployment state machine, namespace allocation, wrapped-program and snapshot coverage; activation parameter must remain fixed for a datadir |
| Hostile workloads and limits | Transaction/block work limits; mining-template enforcement; near-full blocks; twelve contended exits; 60 seconds of unique invalid P2P traffic | Representative-hardware calibration, multi-peer saturation, starvation analysis and coverage-guided fuzzing |
| Independent reproducibility | Pinned upstream fetched and both binaries rebuilt in a clean directory; all three suites pass; 43 implementation/build files match | Outside developer reproduction and cross-platform validation |
| Wallet pilot after gates | Pilot requirements and release gates documented | Deferred until the preceding review and qualification gates are satisfied |

## Verified results

* Rust workspace tests passed, including the 4096-case deterministic malformed-byte FFI corpus; strict Clippy passed.
* Main three-node flow passed twelve attack cases, confidential fee payments, contention recovery, reorgs, chainstate rebuild and exact seed recovery.
* Fixed-height activation passed historical public spends and rejection of historical fake claims against new backing, activation-height deposit/child, rollback below activation, reconnect and recovery.
* A 3,999,009-weight block containing 120 confidential outputs passed. Its 250 proof-work units fit the experimental 256-unit budget. A stock-valid 300-unit block failed upgraded validation, and block assembly selected five of six available proof-heavy deposits.
* Twelve separate claims initially choosing one backing fragment all confirmed after bounded reselection. This used one wallet and does not demonstrate multi-user fairness.
* The main load run processed 625 unique invalid transactions over 60 seconds; the clean-directory run processed 679. None entered the mempool and the peer remained responsive. Traffic was source-limited through one peer, not a network-saturation attack.

See report-v2.json, report-activation.json, report-hostile.json and clean-reproduction.json. Clean-run report copies are under artifacts. Binary identities differ between build directories; bit-identical reproducible builds are not claimed. Compilation overlapped some original timing samples; the clean-run near-full block took approximately 1.1 seconds on the upgraded node. Treat all timings as observations, not guarantees.

## Product feasibility remains open

With the current proof system and work budget, a one-input/two-output confidential payment costs seven work units, limiting a block to 36 such payments. Large proofs also impose substantial transaction fees. Confidential-fee payments avoid transparent fee inputs but reintroduce selected-backing contention. These are material product tradeoffs, not presentation issues.

The highest-value next decision is an independent proceed/revise/stop verdict on this architecture, followed by reviewed proof-system and resource-policy work if it survives that review. No public-network deployment or pilot release is implied by passing these tests.

## Accounts

No accounts are required for the local build or tests. GitHub would be useful to publish the research repository and track findings; Delving Bitcoin would be useful for a public protocol discussion. Neither account guarantees an expert review. No exchange, funded BTC wallet, cloud subscription or paid API is needed at this stage. REVIEW-REQUEST.md contains the draft and relevant links; it has not been sent.
