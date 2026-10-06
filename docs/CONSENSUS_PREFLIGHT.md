# Pre-implementation consensus preflight

This is the initial feasibility check required before coding, not the post-prototype Core mapping.

The direct design requires a hard fork. Current serialized outputs carry a signed plaintext amount. Current input validation adds those values and rejects outputs exceeding inputs. Encoding a confidential coin as legacy zero cannot permit a later positive transparent withdrawal under those rules. New witness restrictions cannot relax this value check. Keeping a public carrier value introduces a second public accounting constraint, not the requested replacement. Other reserve or extension architectures require a different design; this analysis does not establish their impossibility.

Candidate changes: tagged amount outputs (transaction format, consensus, persistent coin storage); range/complement/excess checks (consensus); public fee validation (consensus); commitment-aware signing messages (consensus and wallet); proof byte/work limits (consensus and policy); independent account/address/scanning derivation (wallet); encrypted recovery payload (format, wallet, signature binding); undo and snapshot representation (persistent state). Native outpoint existence/double-spend semantics remain.

A new script opcode alone cannot alter amount accounting. A new witness version could define ownership rules but would still need the broader monetary and representation changes. Public coinbase validation consumes validated fees; confidential coinbase outputs are prohibited.

Sources checked 2026-09-28: [Core transaction format](https://github.com/bitcoin/bitcoin/blob/master/src/primitives/transaction.h), [Core input validation](https://github.com/bitcoin/bitcoin/blob/master/src/consensus/tx_verify.cpp), [Taproot specification](https://bips.dev/341/). Pin a Core revision before any integration work.
