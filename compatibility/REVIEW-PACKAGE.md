# Independent review entry point

Updated October 6, 2026. Prepared for review; no independent reviewer has signed off and no external messages have been sent.

## Scope and reproduction

Read SPEC.md, COMPROMISES.md and ACTIVATION.md first. Inspect reserve.cpp/reserve.h, reserve.patch and patch_core.py against pinned upstream e8e7e91a1144c378dff4da2e2a562eb0f3f2e1d6. Rust consensus code is in ../crates/{core_bridge,confidential_tx,proofs,commitments,keys}. Wallet scanning is in receiver_scan and the local integration harness.

Run from the project root:

```powershell
./scripts/cargo.ps1 test --workspace
./compatibility/build.ps1 -Upgraded
python compatibility/v2_demo.py
```

The build uses this machine's configured MSVC/vcpkg dependency paths. Independent reproduction requires adjusting those paths and independently building the pinned stock binary. report-v2.json records actual executable hashes and results; runs contain isolated node histories and logs. Test seeds are disposable public test material. Source inventory hashes are in SOURCE-SHA256.json inside the archive; review-manifest.json in the working tree records archive and binary identities. A hash is an identity aid, not a review or reproducible-build attestation.

## Claims reviewers should try to break

* Supply: from R=S=0, every accepted transition preserves R=S and nonnegative individually bounded claims. Fees and withdrawals reduce claims and backing equally. Negative-equivalent commitments, wraparound, invalid complements and excesses must fail.
* Authorization: stealing a carrier or substituting selected backing, fees, payment destinations, ordering or native outpoints must fail without the necessary signing keys. Audit inner/outer signature composition and input-derived receiver keys, including rogue-key/multi-party cases; the wallet currently assumes a single sender.
* Native state: normal UTXO spentness and undo suffice. Metadata supplied by a spender must match the native script commitment. No wallet cache or indexer is authoritative.
* Cryptography: audit generator derivation, binding assumptions, proof soundness and canonical encoding, commitment parsing, secret nonces, encryption and key derivation. This is not literal BIP352 interoperability. Breaking proof soundness can forge claims and steal public backing even though old-node public issuance remains capped.
* FFI and parsing: adversarial lengths, integer overflow, allocation before bounds, panic boundaries, trailing bytes, mismatched previous coins, duplicate inputs and differing transaction representations must fail closed.
* Validation cache: full statement includes commitment, proof bytes, binding digest and exact/range mode; successes only; bounded 1024-entry FIFO; mutex poisoning falls back to verification. No cached spentness or authorization. Test races, eviction, cold/warm equality and adversarial unique statements.
* Relay: witness-mutated rejection caching, legacy txid inventory, annex recognition, package limits, replacement/eviction, reorg replay, and CPU use from late-invalid/unique proofs require adversarial testing. Existing functional demonstrations cover only a subset.
* Wallet: bounded backing reselection must preserve payment intent and refuse claim-input conflicts. Descendants of a rebuilt transaction require rescanning/rebuilding/re-signing. Local mempool availability does not guarantee inclusion or prevent starvation.

## Evidence versus missing evidence

The harness covers stock/upgraded differential attacks, upgraded peer relay, ordinary standardness restrictions, independent deposits, reserve-free payments, selected-fragment exits, conflict reselection, stale/rebuilt descendants, confidential-fee payments, reorgs, chainstate rebuild and exact seed recovery. Consult the latest successful report rather than assuming any listed test ran successfully.

The proof cache reduces duplicate mathematical verification only. Cold unique proofs remain large and expensive; successful-prefix reuse does not bound hostile traffic or full-block verification time. Current timings are RPC-level samples, not a resource-limit specification. Current limits and fragment sizes are research parameters.

Missing: independent cryptographic/protocol audit; coverage-guided parser/FFI fuzzing; broad upstream Core suite; cross-platform deterministic validation; sustained adversarial load; proof-cost/fee calibration; activation-boundary implementation and tests; economic exit-contention analysis; a vetted proof-size improvement. None is waived by passing self-tests.

Review output should identify exact source revision/hash, assumption, severity, minimal reproducer, expected/actual behavior and disposition. Consensus findings require replayable vectors and differential block cases. Do not merge a cryptographic replacement based solely on benchmark improvement.


## Updated review surface

The patch now also modifies block-template selection and contextual activation. Inspect all nine changed Core files, not only reserve.cpp. Run activation_demo.py and hostile_demo.py in addition to v2_demo.py. Review the conservative full-mempool eviction at the boundary, historical creation-height classification, shared limits in mining/validation and the resource-unit model. A deterministic 4096-case malformed-byte FFI corpus was added; it is not a coverage-guided fuzzer. REPRODUCE.md and the source archive describe local reproduction. Independent sign-off remains absent.

The clean-directory reproduction completed successfully; clean-reproduction.json records matched implementation files and report identities. STATUS.md distinguishes that local result from independent reproduction.
