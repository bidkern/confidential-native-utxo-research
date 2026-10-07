# Review request: confidential claims with native Bitcoin spentness

We are seeking an initial feasibility review from Bitcoin consensus engineers and applied cryptographers. A useful result would be a written **proceed, revise, or stop** assessment of a specific part of the design, with assumptions and counterexamples. This is a regtest-only, unaudited research prototype developed with substantial AI assistance. Self-generated tests and hosted CI are not independent review. No reviewer endorsement or Bitcoin activation is claimed.

Reviewed implementation target: [2feff014bcc072c2afa4372508089c0bd4b96b15](https://github.com/bidkern/confidential-native-utxo-research/tree/2feff014bcc072c2afa4372508089c0bd4b96b15). Original code is MIT licensed.

## What the prototype actually does

Zero-valued native UTXOs commit to confidential claims; separate fungible public reserve UTXOs provide their BTC backing. Upgraded validation enforces equality between total backing and claims, plus authorization and exact value bounds. Reserve-free transfers can pay public fees without touching backing. Withdrawals and fees paid from confidential balances consume selected backing fragments, so competing selections still conflict.

The outpoint graph, public deposits/withdrawals, backing values and extension scripts remain visible. Receiver derivation is inspired by BIP352, not interoperable with it. Confidential carriers are experimental future witness programs, not ordinary Taproot outputs. Receiver privacy is not a promise of unlinkability from graph analysis. Supply enforcement adds commitment-binding and proof-soundness assumptions; it does not retain today's plaintext supply audit.

The intended upgrade argument is that new validation restricts blocks accepted by old nodes while preserving serialization and public BTC accounting. That argument needs independent review, including historical outputs and wrapped witness forms; passing a differential regtest is insufficient.

## Three useful review tasks

1. **Consensus:** find a counterexample to the backing/claim invariant, old-node validity subset, historical-output handling, signature coverage or reorg behavior. Read [the specification](../compatibility/SPEC.md), including its October 5 follow-up, [activation scope](../compatibility/ACTIVATION.md), and [review package](../compatibility/REVIEW-PACKAGE.md). Record whether a concern defeats the architecture or can be repaired.
2. **Cryptography:** examine the [isolated secp256k1 adapter](../experiments/secp256k1-bulletproofs/README.md), exact MAX_MONEY complement construction, generator/encoding compatibility and transcript composition. The historical library required [two scratch cleanup insertions](../experiments/secp256k1-bulletproofs/SCRATCH-CLEANUP.md) after malformed-proof tests crashed. Full transaction binding, sanitizer qualification and independent vectors remain open. The adapter is not connected to node consensus.
3. **Reproduction:** use your own machine and independently fetched dependencies to run [the complete Core demonstration](../compatibility/REPRODUCE.md). Report commit, environment, commands, binary hashes and discrepancies without seeds or credentials. Hosted Rust CI has passed; outside full-node reproduction has not been completed.

Measured two-output aggregate proof data is 803 bytes versus 16,664 baseline range-proof bytes. This is an isolated experiment, not a demonstrated node-throughput gain. The provisional 36-payment block work cap remains unchanged. See [measurements and limitations](../compatibility/PROOF-FEASIBILITY.md).

Reply in the [consensus request](https://github.com/bidkern/confidential-native-utxo-research/issues/1), [cryptography request](https://github.com/bidkern/confidential-native-utxo-research/issues/2), or [reproduction request](https://github.com/bidkern/confidential-native-utxo-research/issues/3). A scoped critique or referral is welcome; no full audit is expected from an initial reply. Paid scope and fees would require separate agreement.
