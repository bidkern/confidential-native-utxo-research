# Bitcoin deployment feasibility

Research assessment, 2026-09-28. This is an architectural analysis, not a complete consensus specification or a change to the running lab.

Update, 2026-10-04: a separate [native-carrier reserve compatibility experiment](../compatibility/README.md) now exercises an unmodified Core node and an additionally restricted node on identical blocks. It supplies implementation evidence for the tested cases, including entry/exit, differential fraud rejection and recovery. Its per-transaction shared reserve introduces serious contention and transaction-rebasing limitations. The assessment below remains the original architectural analysis; it is not a claim that all listed deployment gates are resolved.

Subsequent V2 work removes the reserve input from normal transparent-fee-funded confidential transfers, allows independent backing fragments, binds outer transaction identities, and adds upgraded-node relay policy and competing-reserve reorg tests. See the current [specification](../compatibility/SPEC.md) and [compromise audit](../compatibility/COMPROMISES.md). Shared-reserve statements below describe the original candidate; V2 still retains reserve accounting and introduces an explicit fee-privacy tradeoff.

## Finding

The implemented direct replacement of Bitcoin's public amount field requires a hard fork. A different design using zero-valued native carrier outputs and a consensus-controlled public BTC reserve is a plausible soft-fork research path. It could retain explicit native outpoints and avoid notes, nullifiers, a federation, a separate chain, and an authoritative indexer. It introduces two accounting views and a reserve settlement mechanism. Its full validity and practical deployment have not been demonstrated.

The earlier statement that confidential values require a separate network was too broad. An adopted Bitcoin consensus upgrade could enforce them on Bitcoin. A separate network is currently needed to test our incompatible implementation safely, not necessarily to host the intended product permanently.

## Why the current implementation is a hard fork

The pinned upstream Core checks each public input value, compares the total to the public output total, and calculates fees from the difference. Our -2 amount tag and modified serialization are outside those rules. A new script rule can restrict authorization but cannot make a negative amount, a new unrecognized serialization, or an unbacked positive withdrawal acceptable to old nodes.

For a minimal example, a zero-valued input cannot create a 1 BTC transparent output without another input supplying that BTC. A zero-knowledge proof does not change that arithmetic. Retaining 1 BTC in every individual public amount field would expose the balances instead.

