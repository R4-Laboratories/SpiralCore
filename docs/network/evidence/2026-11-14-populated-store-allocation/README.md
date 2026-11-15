# Populated owned-store allocation and recovery qualification

This #258/#49 checkpoint adds a Rust qualification witness and native CI checks. [PR #287](https://github.com/R4-Laboratories/SpiralCore/pull/287) merged as `525cd69f3dbda662361fd4b1e9833fb93d809316`. The released tree equals the independently reviewed candidate and CI merge tree. The candidate is `084b1c8c32b8bf5678b678596838a0229d781b11`, tree `fe030e73d1971d2baea9d3733cd7c7d2f1aed5a1`, based on `15f885eb`. Its CI merge `5b783f1b` has that same tree. All 27 executed checks pass; manual genuine-export is skipped/NOT_RUN. Check states are not summed test counts. No product code, API, codec, wire format, authority or deadline changes. Runtime source/manifests for the transfer/object-store/Windows core remain identical to reviewed `adcc7197`; MVM `611a3c76` still consumes that reviewed pin.

Two worker threads each populate a real private store with 64 distinct, product-generated 16 KiB Raw publications with trailing NUL bytes. Two namespace directories, 32 recovery stages, one profile stage and the original markers reach 101 observed entries. Recovery removes the 33 stage files and preserves all 64 publications, leaving 68 entries. Readbacks compare every byte. A new capture of the same directory models component restart; it does not prove installed crash recovery or original-parent re-admission.

Successful content retirement preserves the original writer lock until the lease drops. The negative case adds a 33rd valid object stage, observes 102 entries and requires `Limit` before any unlink. It checks every publication, stage, profile and overflow byte, the original held/named owner identity and the complete unchanged entry count. The failed lease remains held through those assertions; the fixture never claims a runtime refund. Phase coordination has a five-second bound, and all original workers join before results are accepted.

## Allocation domain and measurement boundary

The dedicated test binary installs a `System`-forwarding global allocator. It counts requested Rust heap bytes from child-process startup through the sampled lifecycle, including fixture data, libtest state and conservative simultaneous old/new realloc charges. Failed requests undo their provisional live charge; a failed realloc retains the old charge. Bookkeeping never unwinds on integer overflow and permanently invalidates an overflowed/underflowed observation. Every accepted row requires valid bookkeeping.

This counter **excludes direct libc allocations, allocator overhead/fragmentation, native stacks and RSS**. It is an observed workload peak, not a proof of a worst-case iterator maximum or hard memory ceiling. The comparison is the existing modeled two-component quote, `2 × (SmallV1.operation_charge() + 1 MiB) = 18,681,720 bytes`; this test creates no original-parent permit or authority. It does not prove the 1 MiB filesystem workspace in isolation. Concurrent store lifetimes do not establish deterministic overlap inside both actual directory walks.

Compile-time witness/helper hashes are compared with checkout bytes. The executable is streamed through SHA-256 before and after the fixture; both hashes must match. Hashing uses a fixed 32 KiB native stack buffer, outside the timed workload window. Workload wall/process CPU stops after both workers join, before post-fixture hashing and JSON serialization. Heap accounting includes startup/pre-fixture Rust requests, so its window differs from the workload clocks. The instrumentation has its own cost; these observations are not uninstrumented performance comparisons.

## Actual observations

Each complete row generates and verifies **4,194,304 application bytes**, with **4,250,368 encoded request bytes and 55,808 encoded reply bytes**. These are actual in-process product frame encodings, not transmitted network/TLS/IP bytes. Additional recovery readbacks and byte-preservation checks are included in lifecycle costs without being counted as new application sends.

| Host | Peak requested Rust heap (bytes) | Instrumented lifecycle wall (s) | Child-process CPU (s) |
| --- | ---: | ---: | ---: |
| Native Linux x64 | 2,228,363 | 0.171230104 | 0.248198794 |
| Native Linux ARM64 | 2,234,363 | 0.314974694 | 0.410214840 |
| Native macOS ARM64 | 2,232,126 | 0.448168042 | 0.642631000 |
| Native macOS Intel | 2,232,650 | 1.267331709 | 1.407270000 |
| Wick Linux x64, final direct run | 2,263,327 | 1.546448361 | 0.126914196 |

`native/verified.json` and `wick-summary.json` retain the exact source/executable identities, setup, sender work outside receiver dispatch, receiver dispatch, initial readback, recovery, retirement and overflow-refusal CPU. Worker wall durations are explicitly sums and may overlap; do not add or subtract them to invent a send-only latency. CPU can exceed wall time for the two concurrent workers. These uncontrolled host observations do not establish a speedup, energy, fleet capacity or a matched zstd comparison.

The Wick final direct run uses the unchanged executable `4d70502b…`; external before/after checks match its self-reported digest. All 166 source/fixture hashes match before and after execution. Five public owned-store controls, the retained descriptor lifecycle test, all 18 Linux owned primitives and strict all-target/all-feature Clippy also pass. Formatting and YAML parsing pass. The seven native FlowMux component jobs and remaining source-PR checks pass, preserving the existing sustained delivery, refusal, security and deadline assertions. Four Unix jobs execute the new allocation test; no Windows allocator result is inferred.

The final retained unit receipt records an exited process, exit 0, 6 GiB memory maximum, zero swap and 200% CPU configuration before unit disposal. Its observed cgroup peak is a separate whole-command/kernel accounting domain; it is not the requested Rust heap observation or an installed component bound. Builds reuse the existing Cargo target. No MVM, cloud host or payload corpus was created.

## Review and retained failures

Focused independent review corrected a potential allocator-bookkeeping unwind under extreme requests, expanded the all-byte preservation assertions and checked source/executable/cost boundaries. The first fixture used an unknown root overflow name and correctly received `Storage` before the intended count limit; using a syntactically valid excess recovery stage exercises the unchanged `Limit` assertion. This is a fixture correction, not a product RED/GREEN claim. Earlier successful observations retain their own source/measurement scope. Old CI runs were cancelled after the concrete reviewed hash-binding successor; cancellations are not passes. Original raw logs and the failure journal are in `wick-logs.tar.gz`.

All seven original native ZIPs, their API digests/sizes, 170 member hashes and 163 embedded checksums verify. Four parsed allocation rows match their raw single-test logs. The packet preserves the original data and reports separate scopes rather than combining nested counts.

## Remaining acceptance

#258 stays open for full direct-libc/allocator/iterator/concurrency funding acceptance and the installed boundary; an observed Rust sample cannot close it. All five #42 outcomes remain active: installed mailbox ownership/budgets/shutdown/recovery; persistent recovery and sustained large/control traffic; canonical FlowMux and separately authorized owner access; canonical two-host WAN and supported role/backend acceptance; capacity/fairness and the complete upstream contribution. Ari's compatible signed runtime and private MVMD integration remain external gates. This checkpoint proves no booted MVM/HVF, WAN, Windows MVM hosting, hard memory bound, FIPS/CUI or final delivery qualification. No final upstream MVM PR is submitted.
