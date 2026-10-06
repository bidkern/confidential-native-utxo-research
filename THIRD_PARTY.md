# Third-party provenance

reserve.patch modifies Bitcoin Core commit e8e7e91a1144c378dff4da2e2a562eb0f3f2e1d6. Bitcoin Core portions remain under their upstream MIT license, reproduced in BITCOIN-CORE-COPYING. The build obtains that upstream repository independently; it is not vendored here.

Rust dependency versions/checksums are pinned in Cargo.lock; their upstream licenses continue to apply. No claim of authorship or relicensing of upstream code is made. Public proof-system sources are attributed in PROOF-FEASIBILITY.md. Original project code is MIT licensed under LICENSE. Upstream code retains its original licenses.

The isolated secp256k1 experiment fetches mimblewimble/secp256k1-zkp at b247e1ec8ed62b9abf123dc83189d253e17d488d. Its MIT notice is reproduced in experiments/secp256k1-bulletproofs/UPSTREAM-COPYING. build.rs applies two local scratch cleanup insertions to a build-directory copy; neither the upstream checkout nor consensus dependencies are silently modified.
