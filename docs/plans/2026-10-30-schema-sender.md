# SC-OPT-013 implementation plan

1. **Measure before production edits** — extend the existing Rust
   `examples/schema_fitting.rs` harness/support with bounded stage profiling and
   a per-session command that reuses the tested persistent receiver. Freeze the
   same harness in a baseline binary, recording source/executable/fixture hashes.
   Risk: low; helper tests cover identity, strict output creation and accounting.
2. **Optimize one encoder path** — after the profile, add behavioral/parity
   tests and retain their actual RED evidence before changing `src/schema.rs`.
   Implement exact first-template matching with bounded borrowed token storage
   and the existing general fallback. Risk: medium; raw stream and compressed
   frame parity with the pinned baseline, plus malformed lexical cases, gate it.
3. **Qualify once** — focused tests, the full Rust all-target suite, Clippy and
   changed-code coverage. Reuse the shared target and explicit native triple;
   builds/tests never overlap timing. Independent source/security review is
   limited to this change and its harness.
4. **Measure matched persistent sessions** — freeze the candidate binary with
   the identical harness. Interleave baseline/candidate prepared-schema and
   ordinary-zstd sessions on the same ten distinct targets, ten paired trials
   at small and ~1 MiB sizes. Count actual seed/preparation, both directions,
   sender/receiver/total CPU, durable elapsed and exact immediate/reopened bytes.
5. **Record and deliver** — retain all observations, failures, setup and negative
   controls; perform one compact results-consistency check. Update the ledger
   and delivery notes, publish the reviewed branch through a PR to origin/main,
   and fast-forward the local checkouts without touching user originals.

Files and ownership: codec owner `src/schema.rs`, a focused new test target and
its pinned baseline oracle; measurement owner the existing example/support and
new SC-OPT-013 evidence/report; parent qualification, integration, ledger and
publication. No protocol/receiver changes, new dependencies or new Python code.
