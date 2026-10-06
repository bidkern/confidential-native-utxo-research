# V2: fragmented backing and reserve-free payments

Research implementation, October 4, 2026. Supersedes [V1](v1/SPEC.md). This does not define public-network activation or reserve a Bitcoin witness version.

## Accounting and concurrency

R is the sum of all live public reserve outputs, not one shared outpoint. S is the sum of confidential claims. D is ordinary transparent funding, W ordinary transparent outputs, and F the actual native fee. Each extension transition enforces both:

    R_next - R = D - W - F
    S_next - S = D - W - F

The first equality follows from unchanged public Bitcoin accounting; the second from the in-process confidential verifier. Initially both totals are zero. Inductively R=S under the cryptographic assumptions. Reserve backing and claims must not be counted twice as currency.

For reserve-free payments, D-W=F: ordinary P2TR fee inputs pay the fee, while the total confidential balance is unchanged. Native carrier inputs alone determine confidential conflicts. There is no reserve input, reserve output, miner settlement transaction, note tree, nullifier set or parallel UTXO database in this path.

Deposits independently create reserve outputs plus matching claims. Withdrawals burn claims and consume selected backing outputs. Backing is fungible across all claims; an output is not assigned to one depositor. A public-value fragment is not a separate confidential sub-pool, and does not reveal an individual claim's value by a one-to-one mapping.

## Backing selection and anti-churn restrictions

Each reserve output must have 0 < value <= 10 BTC. The 10 BTC cap is an uncalibrated research parameter. Deposits can split backing into multiple outputs. Coinbases may create neither reserves nor carriers: ordinary deposits initialize the system, with ordinary input maturity unchanged.

If a transition consumes reserve inputs, their total must strictly decrease. Moreover, no selected input may be removable while still covering that decrease:

    reserve_input_sum - smallest_selected_input < reserve_input_sum - reserve_output_sum

This prevents a small exit from unnecessarily sweeping/consolidating unrelated backing. It is a local minimality rule, not a globally optimal selection rule or a guarantee against fee-based contention. It disallows reserve-only maintenance; splitting occurs through deposits or withdrawal change. Multi-input withdrawals remain possible when needed.

The wallet helper prefers the smallest sufficient individual fragment; otherwise it accumulates descending values. It accepts excluded outpoints for conflicts. Live synchronization/retry UX is not yet a production wallet service.

## Native encoding and witness data

Serialization, public nValue, txid/wtxid, block weight and SegWit commitments remain upstream Bitcoin formats. The research reserve script is `OP_15 PUSH32 [0x44 repeated 32 times]`. Carriers are zero-valued `OP_16 PUSH32 SHA256(metadata)` outputs. Metadata is tag 1, owner key 32, Pedersen commitment 33, recovery ciphertext 56. Transparent metadata remains tag 0, uint64 amount, P2TR owner key 32.

Reserved programs are test-only and conspicuous. They are not standard Taproot receiving outputs. Compatibility with literal BIP352 encoding is not claimed.

Reserve inputs form an optional contiguous prefix of vin; reserve outputs form an optional contiguous prefix of vout. The inner cryptographic transaction contains every other input and output, in native order, and the exact public fee. Version is 2, locktime 0, sequences final, scriptSigs empty. Reserve input witnesses are empty. Other inputs are carriers or ordinary P2TR key-path inputs; other outputs are carriers or P2TR.

The first non-reserve input carries one envelope:

    0x50 || "CNU2" || uint32(payload_length) || payload
         || uint32(output_count) || canonical_output_metadata_records

For a P2TR input this is the annex: witness `[BIP341 signature, envelope]`. The normal interpreter checks the signature including its annex. For a carrier it is witness `[previous_metadata, outer_signature, envelope]`. Subsequent carriers use `[previous_metadata, outer_signature]`; subsequent P2TR inputs use one ordinary signature. Creation metadata must match each actual output, prior metadata must hash to the actual Coin, and all value/ownership proofs must validate before acceptance.

Payload is capped at 250,000 bytes; metadata at 500,000; total vin/vout each at 4,096. Standard transaction weight limits apply to relay; normal block weight applies to consensus. These are not calibrated worst-case CPU budgets.

## Transaction-ID protection

Every carrier adds a BIP340 signature under its owner key over:

    tagged_SHA256("CNU/outer/v2", complete_native_transaction_without_witness)

This is single SHA256 with BIP340-style tag prefixing, not double SHA256. It binds reserve selection, fee inputs, outputs, version, sequences and locktime. The existing inner signatures remain mandatory. Ordinary P2TR funding inputs retain normal BIP341 signatures as well.