This establishes the incompatibility of direct replacement, not a universal impossibility theorem for confidential soft forks. [Pinned upstream input accounting](https://github.com/bitcoin/bitcoin/blob/e8e7e91a1144c378dff4da2e2a562eb0f3f2e1d6/src/consensus/tx_verify.cpp).

## Closest candidate: native carrier outputs with a reserve

The following is our architectural synthesis, not a claim that an existing proposal already meets all requirements.

Every confidential coin could occupy a normal base-chain outpoint with public nValue=0. Its script would commit to its ownership key, amount commitment, and recovery metadata, using an explicitly specified upgrade mechanism. Upgraded nodes would retain and validate those fields alongside their Coin records. Spending would consume that exact outpoint. Bitcoin's existing spentness machinery could therefore remain authoritative.

A separate public reserve UTXO would carry the BTC backing those confidential claims. New consensus rules, rather than an operator's signing key, would restrict its use. Block settlement would consume the previous reserve and deposits, create exact authorized transparent withdrawals, pay fees, and recreate the reserve.

Required accounting, with all quantities in integer satoshis:

    R_next = R_previous + deposits - withdrawals - reserve_paid_fees

The fee convention must exclude fees already paid directly by transparent inputs. Upgraded validation must also establish that the aggregate value of outstanding confidential claims equals the reserve, using commitments and sound proofs; individual values remain hidden. The public reserve and its claims must never be counted twice as circulating supply.

For an illustration omitting fees: Alice deposits 10 BTC, creating 10 BTC of confidential claims. She sends 3 BTC to Bob and retains 7 BTC. The reserve remains 10 BTC; legacy nodes see zero-valued carrier outputs. Bob later destroys 2 BTC of claims to withdraw 2 BTC transparently. Settlement creates that transparent output and reduces the reserve to 8 BTC. Every base-chain transaction must still satisfy old public-value rules.

This is economically reserve-backed accounting. It must not be advertised as the original single-representation direct-value design. It does not inherently require a custodian, separately traded token, or independent chain consensus. Whether it satisfies the user's meaning of 'native BTC, no bridge' depends on accepting consensus-enforced entry/exit accounting as a design compromise.

## Historical support and caution

Felix Weis's 2016 Bitcoin-dev sketch proposed zero-valued confidential outpoints plus a global public output. It provides historical support for investigating this family, not a specification to copy. In particular, its proposed same-block spend of a newly created coinbase output conflicts with existing coinbase maturity; a compatible design must avoid that mechanism. [Original proposal](https://gnusha.org/pi/bitcoindev/CAMnWzuVi2qK6FhML=R5M1r95i1346J5YpUuOd1=StSdAZXfG3g@mail.gmail.com/).

SegWit documents upgradeable witness semantics, but those do not waive public amount accounting. [BIP141](https://bips.dev/141/).

Extension blocks are another reserve-backed route. Litecoin's LIP2 describes an integrating transaction, entry/exit rules, and an additional UTXO set. That architecture departs further from the original request than native carriers, so it is not the default recommendation. [LIP2](https://github.com/litecoin-project/lips/blob/master/lip-0002.mediawiki). Its companion proposal discusses confidential transactions and extension-block tradeoffs. [LIP3](https://github.com/litecoin-project/lips/blob/master/lip-0003.mediawiki). These sources establish architectural precedent, not Bitcoin adoption.

## What survives and what changes

| Requirement | Native-carrier reserve candidate |
|---|---|
| Bitcoin consensus is final authority | Yes, conditional on a properly specified, adopted upgrade |
| Explicit txid:vout spentness | Can be preserved |
| Seed-derived scan/spend receiving | Can be preserved; cryptographic composition still needs review |
| Confidential individual balances | Intended; aggregate reserve, deposits, withdrawals and fees public |
| No note tree or nullifiers | Can be preserved |
| No authoritative indexer | Can be preserved; validating nodes need all consensus proof data |
| No separate blockchain/federation | Can be preserved |
| No extra monetary accounting structure | Cannot be preserved in this candidate: public reserve plus confidential claims |
| Ordinary P2TR appearance and existing-wallet support | Not established; specialized outputs and wallet changes likely |
| Literal BIP352 interoperability | Not supplied by our current inspired derivation |
| Deployment without Bitcoin consensus adoption | No |

BIP352 addresses reusable receiving via derived destinations; it does not change amount accounting. A custom confidential script must not be called a standard BIP352/P2TR output without demonstrating compatibility. [BIP352](https://bips.dev/352/).

## Unresolved engineering gates

1. Specify an exact legacy-valid serialization and upgrade hook. Every accepted upgraded block must remain acceptable to unmodified nodes. Ordinary existing input types must retain their current rules.
2. Bind every created confidential output to its complete metadata and proof before admission to the UTXO set. Merely checking proofs when a coin is eventually spent is insufficient. Define consensus data commitments, transport, resource bounds, and availability; a hash alone does not make data available.
3. Define settlement exactly: reserve initialization, deposits, withdrawals, fee accounting, ordering, conservation, and rejection of unauthorized reserve spends. Resolve block-level aggregation and mempool policy without exposing a miner discretionary custody path.
4. Demonstrate withdrawal and child-transaction behavior under reorgs. If settlement inputs change, its txid and withdrawal outpoints can change. A waiting rule can mitigate ordinary reorg exposure but is not a proof against arbitrary-depth reorgs. Existing coinbase maturity must remain intact.
5. Prove the reserve invariant by induction and audit cryptography independently. A fixed public reserve can cap base-chain issuance even if confidential proofs fail, but forged claims could still steal backing from legitimate holders. Reserve conservation is not user-fund safety.
6. Quantify privacy. Visible graph structure and known deposits can reveal amounts by subtraction, especially for single-input/single-output flows. Receiver key unlinkability is not anonymity against graph and network observations.
7. Benchmark validation, bandwidth, UTXO/undo storage, scanning and denial-of-service limits. Our measured proof baseline is evidence about the current prototype only.
8. Publish a specification and obtain independent review before treating this as a deployable proposal. Technical soft-fork compatibility does not establish community acceptance or activation readiness.

## Next decisive experiment

Preserve the current hard-fork lab as a reference. First write the carrier/reserve transaction and block specification, then construct deposit, confidential transfer, withdrawal and fee examples accepted by an unmodified node. Run a second upgraded node that enforces the extra rules. It must reject malformed commitments, unauthorized reserve spends and over-withdrawals that the unmodified node may accept. Exercise competing branches and recovery against both views.

Passing those tests would be evidence of backward compatibility for the tested cases, not a mathematical proof for every block. This experiment can run privately with an unmodified reference node; public-testnet publication alone would not cause public nodes to enforce the proposed confidential rules.

Decision: the exact implemented single-accounting design remains a hard-fork candidate. The strongest nearby soft-fork research candidate is native carriers plus a consensus-controlled reserve. Pursuing it requires explicitly accepting the reserve/accounting compromise; no implementation switch has been made by this assessment.
