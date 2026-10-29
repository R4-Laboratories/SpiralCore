# 16 MiB paired WAN experiment

Registration state: NOT_RUN. This directory first retains the reviewed Rust
aggregation helper and source-review record before measurement. The experiment is
registered in [the plan](../../../plans/2026-10-29-cache-profile-and-large-wan.md).

The full campaign requires three disjoint `large16-localized` shards, thirty
global trials, and 180 successful raw-seed/update operations. Each shard must be
verified with the canonical `spiralcore-bench verify` command against its closed
receiver cache. The aggregate helper additionally checks complete pairing and
reports every metric vector in global-trial order, midpoint medians, nearest-rank
p95 and paired geometric means. It does not replace canonical fixture/reopen
verification or packet-capture completeness checks.

The local working packet is
`/Users/casey.allard/spiralcore/local/large-wan-20261029`. Secret capabilities are
outside this tracked packet and must not be copied here. The helper's absolute
identity manifest identifies the original local build; its source and lockfile
are retained here for reproduction. Compile and run the helper with Rust, passing
the campaign directory containing `shard-0/remote/rows.jsonl` through shard 2.

Raw receipts, verification, executable/source pins, capture diagnostics and the
observed result will be added after execution. Failed or incomplete runs are
retained and cannot be relabeled as successful performance qualification.
