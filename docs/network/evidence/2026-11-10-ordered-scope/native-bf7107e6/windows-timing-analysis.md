# Windows timing gate — run42927983955

## Custody and actual result

Failed job130219221283, artifact13815117554: all3manifest member hashes verified; ZIP SHA256 cc1f9ba79a9aaf44f9bb45ddd38b5ff141efd039b147c63834c91c2bae8977ee. Actual checkout57b13eb53a23b35018da3cc6b1d9c0dcb2aa2904, treecce2e94d568aaba7e3dad11d02ab49207dd832d5, exactly matching branch bf7107e62e7fa35547f86795b56d7f8d7b3793fc tree. No Rust files differ from661c74e7. Windows2025/x64 image20261103.298.1, witness-timing, unchanged4096/240s/2s.

At failed snapshot:242.002462s wall and103.046875s processCPU;3065ACK/read each direction, actual TLS leftsent4,037,085/received4,037,050, matching peer counters.64publications/1,051,712bytes/3001evictions each. Both original scope checks returnedOk. Both writers failed their original240s overall gate before sending block3065. There was no recorded2sobject-operation timeout. Full harness242.10s includes unwind. Partial completed reads are exact; complete4096gate FAILED.

Verified comparator from run42925924991, artifact13813812655: all3hashes verified; ZIP SHA256 b0369d3d12db132aa523158848fb3b1f74766a2ba00f67c6a01d588ec6d1ae44. Its actual source073f94ef/treec0ab4c8 equals branch1ae9fa35 tree; Rust is unchanged relative661 and bf. That recorded PASS is193.576701s lifecycle/119.265625s processCPU,4096exacteach andTLS5,395,050/5,395,015. A separately reported224spass, if applicable, is a different witness and is not substituted for this artifact. Preserve both this PASS and the latest FAIL. Same image/lock/features do not identify the same physical hosted machine, storage performance or load.

## Actual elapsed phase comparison

| Phase | PASS sideA/B mean(ms) | FAIL sideA/B mean(ms) | FAIL sideA/B maximum(ms) |
|---|---:|---:|---:|
| Receiver Begin |6.117/6.003|6.917/7.074|93.179/278.034|
| Receiver Commit |20.852/20.574|38.966/41.512|1088.005/1054.602|
| Sender Begin response wait |12.051/14.511|27.407/26.042|1088.890/1011.457|
| Sender Data response wait |13.020/10.486|6.693/10.237|419.860/1055.698|
| Sender Commit response wait |21.995/22.075|43.958/41.784|1055.686/1088.889|

FAIL Session CV waits total70.472/73.059ms, max0.387/0.552ms; TX CV waits total56.504/67.394ms, max0.772/0.307ms. No lock timeout/stopped/poison bucket or overflow is nonzero. Sender/receiver TX acquisition phases are also submillisecond at maximum. Ownership waiting is not the substantial recorded elapsed cost in this row.

Receiver Commit totals119.431/127.234s, while Begin totals21.202/21.682s and Data totals32.928/33.866ms. Commit is the largest measured handler and roughly twice the comparator mean. This span includes decode/hash, a fresh quota scan, staging writes plus file sync, checked write-through rename, publication readback, and ordered cache verification. Subsequent Worker readback/delivery is outside this handler. It is not yet a separate measurement of flush, security-open or quota CPU.

Poll elapsed97.738/89.404s overlaps receiver and opposite-direction work; maxima35.663/36.279ms. Session operations total0.848/0.825s. Nested/cross-thread elapsed totals must not be added to obtain runtime or send-only time. ProcessCPU is aggregate including both peers and diagnostics; failure CPU is to-snapshot, comparator CPU includes completed cleanup. Counters do not isolate observer CPU, and these are not paired feature-on/off trials. Clock/atomic overhead and hardware/storage variability cannot be assigned causal percentages from these rows.

## One bounded implementation proposal

Remove only the redundant Windows private-file preflight inside endpoint::read_meta. Current read_meta calls private_file(path), which opens, validates and discards a handle, immediately before storage::open_read(path), which repeats the path/ancestor/SID/private-file checks on the actual handle used to read metadata. On Windows private_file and open_read both pass through windows::open/private; actual open_read uses the stricter read-only sharing profile.

Retain missing-file mapping and the complete checked open_read on the consumed handle. Keep Unix behavior unchanged for this Windows-focused slice. Do not remove post-publication full-byte/digest verification, repeated authorization, fresh quota accounting, staging sync_all, write-through rename, identity checks, resource bounds, cache limits, or deadlines. This removes one discarded checked open per metadata lookup without caching permission or trusting stale metadata. Both the durable publish readback and ordered_cache_meta path make such metadata lookups, as do eviction/readback paths.

Before claiming a benefit, place old/new metadata-read paths in one bounded Rust native benchmark using identical private64-file stores of16KiB files (at most2MiB synthetic payload on disk) and6paired trials with alternating old/new order. Record actual process/threadCPU and wall plus exact metadata/readback agreement; include bad ACL, hardlink, reparse, missing-file and writer-sharing refusal fixtures. Obtain narrow independent security review that the actual consumed file handle keeps all checks. Then execute unchanged4096/240/2s qualification. Keep before/after artifacts, including losses. This is an algorithmic repeated-open reduction proposal, not a measured speedup or a guarantee that the240sgate will pass.

No product edits, builds, CI dispatches, blind retries or assertion changes were performed for this analysis. Normal full-matrix bf outcomes are separate and are not presumed green here. Canonical MVM implementation can continue separately while this retained native performance gate remains open.
