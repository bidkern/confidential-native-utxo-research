# Remaining work in priority order

1. Secure named independent consensus and cryptography reviewers and written proceed/revise/stop verdicts. Requests are open in issues #1 and #2; no reviewer has accepted yet.
2. Obtain an outside developer's full-node reproduction report (issue #3). Hosted Rust/comparator CI is useful but is not outside architectural review or the full Core demonstration.
3. Select and review a secp256k1-compatible aggregated-proof implementation. Build an isolated adapter with exact monetary bounds, transaction transcripts, vectors and memory/cold-verification benchmarks. The current Ristretto comparator cannot replace our commitments.
4. Resolve review findings, then design a versioned proof upgrade and recalibrate CPU/weight/fee limits on representative hardware. Keep the current 36-payment work cap until evidence supports a change.
5. Complete deployment-state activation, wrapped-output/snapshot coverage, sustained multi-peer stress, parser fuzzing and multi-user exit fairness tests.
6. Build the wallet pilot only after the review and qualification gates pass. Public Bitcoin deployment requires a separately agreed consensus upgrade.

Owner decision still open: license for original project code. MIT was suggested; no grant has been added without a choice. GitHub is already available; Delving Bitcoin is optional for broader outreach. Accounts do not substitute for engaged reviewers.
