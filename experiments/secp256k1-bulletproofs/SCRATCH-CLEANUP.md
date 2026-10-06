# Historical verifier scratch cleanup finding

During local Windows release tests, a valid aggregate verified, then repeated single-byte proof mutations produced STATUS_HEAP_CORRUPTION / STATUS_ACCESS_VIOLATION. Inspection found two missing scratch-frame deallocations in upstream src/modules/bulletproofs/rangeproof_impl.h: returns after failure to deserialize A/S or T1/T2 (upstream lines 246 and 264).

The verifier allocates an inner scratch frame before these branches. Returning without releasing it makes the caller release the wrong frame, leaving the outer frame allocated. Repeated malformed points accumulate frames; the historical scratch implementation uses fixed arrays of five frames and guards overflow only through VERIFY_CHECK. The original experiment build did not enable VERIFY. This is a concrete reason not to use the unmodified historical candidate in a node.

build.rs applies exactly one insertion at each matching branch in an OUT_DIR source copy:

```c
secp256k1_scratch_deallocate_frame(scratch);
return 0;
```

The original pinned checkout remains unchanged. The patched build enables VERIFY and reruns each-byte proof mutation tests on one reused context, exercising the cleanup path repeatedly. Passing this regression does not establish memory safety elsewhere or independent approval. This report is a local diagnosis, not a claim about current downstream maintained implementations. No upstream advisory or reviewer acceptance is implied.
