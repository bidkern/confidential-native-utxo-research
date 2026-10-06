# Observer model

All properties are ASSUMED or EXPERIMENTAL, not established by test-suite success. No test proves that an adversary cannot derive a discrete logarithm.

| Property | Observer knows | Observer does not know | Observer may infer |
|---|---|---|---|
| Published-address unlinkability | Reusable public scan/spend keys; full public chain | Scan secret, input private keys, ECDH point | Payment timing, sender disclosures, graph correlations; a sender knows its own payee |
| Repeated-payment unlinkability | Distinct output keys, input sets and fees | Receiver's scan secret | Consolidation links receipts; distinct keys alone do not prove unlinkability |
| Input-value confidentiality | Prior commitment, funding ancestry, proofs | Opening and hidden amount unless disclosed | Known transparent ancestor totals and single-output flows may reveal exact amounts |
| Output-value confidentiality | Commitment, complement, constant range bounds, excess and ciphertext | Per-output blind and decryption key | Sum constraints, fee differences, public withdrawals, merchant prices |
| Independent accounts | Both reusable addresses and network/version | Seed, hardened ancestor secret | External identity metadata; there is no shared scan public key or seed fingerprint in the payload |
| Seed recovery | The owner's seed and full canonical chain | Lost outgoing memo or unpublished recipient identity | Current owned UTXOs within scanned account range, and spend status from visible inputs |
| Incoming viewing | Scan secret and spend public key | Spend private key | All matched incoming values/openings and visible later spend edges; incoming view is intentionally not unlinkable |

Excluded adversaries from address-only claims: payer holding relevant input secrets, compromised scanner, recipient disclosure, endpoint malware, network surveillance combined with identifying data. Sender and receiver may reveal a payment. No private view key can make already disclosed information disappear.

Transparent->confidential hides neither public funding nor aggregate output value. Confidential->transparent publishes withdrawn value. Even confidential->confidential is not unconditional amount secrecy if previous disclosures and visible graph equations determine its values. Confidential output format fingerprints participation. Change uses the same encoding, but coin-selection and spend patterns can identify it. Public fees can fingerprint software and activity.

Recovery uses sequential accounts and a configurable gap limit/range. Seed-only recovery for arbitrarily sparse account indexes has no finite completeness guarantee. Blockchain here means full authenticated history, not only today's UTXO snapshot. Optional indexes are hints; omission can slow/delay receipt discovery but cannot create valid coins. A validating local replay removes their authority.

Tests 3 and 10 from the request are operational checks only: wrong scan keys miss outputs, payloads expose no common public account identifier, and public APIs do not accept an address as a scan secret. Computational indistinguishability requires a protocol security argument beyond these tests.
