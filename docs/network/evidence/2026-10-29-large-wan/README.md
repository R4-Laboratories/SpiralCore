# 16 MiB paired WAN experiment

Completed at source `f4e5e9afe6f3628ffe82e4553a88111f5bd6d126` after the
[registered plan](../../../plans/2026-10-29-cache-profile-and-large-wan.md).
All 180 seed/update operations reopened exactly, totaling 3,019,898,880 compared
bytes. All thirty update pairs favor sparse on elapsed and combined endpoint CPU.
See [the result and limitations](../../large-wan-results-2026-10-29.md).

The campaign has three disjoint `large16-localized` shards, thirty global trials,
and 180 raw-seed/update operations. Each shard was verified with the canonical
`spiralcore-bench verify` command against its closed receiver cache. The aggregate
helper additionally checks complete pairing and
reports every metric vector in global-trial order, midpoint medians, nearest-rank
p95 and paired geometric means. It does not replace canonical fixture/reopen
verification or packet-capture completeness checks.

The local working packet is
`/Users/casey.allard/spiralcore/local/large-wan-20261029`. Temporary secret capabilities
were removed and are never included here. The helper's absolute
identity manifest identifies the original local build; its source and lockfile
are retained here for reproduction. Compile and run the helper with Rust, passing
the campaign directory containing `shard-0/remote/rows.jsonl` through shard 2.

## Contents and reproducibility

- `shard-0/` through `shard-2/`: original remote canonical plans, raw JSONL rows,
  Rust summaries, capture analyses, drop/filter counters, route/offload records,
  producer identities, local exact-verification receipts and before/after process
  snapshots. Raw outputs retain original whitespace.
- `aggregate.json`: all thirty paired measurements, each integer vector, midpoint
  medians, nearest-rank p95, and raw/sparse and zstd/sparse paired ratios. Combined
  CPU and seed-plus-update costs are formed per operation/trial before statistics.
- `analysis/`: frozen Rust aggregator source/lock, binary identity and three
  qualified tests, including oversized-input and malformed/missing-row negatives.
- `independent-source-review.json`: the original premeasurement architecture,
  security and analysis-source review, preserved byte-for-byte from its initial
  name in source commit `f4e5e9a`; distinct from later numerical review outputs.
- `independent-review*`, `independent-pcap-review*` and the review method:
  separately authored Rust reconstruction of fixture identities, scopes, order,
  statistics, producer/reopen provenance and campaign packet totals. The final
  receipt distinguishes the qualified twenty-pair IP subset from the incomplete
  thirty-pair IP campaign. Independent PCAP replay does not independently prove
  per-row TCP connection ownership.
- `local-check/`, `qualification.json`, `receiver-qualification.json`,
  `coverage.json`: the full sixty-operation local HTTP/reopen qualification,
  targeted RED/GREEN tests, 128 routine hardware tests plus doctest, build/lint
  results and production-source coverage. Setup failures are recorded separately.
- `cleanup.json` and `diagnostic-cleanup-identities.sha256`: shutdown, credential
  removal and historical hashes of removed console diagnostics. Those removed
  paths are not part of the retained-file verification manifest.

Reproduce aggregation from this packet with:

```sh
cargo run --release --locked --manifest-path analysis/Cargo.toml -- .
```

Run that command from this directory. The independent checkers' exact compiler,
dependency, invocation and negative-test commands are in their method receipt.
To repeat the network experiment, use the [WAN runbook](../../wan.md) with a new
campaign ID, capabilities and fresh caches; reuse the source revision, fixed
three-shard schedule and recorded comparison policy. Prior URLs are expired.

## Packet boundary and retained large files

Shards 0 and 1 qualify the runner-to-Cloudflare IP leg. Shard 2 contains 622,806
saved packets versus 622,809 reported received-by-filter, with zero reported kernel
drops and no unassigned or ambiguous flows. This unresolved three-packet difference
keeps `capture_complete=false`. All thirty body/time/CPU pairs remain in the result;
no complete thirty-pair IP reduction is asserted, and the shard was not replaced.

The original 96-byte-header PCAPs are retained locally, outside this compact Git
packet, and identified by each shard's `remote/capture-sha256.txt` and
`remote/local-pcap.sha256`:

| Local path under the working packet | File size |
|---|---:|
| `shard-0/remote/headers.pcap` | 69,379,089 bytes |
| `shard-1/remote/headers.pcap` | 69,278,365 bytes |
| `shard-2/remote/headers.pcap` | 69,340,557 bytes |

The full receiver caches and frozen binaries are also retained locally. GitHub
run artifacts provide the original captures during their one-day retention; the
committed compact evidence remains available afterward, but a new reviewer needs
the locally retained captures to independently replay the original packet bytes.
IP counts use header-declared packet length, not the 96-byte saved prefix.

Run `shasum -a 256 -c SHA256SUMS` from this directory to verify retained compact
files. Absolute build and cleanup identity manifests preserve their original
location/history and are not substitutes for this packet-relative manifest.
