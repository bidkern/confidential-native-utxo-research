# Private Bitcoin Core integration

Implemented after the standalone prototype at the user's request for the two-node demonstration. Baseline: Bitcoin Core e8e7e91a1144c378dff4da2e2a562eb0f3f2e1d6. This is a hard-fork research patch, not a BIP or deployment. The original project checkout and its earlier changes were not reused as protocol code.

## Consensus authority

Core's normal CCoinsViewCache and chainstate database hold the confidential coins under native txid:vout. CheckTxInputs fetches those records, checks maturity, invokes the statically linked Rust verifier and returns the validated public fee. Core's normal block processing consumes and creates those outpoints. Native undo records include the extended outputs. There is no parallel monetary ledger, note tree, nullifier database, trusted wallet verifier or authoritative indexer.

Each daemon has its own verifier. The wallet may construct an invalid transaction but cannot tell a node to accept it. For the reserved CNU version, the linked verifier checks every input's ownership signature and all value proofs. Ordinary transactions retain ordinary validation and cannot spend confidential outputs through an unknown-witness anyone-can-spend interpretation: the negative type tag fails their ordinary input amount checks.

## Experimental encoding

- CNU transactions reserve version 0x43554e30.
- Confidential CTxOut uses nValue=-2 as a type marker, not an amount. Its script is OP_2 PUSH32 ownership_key. Fixed fields following the script carry a 33-byte commitment and 56-byte encrypted opening. Transparent P2TR outputs retain their normal encoding.
- After nLockTime, CNU transactions carry an explicit 64-bit fee and CompactSize-prefixed Rust proof/signature payload, capped at 250,000 bytes. Input scriptSig and witness must be empty, and sequence final.
- Core reconstructs the payload's expected base statement from the actual native transaction. Exact equality is required. Referenced input records are supplied only from Core's UTXO view. A solved-block attack mutating a native commitment while preserving its payload is rejected by both nodes.
- The redundant payload is a deliberate adapter for this spike, not the final wire design. All payload bytes, including proofs/signatures, are in the native base transaction and affect its txid. There is no witness discount. Created outpoints use Core's hash, never the internal Rust digest. This differs from the standalone witness-separated design.
- Coin and undo compression reserve uint64::MAX as the confidential amount tag, with the commitment and encrypted opening stored alongside the script. Restart and chainstate rebuild tested this path.

Coinbase remains transparent. This lab retains Core regtest's 150-block subsidy-halving interval rather than the simulator's 210,000-block interval. The demonstration stays below its first halving. Core's normal coinbase rule consumes the explicitly validated fees. The demonstrated flows are transparent->confidential and confidential->confidential; full script compatibility is not claimed.

## Isolation and wallet surface

The daemon refuses non-regtest startup and uses P2P magic 43 4e 55 30. The harness binds RPC/P2P to loopback, disables discovery/automatic peer connections, and explicitly connects its two nodes. Use only the separate generated datadirs; do not mix them with stock regtest data.

The test wallet is the external Rust cnu-wallet helper, not Core's production wallet (ENABLE_WALLET=OFF). It derives keys, builds transactions and scans the chain served by locally validating nodes. Its cache is disposable. getrawtransaction/getblock/gettxout expose commitments and ciphertext and report null values for confidential outputs.

Legacy aggregate amount/statistics RPCs, PSBT, descriptors, production fee estimation, general script paths, relative locks and ordinary-wallet import are not comprehensively adapted. Aggregate plaintext-value RPCs must not be used as supply audits on this chain. The supported demonstration interface is integration/lab.py. The helper expects well-formed local requests and is not a hardened service.

## Evidence and limitations

integration/report.json records transaction IDs, rejection evidence from both nodes, reorg hashes, persistence checks and the daemon hash. Per-run console/debug logs remain available. Invalid-block tests disconnect peers so that each node independently validates the attack rather than returning a peer-populated cached result.

The same verifier running in two processes establishes independent execution and state handling, not independent cryptographic implementations or audit. Proofs remain expensive. Verification is deliberately repeated in CheckTxInputs and CheckInputScripts, and the wire duplicates data. No claim is made that the entire upstream Core test suite has passed or that this partial integration is suitable for real funds.
