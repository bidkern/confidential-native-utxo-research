# Compromise audit and remaining gates

October 5, 2026. Status distinguishes tested improvements from fundamental tradeoffs and outstanding work. No independent review or Bitcoin adoption is implied.

| Issue | V2 result | Remaining cost or gate |
|---|---|---|
| Every payment spends one shared reserve | Removed for normal fee-funded confidential payments; concurrent payments and a child relay/confirm | Requires transparent fee funding or another future fee mechanism |
| All deposits conflict | Removed: deposits independently create backing | Public amounts and initial provenance remain |
| All withdrawals conflict | Reduced to selected native backing outputs; disjoint and multi-fragment exits tested | Bounded automatic reselection tested; starvation, fee races and exit-heavy loads remain |
| Tiny exit sweeps many backing outputs | Reject unnecessary inputs; cap new fragments at 10 test BTC | Cap and fragment-distribution policy need workload-based selection; tiny fragments can increase UTXO cost |
| Third party changes reserve input and txid | Outer native-transaction signatures reject substitution | Authorized input replacement still changes txid, as in Bitcoin generally |
| Reorg test used only empty competing blocks | Winning branch now spends backing; unaffected prebuilt transfer and withdrawal survive | Backing-conflict rebuild and stale/rebuilt descendants now tested; deeper cascading recovery remains |
| Coinbase-specific bootstrap | Removed: ordinary deposits establish backing from zero | Fixed-height activation/history tests pass; production deployment and namespace allocation remain open |
| Only direct submitblock tests | Upgraded peer relay, concurrent mempools and child acceptance tested with standardness enabled | Stock relay does not know these rules; comprehensive package/RBF/eviction/DoS testing outstanding |
| Broad nonstandard allowance | Replaced by extension-specific policy; unrelated annex rejection tested | Explicit experimental work limits added; independent review and calibrated policy remain open |
| Witness stripping poisons transaction download | Classify missing witnesses as TX_WITNESS_STRIPPED and other extension failures conservatively as TX_WITNESS_MUTATED; legacy-txid P2P recovery tested | This does not replace comprehensive relay/cache fuzzing |
| Wallet cache dependence | Exact seed recovery from all three chains tested after deleting caches | Full rescan requires retained historical witness data and account discovery bounds |
| Supply accounting | Global confidential claims match total backing in tests and by the stated induction argument | Additional cryptographic assumptions; forged claims could steal backing if proofs break |
| Proof size and work | Wire/vbytes plus native RPC validation samples for 2, 8 and 24 confidential outputs, including invalid last proofs | Bounded successful-proof cache reduces repeat work; cold unique proofs remain expensive; no calibrated CPU fee budget |
| Recipient privacy | Seed-derived one-time keys preserved | Not literal BIP352 interoperability; graph/fee-source/network inference remains |
| Confidential native amounts | Native carrier spentness retained | Two monetary views and a public reserve are still a departure from direct value replacement |
| Deployment readiness | Stronger executable compatibility evidence | No formal proof, independent crypto audit, activation process, broad Core suite or community approval |

## Fee privacy is a real new tradeoff

A reserve-free transaction's public inputs pay its actual Bitcoin fee. The fee input and its change are observable alongside the confidential transfer. Reusing an identifiable funding source can link payments or reveal a participant. Independent fee UTXOs avoid a shared spending bottleneck but do not eliminate that information leak. Wallet coin-control and fresh funding can reduce reuse, not prove anonymity. The fee-input secret also participates in the current single-sender receiver construction; arbitrary multi-party sponsorship has not been established secure.

Paying every fee from confidential balance would require moving backing, bringing reserve selection back into that path, or introducing a different settlement/fee mechanism. V2 does not claim to have obtained both benefits for free.

## Backing fragmentation is not unlimited exit scalability

Fragments are fungible backing, not private balances tied to depositors. Every legitimate claim holder can authorize an exit using available fragments. A public shared resource can still face targeted fee competition. The minimal-input restriction prevents unnecessary sweeping but does not allocate exclusive reservation rights or prevent a higher-fee legitimate exit from winning. The 10 BTC bound is chosen only to test bounded fragmentation; adopting it without studying UTXO growth, whale exits and liquidity would be unjustified.

## Highest-value remaining work

