# Confidential native UTXO research

**Experimental, unaudited, regtest-only. No Bitcoin activation, independent architectural approval, or production wallet is claimed.**

This prototype combines confidential-value claims, seed-derived receiver keys inspired by BIP352, visible native UTXO spentness, and a fragmented public backing pool. It preserves ordinary Bitcoin serialization/public amount accounting and adds validation restrictions. This differs from directly replacing Bitcoin output amounts.

Start with [status and limitations](compatibility/STATUS.md), [specification](compatibility/SPEC.md), [reproduction](compatibility/REPRODUCE.md), and [independent review scope](compatibility/REVIEW-PACKAGE.md).

## Reproduce

On a fresh Windows checkout with Git, Python, Rust/MSVC and Visual Studio C++/CMake/vcpkg installed:

```powershell
./compatibility/reproduce.ps1
```

This fetches pinned upstream Core, builds stock and upgraded nodes, and runs three isolated demonstrations. See REPRODUCE.md for dependency configuration. Rust-only checks: `cargo test --locked --workspace`. The complete node flow is Windows-tested; CI Rust tests do not substitute for it.

## Evidence and open decisions

The local tests cover differential block validation, reorgs, recovery, activation boundaries, proof-work budgets, backing contention, and invalid P2P traffic. Reports are self-produced evidence; public copies omit local filesystem paths. No seed databases, running-node directories or binaries are published.

[Smaller-proof feasibility](compatibility/PROOF-FEASIBILITY.md) and [reproducible fee scenarios](compatibility/economics.json) separate measured baseline costs from hypothetical aggregation gains. No alternative proof system is integrated. Current work limits remain unchanged.

## Reviewers and independent reproducers wanted

We seek separate consensus and applied-cryptography proceed/revise/stop assessments and an outside clean-build report. See the repository issues and CONTRIBUTING.md. Opening requests does not mean reviewers have accepted them. The wallet pilot remains gated on review and qualification.

Legacy direct-value research documents under docs describe an earlier hard-fork experiment; compatibility/SPEC.md defines the published compatibility prototype. They must not be confused with a production upgrade proposal.

Third-party notices are in THIRD_PARTY.md. No new project-wide license grant is made by this publication; licensing of original contributions requires the owner's explicit choice.
