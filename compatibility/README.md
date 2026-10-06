**Current verified status:** [STATUS.md](STATUS.md).

# Bitcoin-compatible confidential UTXOs: V2 experiment

V2 removes the global reserve dependency from normal confidential payments. It uses fragmented public backing for deposits/withdrawals, transparent fee inputs for reserve-free payments, and outer signatures binding the complete native transaction. This remains an isolated consensus research implementation, not an adopted Bitcoin upgrade.

Read [SPEC.md](SPEC.md), [the compromise audit](COMPROMISES.md), and [report-v2.json](report-v2.json). The original shared-reserve specification, source, patch and report are preserved under [v1](v1/SPEC.md); its executable is saved in bin/v1. The original direct-value hard-fork lab is separate and untouched.

## Run

From the confidential-native-utxo directory:

```powershell
python compatibility/v2_demo.py
```

The older `python compatibility/demo.py` command forwards to V2. Each run creates isolated data directories and disposable test seeds, runs one stock Core node and two upgraded nodes, and stops those three processes afterward. Test seeds/caches are stored unencrypted. The harness does not stop the original lab or publish anything to a public network.

The stock executable was built from clean upstream commit e8e7e91a1144c378dff4da2e2a562eb0f3f2e1d6. The report records executable hashes and actual transaction/block IDs. The upgraded peers run with `acceptnonstdtxn=0`; they have explicit additional relay policy for the extension. Stock Core still accepts the honest blocks but does not enforce the confidential rules or relay these transactions under its ordinary policy.

## Current evidence

- Independent deposits create backing outputs without consuming a global reserve or using a special coinbase bootstrap.
- Independent confidential payments and an unconfirmed child coexist in mempools, relay between upgraded peers and confirm together with unchanged txids.
- Withdrawals using different backing outputs coexist. Selecting an already-used output causes an ordinary replacement/conflict rejection.
- Multiple backing inputs can fund a larger withdrawal; a deterministic selection helper supports exclusions.
- A competing reserve-spending reorg does not require rebuilding an unrelated preconstructed transfer or an unaffected withdrawal.
- Stock-valid attacks are rejected by upgraded consensus, including hidden inflation, proof/metadata/signature tampering, third-party reserve substitution, backing theft, excessive backing churn and oversized backing outputs.
- An unrelated Taproot annex remains nonstandard; the extension does not blanket-disable that policy.
- A stripped-witness copy does not prevent a legacy-txid peer from requesting and delivering the valid complete transaction.
- All three nodes rebuild chainstate, verify the chain and serve matching seed-based wallet recovery.

Read the report for the exact latest cases. `null` as a submitblock result means acceptance. An upgraded peer can return `duplicate-invalid` after learning a failed block from its peer; the first upgraded node's explicit validation result is recorded separately.

## Build

```powershell
./compatibility/build.ps1 -Upgraded
```

This machine's Visual Studio/vcpkg and local Rust are used. `reserve.cpp`/`reserve.h` are maintained copies; the current checkout contains matching copies in src/cnu_reserve. Apply `patch_core.py` only to a clean pinned checkout, after saving a stock build. The build script refuses to label dirty source as stock. [reserve.patch](reserve.patch) contains the full current Core delta.

The next important work is hostile-load/fee-policy measurement and independent protocol review, not another claim of deployment readiness. Fee-input linkability, expensive proofs, exit contention, namespace/activation and the added reserve accounting remain explicit compromises.

## Fee, contention and verification follow-up

The harness now tests fees paid from confidential balances (no transparent fee inputs, but backing contention returns), bounded automatic backing reselection, and stale/rebuilt descendants. A bounded proof cache reduces repeated verification without changing proof formats or cold-validation requirements. The latest report includes first/repeat timings.

Read [the review package](REVIEW-PACKAGE.md) for independent audit scope and [the activation proposal](ACTIVATION.md) for historical-output treatment and boundary tests still required. No public-network activation or external audit is claimed.


## Activation, resource and reproducibility tests

Run `python compatibility/activation_demo.py` for fixed-height activation/history/rollback and `python compatibility/hostile_demo.py` for cold near-full blocks, work-budget enforcement, template selection, exit contention and sixty seconds of unique invalid P2P traffic. Read [resource limits](RESOURCE-LIMITS.md) for the throughput/fee tradeoff and workload limitations.

[REPRODUCE.md](REPRODUCE.md) describes the clean Windows build workflow. [REVIEW-REQUEST.md](REVIEW-REQUEST.md) is an unsent architectural review request. [PILOT-GATES.md](PILOT-GATES.md) records why an end-user pilot remains gated on independent review and outside reproduction.
