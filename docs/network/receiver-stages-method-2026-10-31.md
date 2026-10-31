# SC-OPT-018 coarse receiver diagnostic

`examples/receiver_stages.rs` calls the unchanged public `Endpoint::handle`.
It is an in-process attribution diagnostic, not a WAN measurement or a receiver
optimization. Run only in the parent's exclusive build/timing lane, after the
source and executable are frozen:

```text
receiver_stages NEW_STORE_PARENT NEW_JSONL SOURCE_COMMIT_40HEX
```

Both paths must be new; place the JSONL outside the store parent. No automatic
retry is performed. Errors retain the store and previously flushed records.
Successful stores are removed only after the endpoint is dropped, reopened,
and all eleven objects are independently compared byte for byte. The output
retains exact object IDs, SHA-256 digests, mode, frame counts and error status.

The fixed dose is **44 actual objects**: 1 and 16 MiB, two clock modes, one seed
and ten updates per sequence. Use trial zero from the persistent-session fixture:
xorshift64 13/7/17 emitted little-endian, independent replacement at update five,
and nine 32-byte XOR-a5 edits. Edit offsets are midpoint plus 64 times the index
within each pre/post-escape group. All eleven object identities must differ.
Seed and escape are Raw; the nine localized changes use Sparse. This isolates
receiver handling from sender codec selection. Modeled HTTP-body counts are
not bytes transmitted by this in-process experiment.

Frames, expected replies, input identities and modeled sizes are constructed
before the receiver timer. The outer interval covers all handler calls and
reply collection. `stages` adds only three coarse intervals: Begin, **all Data
frames as one group**, and Finish. No Data-frame, COPY-operation, directory-entry
or byte-loop timers are added. Reply validation, logging, fixture generation,
endpoint opening and stopped-store verification lie outside the outer interval.
The same static authority and disabled cold-DAG/graph-retention policy apply to
both clock modes. CPU is process CPU; no RSS, power or energy is measured.

At 1 MiB the order is outer-only then stages; at 16 MiB it is stages then
outer-only. Each mode starts with a fresh store and retains every observation,
including seed and the first update. There is no warmup discard. For each size,
pair the eleven corresponding positions and report their staged/outer-only
outer-CPU ratios. The registered attribution screen is the median ratio at most
1.10. A failed screen prevents interpreting the stage attribution for that size;
it does not delete observations or trigger another run. This is one sequence per
size/mode: positions are different updates, not independent replicate trials.

Begin/Data/Finish clocks are **nested within outer CPU/wall**. Never add them to
the outer total. They distinguish broad receiver phases, not base hashing from
filesystem reads or target hashing from durable publication. A follow-up split
requires evidence that a broad phase contains a material opportunity; do not
reuse the historical intrusive per-entry profiler as if its attribution passed.

Focused self-tests establish fixture uniqueness/escape, exact prepared frames,
both clock modes with complete reopened sequences, preservation of existing
paths, and fail-closed malformed Data without publication. Those small test
operations are qualification and are not part of the 44-object registered dose.
