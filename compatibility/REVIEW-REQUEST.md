# Architectural review request â€” draft, not sent

Subject: Request for feasibility review: confidential Bitcoin claims with native spentness and fragmented public backing

We have a regtest-only consensus-extension prototype preserving Bitcoin transaction serialization and public BTC accounting. Zero-valued native carriers commit to confidential metadata; fungible public reserve outputs back hidden claims. Normal payments can avoid touching backing by using a public fee input, or pay fees from confidential balance by consuming selected backing. The outpoint graph remains visible. Receiver derivation is inspired by BIP352, not interoperable with it.

Before investing in a wallet pilot, we seek independent Bitcoin consensus and applied-cryptography review of four questions:

1. Does the proposed restriction remain a subset of old-node block consensus, including historical outputs, wrapped witness forms and reorg contexts?
2. Is the R=S reserve/claim invariant sufficient, and can any valid statement extract backing without authorized, bounded claims?
3. Are the inner/outer signatures and receiver-key derivation sound under the stated single-sender assumptions?
4. Are the proof costs, extra supply assumptions and shared-backing economics acceptable enough to justify a formal proposal?

Please provide a written proceed/revise/stop recommendation with assumptions and counterexamples. A first architectural pass is distinct from a full implementation/cryptographic audit. Scope, availability and any fee would need agreement before commissioning paid work.

The source-only package contains the specification, patch, fixed-height activation experiment, hostile-load tests, reproducibility instructions and reports. No mainnet readiness, assigned witness namespace, independent audit or community approval is claimed. Known gaps include deployment-state-machine integration, sustained hostile saturation, sophisticated exit contention, cross-platform validation and coverage-guided fuzzing.

## Where to seek review

* [Delving Bitcoin](https://delvingbitcoin.org/) has protocol-design discussions; a focused public architectural request is a possible first step. A forum account would be needed to post. Participation does not guarantee expert review.
* [Bitcoin Core's contribution guidance](https://bitcoincore.org/en/faq/contributing-code/) describes its open review/testing model. Publish this as a separate research repository first; do not present it as an accepted Core feature or open an activation PR before design review.
* Commission a separately scoped applied-cryptography review from a reviewer with verifiable work on secp256k1 commitments/range proofs and protocol composition. No reviewer has been selected or engaged, and no audit vendor is endorsed by this document.

Accounts potentially useful: GitHub to publish source/issues and Delving Bitcoin to discuss the proposal. Neither is required for local work. No exchange account, funded Bitcoin wallet, cloud account or paid API subscription is needed. The user should own the accounts and authenticate through their normal interfaces; passwords and seed phrases should not be sent in chat.
