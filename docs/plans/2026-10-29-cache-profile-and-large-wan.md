# Native cache profile and larger WAN transfers

Registration state: NOT_RUN. Starting source is
`f0966ebedcb8994f0985b54e9a2b961eeb8be914`. The user has authorized this continuation:
measure the remaining native Rust cost, preserve exact recovery and byte savings, and
exercise larger objects. MVM and a WASM port remain outside this experiment.

## Phase 1: direct quota-scan attribution (SC-OPT-004)

1. Preserve a source snapshot and reuse the existing Rust receiver probe under
   `local/cache-profile-20261029/` in the primary checkout. Record source, helper,
   compiler, feature and executable hashes. Use hardware SHA on this native ARM host.
   Do not alter any earlier evidence. Dependency: none. Risk: low.
2. Instrument only the snapshot's `Endpoint::disk_usage`, with bounded aggregate
   call, entry, CPU and wall counters. Read counters outside timed handler calls.
   Preserve enumeration, errors and all quota decisions. Qualify counters with
   success and failure cases before measurement; save the exact diagnostic patch.
   Dependency: step 1 and architecture review. Risk: medium (instrumentation bias).
3. Reuse the pinned 1 MiB seed42 fixture, private synchronized template clones,
   N=0/64/256/768 unrelated publications, raw/sparse and normal/tiny node budgets.
   Run two complete blocks for attribution only. Retain every observation, compare
   bytes after reopening, and report scan time as a subset of operation time.
   No performance gain is inferred from instrumented versus old timings.
   Dependency: helper qualification. Risk: low.
4. Record findings in `docs/network/cache-profile-results-2026-10-29.md` and compact
   sources/receipts under `docs/network/evidence/2026-10-29-cache-profile/`.
   Select a production candidate only if attribution and semantics support one.
   A cached quota index cannot silently stop detecting external file growth or
   additions. The failed SC-OPT-002 formatting candidate is not an accepted win.
   Dependency: complete raw receipts and independent review. Risk: low.

Any production optimization requires an additional committed, concrete candidate
description and paired acceptance gates before its implementation and measurement.
If the remaining cost is inherent to the current filesystem observation contract,
record that finding and proceed to the larger transfer experiment without weakening
quota, authorization, corruption, staging or durable-publication behavior.

## Phase 2: 16 MiB WAN qualification (SC-WAN-002)

1. Extend `crates/spiralcore-transfer/src/benchmark.rs`, its CLI and
   `tests/benchmark.rs` only as necessary for quota-safe shards of larger paired
   workloads. First test deterministic pairing, unique scopes, size/limit checks
   and bounded retained cache footprint. Preserve old suites. Dependency: plan
   and transport review. Risk: medium.
2. Update `.github/workflows/spiralcore-wan.yml` for the bounded suite and explicit
   build features; pin the exact source before dispatch. Use the existing free
   GitHub-runner-to-Mac Cloudflare relay path and synthetic data. A temporary token,
   source allowlist and run-scoped authorization are required. Dependency: tests,
   Rust/security/CI review and clean build. Risk: medium.
3. Qualify a local 16 MiB seed/update/reopen first, then run 30 paired trials per
   chosen workload in fresh-cache shards within existing 1 GiB global and 512 MiB
   namespace/resident limits. Include raw and sparse, with zstd where applicable.
   Register exact workloads, order and shard sizes before timing. No builds or
   other benchmarks during measurement. Dependency: local qualification.
   Risk: medium (external network variability).
4. Retain both-direction application bytes, captured encrypted runner-leg bytes
   with completeness checks, CPU, elapsed, separate seeding, reopen verification,
   failures, distributions and paired ratios. Elapsed includes CPU. Network-only
   time and energy remain unmeasured unless directly instrumented. Record raw's
   optional DAG preparation separately from sparse's readiness. Dependency: all
   receipts. Risk: low.
5. Publish the evidence/report and update `docs/network/optimization-ledger.md`.
   Remove only owned temporary credentials, servers, tunnels and redundant
   diagnostic logs after retaining scientific receipts and hashes. Keep existing
   local research files unchanged. Dependency: independent evidence review.
   Risk: low.

## Later size gate

64 MiB remains a protocol ceiling until a separate resource qualification and
bounded smoke run succeed. Do not raise default limits or call an unrun size
qualified. The first deliverable here is repeated 16 MiB sparse-edit WAN evidence.

## Success criteria

- Attribution identifies measured quota-scan CPU without changing its decisions.
- New active tooling and analysis use Rust; historical Python artifacts are preserved.
- Every successful transfer reopens as the exact original byte string and length.
- Bytes, CPU and wall time are reported together, including seed cost and negatives.
- Larger real-WAN evidence is repeatable and quota-safe; comparisons state the
  transport, readiness and packet-accounting boundaries.