1. Extend the initial 2/8/24-output validation measurements to hostile load: maximum proof sizes, high input counts, repeated invalid proofs, sustained peer traffic and reorg replay. Establish signature/proof work limits and fee policy before treating the relay patch as hardened. The new cache can reuse valid prefixes, but cold unique late-invalid proofs remain expensive.
2. Exercise exit contention under multiple senders and competing fee rates, automatic fragment reselection, mempool eviction, withdrawal descendants and deeper reorgs. Do not add an operator with custody authority to solve these problems.
3. Extend the tested fixed-height activation treatment to reviewed deployment-state activation, wrapped programs, downgrade handling and snapshots. Historical lookalike spending and claim rejection are now tested.
4. Obtain independent review of the reserve invariant, outer/inner signing composition, receiver derivation, FFI/parser boundary and cryptography. Self-tests cannot replace that review.
5. Evaluate a vetted smaller range-proof system against explicit security assumptions and measured node costs. Do not swap unaudited cryptography merely to reduce reported byte counts.

These are concrete unresolved gates. They cannot honestly be marked solved by the current demonstrations, and some privacy/accounting tradeoffs are intrinsic to this chosen design rather than bugs waiting for a patch.

## October 5 implementation update

The wallet harness now demonstrates two fee policies. Reserve-free payments retain public fee inputs. Confidential-fee payments instead deduct the fee from hidden balance and consume selected backing, with no transparent fee input. Two such payments using disjoint fragments relayed and confirmed together. Fees, reserve values and graph edges are still public; neither mode proves sender anonymity.

Bounded withdrawal retry checks local mempool/chain availability, reselects fragments and rebuilds/re-signs while preserving amounts and destinations. It refuses to retry when the actual claim input is spent or when no backing conflict is observed. The live scenario recovered from one backing conflict, rejected the stale descendant, and accepted its rebuilt replacement. This is experimental wallet-harness behavior, not a complete production wallet or a guarantee against contention.

Proof verification now has a 1024-entry, successful-statement FIFO cache. Full proof bytes and all verification arguments are compared, so correctness does not rely on a new cache-key collision assumption. Cached proof byte storage is at most 4,265,984 bytes, plus statement/container overhead and transient concurrent requests. Verification runs outside the mutex; poison disables caching. Cold chainstate reconstruction still verifies all proofs. A cache cannot justify looser consensus work limits.

The latest 24-output RPC sample measured 242.86 ms first validation, 11.97 ms repeat validation and 15.35 ms for a corrupted last range proof with a cached valid prefix. The transaction remained 208,696 bytes. The peer took 230.61 ms on the cold invalid case. These are single machine samples under concurrent build activity, not guarantees. See report-v2.json for executable hashes and all test results.

[REVIEW-PACKAGE.md](REVIEW-PACKAGE.md) provides a concrete audit entry point. [ACTIVATION.md](ACTIVATION.md) documents the now-tested fixed-height treatment of historical lookalike scripts and the remaining deployment tests. Neither external review nor public-network activation has been completed.


## Activation, resource and reproducibility follow-up

Fixed-height activation is implemented; historical lookalikes remain public coins and cannot redeem new backing as fake confidential claims. Forced rollback, mempool eviction/re-admission context, reindex and recovery passed. Versionbits deployment, snapshots, wrapped-program coverage and public adoption remain open. Boundary handling conservatively evicts unrelated mempool transactions too.

Explicit work limits now apply to transactions, blocks and mining-template selection. Hostile tests cover a near-full 4-million-weight block with 120 confidential outputs, a stock-valid over-budget block, unique invalid proofs and a bounded twenty-message P2P burst. These do not establish sustained saturation safety. The consensus budget is provisional.

The lab sets very low explicit relay/mining fee floors (-minrelaytxfee=0.00000001 and -blockmintxfee=0.00000001 BTC/kB). Passing standardness with those floors does not establish realistic fee economics. A roughly 53,050-vbyte transaction costs 53,050 sats at 1 sat/vbyte, or 265,250 sats at 5 sats/vbyte; the current tiny demonstration fee is not representative of those scenarios.

A curated source-only package and Windows reproduction script are available. No end-user pilot is released: independent architectural review, outside reproduction and sustained-load gates still apply (PILOT-GATES.md).