Third parties cannot rebase a signed payment onto another reserve or alter output partitioning while retaining valid carrier authorization. Witness proof variation does not change txid. A legitimate sender changing a spent input still creates a new txid and invalidates descendants: the design removes reserve coupling from normal payments, not Bitcoin's ordinary dependence on consumed outpoints.

## Relay policy

Two upgraded nodes are tested with standardness enabled. Their policy specifically recognizes the extension's witness programs, exempts zero-valued carriers from public-value dust classification, and permits the tagged annex only for extension transactions. Consensus validation runs before the script policy exception for these future witness programs. Unrelated annex policy remains intact and is tested.

This is custom upgraded-node policy, not stock relay compatibility. Fee-estimation, package/RBF edge cases, peer DoS resistance and large-scale propagation are not comprehensively tested. The demonstration uses a fixed low test fee and does not establish production fee viability.

Extension failures without any witness use TX_WITNESS_STRIPPED so a stripped copy does not enter the txid reject filter and prevent later witness download. Other extension failures conservatively use TX_WITNESS_MUTATED. ConnectBlock still treats either as a consensus-invalid block. A legacy-txid P2P test sends a stripped copy, announces the valid txid, waits for getdata, then verifies that the complete transaction is accepted and relayed. This is distinct from merely retrying through RPC, which would not exercise the download filter.

## Recovery, reorgs and evidence

Scanning reads committed witness envelopes and historical native coins, then records real outer txids with the reserve-prefix output offset. Seeds recover confidential amounts/openings and spend keys without an authoritative indexer. Full historical recovery still needs archival data; a pruned node alone cannot supply already-pruned witness history.

The differential run tests concurrent deposits, two independent payments plus an unconfirmed child, disjoint withdrawals, multi-fragment withdrawal and a reorg whose winning branch actually spends reserve backing. An unrelated payment prepared beforehand and a withdrawal on an unaffected fragment retain their native bytes. An exit that conflicts with the winning branch must be rebuilt just as an ordinary double-spent transaction must.

The stock node and upgraded nodes process honest blocks, restart and rebuild chainstate. The stock node accepts deliberate confidential fraud blocks that the upgraded validator rejects; afterward the harness invalidates those attack blocks on stock to restore its parent. The honest reorg instead uses normal greater-work branch selection. [report-v2.json](report-v2.json) contains concrete evidence.

The report also records one native testmempoolaccept timing sample for each of 2, 8 and 24 confidential outputs and an invalid last range proof for each. Timings include RPC and node validation on this machine, not isolated cryptographic microbenchmarks. These measurements do not establish worst-case cost or production fee policy.

## Limits and deployment

V2 is a fresh isolated-regtest experiment, not an in-place migration from V1. Reserve accounting remains additional monetary machinery. Public deposits/withdrawals/backing, fee-source links, identifiable scripts and the visible graph remain. Proof soundness/binding and composition are unaudited. Small-input performance success does not settle worst-case verification cost.

No activation mechanism or treatment of pre-activation reserved-version outputs is specified. The namespace and fee policy require independent review. Existing ordinary Bitcoin nodes would not protect these balances without adoption of the new rules; block compatibility does not establish adoption. See [COMPROMISES.md](COMPROMISES.md) for the remaining work.

The architectural baseline was compared with [LIP2's integrating-transaction approach](https://github.com/litecoin-project/lips/blob/master/lip-0002.mediawiki). V2 instead keeps individual transfers in native base-chain outpoints. Existing witness serialization/commitments are described in [BIP141](https://bips.dev/141/); those mechanisms do not themselves activate these monetary rules.


## Fixed-height activation and work budget (October 5 follow-up)

The regtest-only -cnuactivationheight parameter defaults to zero. Inputs are extension claims/backing only when their Coins creation height is >= H; output creation rules apply at candidate height >= H. Historical scripts retain old public spending semantics and cannot be interpreted as private claims in an extension transaction. See ACTIVATION.md and its test report for exact coverage and limitations.

An extension transaction costs one work unit for its excess proof, two per confidential output, two per post-activation confidential input (ownership checks), and one per other non-reserve input for its inner signature. Limits are 128 units per transaction, 256 per block, and 256 native inputs/outputs per extension transaction. These are deterministic research limits, not a CPU-time guarantee or an accepted fee schedule. Normal Bitcoin weight, monetary and signature checks continue to apply. Block assembly enforces the same aggregate work limit when selecting mempool chunks.

The unit model deliberately charges carrier signatures conservatively but does not claim range-proof and signature costs are equal. Proofs have pinned sizes; cold verification must be benchmarked on representative hardware before changing limits. Peer traffic is not bounded merely by limiting accepted blocks.
