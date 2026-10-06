# Wallet pilot decision

The user requested a wallet pilot **after** architectural/security and reproducibility gates. Those gates are not all satisfied. The existing wallet helper and Alice/Bob harness are engineering tools, not an end-user product. No pilot release is claimed.

Release gate evidence required:

1. Named independent consensus and cryptographic reviewers provide a written disposition of the architectural questions; blocking findings are fixed and retested.
2. An outside developer reproduces clean builds and all decision/invariant checks. Local clean builds alone do not constitute independent reproduction.
3. Fixed-height and deployment-state activation, contextual mempool/reorg recovery and snapshot rules are reviewed. The fixed-height test is only one part of deployment validation.
4. Cold-validation and sustained adversarial-load measurements justify work/fee limits on representative hardware; exit-contention and starvation behavior are documented.

The smallest subsequent pilot should provide create/restore account, copy reusable receive address, historical scan with progress, balances, fee-policy choice, send preview, transaction status, backing-conflict retry and descendant recovery. It must use an explicitly experimental network and make its activation assumptions visible. No custodial server or authoritative private balance indexer should be introduced.

Acceptance tests must include two people using independently started nodes/wallets, seed-only restoration after losing application state, temporary disconnection, conflicting backing selection, and rollback. Accounts or balances from the original hard-fork lab must not silently migrate into this compatibility experiment.
