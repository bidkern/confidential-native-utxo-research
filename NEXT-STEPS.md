# Remaining work in priority order

1. Secure named independent consensus and cryptography reviewers and written proceed/revise/stop verdicts. Requests are open in issues #1 and #2; no reviewer has accepted yet.
2. Obtain an outside developer's full-node reproduction report (issue #3). Hosted Rust/comparator CI is useful but is not outside architectural review or the full Core demonstration.
3. Review and qualify the new isolated secp256k1 adapter (experiments/secp256k1-bulletproofs), including its historical dependency and two-line scratch cleanup patch. Exact-bound and byte-compatibility checks pass locally; independent cryptographic review, sanitizers, differential vectors and peak-memory measurements remain required.
4. Resolve review findings, then design a versioned proof upgrade and recalibrate CPU/weight/fee limits on representative hardware. Keep the current 36-payment work cap until evidence supports a change.
5. Complete deployment-state activation, wrapped-output/snapshot coverage, sustained multi-peer stress, parser fuzzing and multi-user exit fairness tests.
6. Build the wallet pilot only after the review and qualification gates pass. Public Bitcoin deployment requires a separately agreed consensus upgrade.

Original project code is MIT licensed, as approved by the owner. GitHub is already available; Delving Bitcoin is optional for broader outreach. Accounts do not substitute for engaged reviewers.
