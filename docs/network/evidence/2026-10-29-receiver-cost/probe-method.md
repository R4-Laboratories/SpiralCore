# SC-OPT-002 receiver diagnostic

This is a local Rust diagnostic, not production code or a WAN benchmark. The
production baseline is `fedf38a8e308f3064b6ca3006aaf721a418c0d80` (unchanged
production code from measured source `b552017`). The pre-registered comparison
and thresholds are in repository commit `74ee59d`, plan
`docs/plans/2026-10-29-receiver-efficiency.md` and the optimization ledger.

The fixture is **per-byte xorshift64, seed 42**: advance xorshift (13,7,17) for
every output byte, taking its low byte. A 1 MiB target differs from the base in
exactly 32 bytes at offset 524288, XOR 0xa5. This is deliberately named separately
from the WAN fixture, which emits eight little-endian bytes per PRNG step.

N = 0,64,256,768 unrelated 32-byte publications, two per namespace. The active
namespace has exactly one identical base before each observation and base plus
target afterward. Templates are made only through the public Endpoint API.
Every new observation receives a fresh private clone of the closed template.
Each cloned file and directory, and parent creation, is synced before timing.
No target is already published. No OS cache flushing is attempted. Actual
namespace/publication counts, publication lengths and manifest readiness are
recorded. Successful owned working copies are removed after exact reopened
comparison; failed copies and failed receipts remain and stop that invocation.

Four arms: raw/sparse × max_nodes=262144/1. Both policies set graph_cache_bytes=0
and graph_cache_entries=0. The tiny node budget changes graph admission and
reservation, not just hash work, and prevents useful DAG readiness for this
random 1 MiB raw target. It still permits small filler graphs. Both policies
use the same base publication with a manifest produced during template setup.
Sparse ignores that optional base manifest and refers to the full ObjectId.

**Primary metric:** `local_operation_total.cpu_ns`, whole-process CPU from
before recipe/frame construction through validated Published and request-buffer
teardown. It includes synchronous sender-like preparation, Endpoint calls and
bounded instrumentation. It is not receiver-only CPU. `handler_stages` brackets
each public Endpoint::handle call; its sum is a subset of the primary metric.
Never add the handler, dispatch or frame-preparation CPU to the primary total.
Fixture generation, clone/sync, Endpoint::open and close/reopen/exact verification
are recorded separately. All handler timing uses preallocated frame-sample
storage; reply validation and bounded reply-body capture happen after the
handler clock. Modeled frame bytes are body length + 56, not actual transport
traffic. No sockets, HTTP, TLS, WAN or joule measurements occur here.

The primary pre-registered cell is N768/sparse/max_nodes262144. Supporting cell
N768/sparse/max_nodes1 uses the same threshold. The joint target requires BOTH
to achieve old/new paired geometric mean >=1.10 with >=6/8 new-faster pairs.
All populations, policies and negative controls remain in results. Begin and
Finish handler times are mechanistic secondary metrics, not a replacement gate.

`schedule.json` fixes eight paired blocks: four old→new and four new→old.
One process invocation executes a block of 16 observations. Population order
rotates by block; all four arm positions occur twice over eight blocks. The
same block has the same internal order for old and new. The parent orchestrator
owns execution of the external AB/BA process schedule. A single initial baseline
block is characterization only, not the final effect estimate.

Commands after review:

```sh
./build-probe.sh old
build-old/receiver-probe prepare --root /Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/experiment
build-old/receiver-probe run --root /Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/experiment --out /Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/baseline-block0.jsonl --block 0 --variant baseline
# After the reviewed production change is committed:
./build-probe.sh new
# Run the immutable old/new binaries in schedule.json order, one block per process.
build-old/receiver-probe run --root /Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/experiment --out /Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/pair0-old.jsonl --block 0 --variant old
build-new/receiver-probe run --root /Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/experiment --out /Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/pair0-new.jsonl --block 0 --variant new
```

Use absolute root/output paths. An initial bare-relative-root setup invocation
failed before any template or observation; its error receipt and empty directory
are retained in `setup-attempt-1-error.json` and `experiment-setup-failed-relative-path`.

The build script refuses to replace existing build records and rejects dirty
production inputs. It explicitly selects its owned target directory and rustc's
native host target, so inherited target-directory/cross-target configuration
cannot redirect the build away from the executable being preserved.
SOURCE_SHA256 is SHA-256 of a per-file SHA-256 manifest: **all files in both
production source directories** and both crate Cargo.toml files in sorted path
order, followed by the probe Cargo.lock hash line. The full manifest is retained.
Probe source snapshots/hashes, compiler details, flags, source revision and
executable SHA-256 are also preserved. The code fingerprint is rechecked after
compilation. Old/new must use the same helper source and dependency lock. No
production source is edited by this helper.
