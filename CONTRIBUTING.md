# Review and reproduction

Do not use real BTC or personal wallet seeds. Before proposing consensus changes, state the invariant affected and supply a reproducer. Security review must specify commit, assumptions, severity and proceed/revise/stop disposition. A maintainer's own test run is not an independent verdict.

Outside reproduction reports should include repository commit, OS/compiler/Rust versions, dependency provenance, commands, report success fields, binary hashes, divergences and log excerpts without credentials or wallet seeds. Record whether you used your own machine and independently obtained dependencies. Wall-clock timings and random txids need not match.

Cryptography changes require exact monetary bounds, transcript/encoding rules, vectors and cold-verification measurements. Keep experimental adapters separate from consensus until reviewed. Do not submit automated tool output as an independent human audit.
