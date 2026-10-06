# Decision log

Each row records the question, alternatives, selection/reason, security and compatibility consequences, and remaining risk.

| Question | Alternatives | Selected / reasoning | Security consequence | Compatibility consequence | Unresolved risk |
|---|---|---|---|---|---|
| Supply audit meaning | Literal plaintext audit; conditional cryptographic audit | User explicitly accepted cryptographic assumptions, 2026-09-28 | Binding and proof bugs can enable undetectable inflation | Public issuance retained; plaintext UTXO sum lost | Full audit and independent implementation |
| Spend identity | Explicit outpoints; hidden membership | Explicit outpoints reuse UTXO lifecycle | Graph exposed; no membership/nullifier state | Same outpoint concept, new coin value type | Graph inference |
| Fees | Public; hidden | Public fee, simpler subsidy accounting | Fee fingerprinting | Consensus checks explicit fee rather than plaintext difference | Resource fee policy |
| Commitment | Pedersen; generic proof circuit; plaintext | secp256k1 Pedersen is homomorphic and supported | Unknown generator relation and discrete-log binding essential | Requires new amount encoding | Generator/API review |
| Range proofs | Borromean; Bulletproof family; SNARK | Existing library baseline, two fixed 52-bit proofs | Exact Bitcoin upper bound via complement tally | Large proofs and resource costs | Optimize only after measurements |
| Excess | Balanced last blinding; Schnorr excess; exact-zero proof | Exact-zero library proof avoids custom group conversions | Proves zero value for residual blind | Additional witness commitment/proof | Compact Schnorr replacement later |
| Receiver ECDH | Input aggregate; explicit ephemeral | Normalize and aggregate single-sender input keys | Input-bound uniqueness; no malicious multiparty guarantee | BIP352-inspired, no byte compatibility | Formal composition and multi-party support |
| Account keys | Shared scan labels; independent hardened branches | Independent scan/spend pairs | No obvious address linkage through repeated scan key | New experimental path | Gap-limit completeness |
| Metadata | Rewind proof; AEAD; deterministic blind only | AES256-GCM-SIV encrypted value+blind | Misuse-resistant replacement behavior; scan key reveals openings | 56 ciphertext bytes per output | Malformed ciphertext burns payment |
| Scope | General tapscript; key-only | Key-only simulator before Core integration | Smaller signing surface | Not full Taproot execution | Script integration later |
| Fork class | Witness-only soft fork; direct new encoding | Direct hard-fork candidate | Old nodes do not enforce new accounting | No soft-fork claim | Deployment is out of scope |
| Simulator issuance | Arbitrary funding hook; transparent block reward | Explicit transparent coinbase with maturity/subsidy limits | No hidden faucet transition in validation | Simulated chain only | Not proof-of-work or production chain selection |

The initial architecture's Schnorr excess and ChaCha20-Poly1305 sketches were replaced before coding; CRYPTOGRAPHY.md describes the selected backend. No note-based architecture is inherited.
