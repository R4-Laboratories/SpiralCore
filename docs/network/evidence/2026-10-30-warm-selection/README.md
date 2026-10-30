# SC-OPT-015 warm-object selection evidence

This packet records the additive `prepare_warm` selector and consuming client,
compared with the unchanged forced Sparse and prepared Whole zstd/Raw senders.
The registered [measurement contract](measurement-contract.md) remains unchanged;
its preimplementation status describes when registration occurred, not the final
execution status. See the [result report](../../warm-selection-results-2026-10-30.md)
for observed outcomes and limits.

- Design/plan: `a438fc7658a823a48fe2a83efb502e2e8c87d331`.
- Qualified implementation/local source: `389accd2be2f8913ff6995f0c79c8867577ec6c1`.
- WAN source: `8bf2d85f6ece94013e66f7df9b6e1d6cddc8c43c`, with identical compiled
  crate sources and workflow to the local implementation checkpoint.
- `qualification/`: actual RED/GREEN, all-target tests, Clippy/build, focused
  coverage and independent procedural/code/Rust/security/infra receipts.
- `local-qualification/`: separate-process 36-transfer qualification, stopped-store
  exact comparison, setup and complete paid lifecycle records.
- `wan/`: six frozen plans, receiver observations, downloaded sender receipts,
  exact stopped-store readback, summary and custody/cleanup receipts.

The WAN registration is 30 paired trials per size at 1 and 16 MiB, with three
arms, actual Raw base seeds and one 32-byte localized change per session.
Every selector session pays one actual schema fit and context preparation.
Evaluated costs and pruning bounds are different receipt fields. The sender's
selection/preparation is timed, and the immutable prepared payload is consumed
without repeating encoding or hashing.

The production Rust example is the reproducible analysis entry point:

```text
schema_fitting warmselect-plan RUN SHARD NEW_PLAN
schema_fitting warmselect-paired PLAN EXE URL RECEIVER_SHA NEW_JSONL
schema_fitting warmselect-verify PLAN STOPPED_CACHE NEW_JSONL
schema_fitting warmselect-summary PLAN0 RAW0 VERIFY0 ... PLAN5 RAW5 VERIFY5 NEW_JSON
```

Source, observation and receiver-binary environment variables are documented in
the contract. The pinned workflow records the exact commands, compiler, source
hashes, features and Linux executable. Output paths must be new. Verification
requires stopping the receivers and reopening their original scoped stores;
ACKs or digest receipts alone do not substitute for full byte comparison.

Bytes count request and response HTTP bodies. CPU is sender-process CPU plus
disjoint receiver-handler CPU, excluding receiver socket/logging and relay work.
Exchange wall time includes receiver work and waiting, so it is not pure wire
time. Completion runs from preparation through the durable verified ACK.
Actual fitting/context setup, seed and update are summed once per lifecycle.
Fixture generation, process startup, receipt hashes and final readback are
outside operation timing. No energy, full-path packet, shaped-WAN, MVM or
deployment-security qualification follows from these measurements.

Formal raw evidence, including failed qualification attempts, is retained for
review. Generated multi-GiB object stores are disposable only after exact
verification and evidence custody. Frozen executables remain outside Git and
in the hosted artifact under its retention period; small measurement receipts
and their checksums are committed here.

The first-attempt hosted run
[41144943338](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41144943338)
completed all 360 registered transfers. [Summary](wan/summary.json) reports
360 exact reopened objects, sixty actual schema fits and zero discarded
observations. [Custody](wan/custody.json) verifies 51 downloaded artifact members,
86 source hashes and six cross-platform plans. The
[final independent review](final-review.json) checks raw joins, distributions,
paid lifecycles and the report without rerunning the experiment.

[Cleanup](wan/cleanup.json) records twelve stopped owned processes, six removed
verified stores (3,060 MiB payload), and removal of the temporary capability,
one GitHub secret and nine GitHub variables. The small raw evidence and frozen
executables remain retained; other lab state was not touched. `SHA256SUMS`
covers the files in this packet, excluding the checksum index itself.
