# Activation design gate â€” October 5, 2026

The fixed-height portion is now implemented and tested on isolated regtest. This is not a production deployment or a request to activate on Bitcoin. The executable refuses non-regtest networks. -cnuactivationheight selects H, defaulting to zero for the original V2 tests; do not change it on existing chainstate.

## Compatibility argument and its limit

Bitcoin's public amount accounting remains authoritative to old nodes. The extension adds spending constraints and confidential-claim accounting; it does not authorize creation of public BTC. Honest experimental blocks pass an unmodified pinned Core binary. That is necessary evidence for a soft fork, not a universal proof of compatibility.

[BIP141](https://bips.dev/141/) reserves witness versions for additional semantics. The experiment uses version 15 with one exact backing program and version 16 with 32-byte metadata hashes. Those are experimental choices, not allocated production namespaces. No Bitcoin deployment is claimed.

## Proposed boundary rules to implement and test

1. Select an activation mechanism through public review. If a versionbits state machine is chosen, derive state from the parent chain, following the state transitions in [BIP9](https://bips.dev/9/). Signalling and technical activation do not themselves establish community agreement. No bit, date, threshold or production height is selected here.
2. Determine an activation height H for each active chain history. Before H, leave historical consensus behavior unchanged. At H and later, recognize extension *inputs* only when the prevout was created at height >= H in an extension namespace. Existing Coins height is available; same-block outputs use the candidate height. Apply extension creation checks to new outputs at >= H.
3. Grandfather older matching scripts as ordinary public coins, never as confidential claims or protocol backing. Spending an older zero-valued fake carrier cannot mint a claim. An older positive matching script cannot redeem protocol backing merely by resembling a reserve. A migration deposit, if supported, must explicitly fund a newly validated claim and backing under ordinary value accounting. Current input decoding does not implement that migration path.
4. Genesis of the confidential accounting starts with R=S=0 at H. Only post-activation validated deposits can increase both totals. A pre-activation lookalike cannot seed the induction argument.
5. Reorgs across H must undo both spentness and classification using the active branch's deployment state. Clear or revalidate mempool entries on changes of activation context. Proof-cache entries are mathematical statements only and must never cache activation or UTXO eligibility.
6. Apply the same rules to block validation, mempool acceptance, reindex, block assembly and wallet scanning. Snapshot/bootstrap validation must establish the history-dependent claim invariant; a UTXO snapshot containing metadata hashes alone is insufficient evidence for a new deployment unless its chain state is appropriately authenticated and background validation is defined.

Grandfathering avoids imposing new ownership semantics on historical lookalikes. Input classification now checks creation height as well as scripts, and new-output checks use candidate height. Historical lookalikes cannot be used as confidential inputs in mixed extension transactions; an ordinary public spend remains possible. P2SH-wrapped witness programs need an explicit decision and regression tests; the current extension recognizes only native output scripts.

## Required boundary matrix

| Case | Required result |
|---|---|
| Matching script created before H and spent after H | Historical public semantics; no automatic claim/backing recognition |
| Fake confidential metadata created before H | Cannot contribute private value after H |
| Deposit at H, spend in same block | New checks apply to both; ordinary parent ordering retained |
| Malformed creation at H-1 vs H | Historical behavior before; extension rejection at/after boundary |
| Competing branches on opposite sides of activation | Deterministic disconnect/reconnect and mempool revalidation |
| Reindex and seed scan across H | Same classification, spendable outputs and claims as initial validation |
| Wrapped programs and unrelated witness versions | Explicitly specified behavior; no accidental bypass or blanket restriction |
| Stock and upgraded validation | Every accepted upgraded block also accepted by stock consensus |

activation_demo.py now tests historical reserve/carrier public spends, rejection of historical claims against new backing, H-height deposit and same-block spend, rollback below H, inactive mempool rejection, boundary mempool eviction, reconnect, reindex and historical seed recovery. See report-activation.json. The complete matrix is not yet covered: deployment-state-machine branches, wrapped-program cases and snapshot behavior remain open. The fixed-height harness uses invalidate/reconsider RPCs to force rollback; it is not evidence for a versionbits state machine.

## Downgrade and deployment prerequisites

An old node accepts blocks without checking the confidential restrictions. It cannot safely report confidential ownership or protect backing against invalid upgraded-rule spends. Wallets must not treat an old node's acceptance as confidential finality. Under loss of enforcing mining/economic support, backing can be stolen under old rules; local testing cannot solve that deployment risk.

Before any deployment proposal: assign reviewed namespaces; establish deterministic cold-validation work limits; complete parser/cryptography/relay review; reproduce across independent implementations and platforms; test activation/reorg histories; specify recovery and downgrade behavior; and obtain the required ecosystem agreement. Public testnet submission before activation would demonstrate serialization, not Bitcoin enforcement of confidentiality or claim validity.

## Boundary mempool policy

At a next-block activation transition, the research node conservatively evicts the entire mempool, including unrelated transactions. Re-admission evaluates the new context. Before H, extension-looking transactions are nonstandard and cannot silently re-enter as confidential wallet payments; historical consensus still permits matching scripts in blocks. Selective revalidation would improve UX but is not implemented. The original V2 test separately exercises a longer competing chain.
