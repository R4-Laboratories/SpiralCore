# SC-WAN-004: persistent compressed-schema WAN evidence

The registered comparison uses a GitHub-hosted x86-64 Linux sender, Cloudflare
HTTPS/QUIC relay, and the local Apple M1 receiver. It compares the old and new
prepared schema encoders and ordinary zstd, with the same receiver, authorized
HTTP protocol, durable publication policy, and synthetic input bytes.

See the [measurement contract](measurement-contract.md),
[design](../../../specs/2026-10-30-schema-wan-design.md), and
[implementation plan](../../../plans/2026-10-30-schema-wan.md).

## Source and qualification

Implementation commit: `6c4e5464bdd1f65b436603bf855be7aee80f2dce`.
Measured campaign source: `2fcfb0dae9524c073f6aab8982736fb4b9242173`, which adds
only the failed initial service-launch evidence. Product and harness source is
identical between these commits. The macOS receiver and verifier were frozen
before launch; the workflow builds its Linux senders on a fresh hosted runner.

The baseline is a source-derived comparison: only `src/schema.rs` is replaced
with the immutable SC-OPT-012 source fixture. All other source, compiler options,
dependencies, benchmark code, and zstd settings match the optimized sender.
The workflow preserves the substitution patch, hashes, restoration check and
both frozen Linux executables. It does not claim to rebuild the entire older
repository revision.

`qualification/` retains actual RED/GREEN results, initial and resolved build
failures, all-target and documentation test output, Clippy, release build,
compiler/platform, and focused production coverage. Coverage uses only current
test/CLI objects and newly emitted profiles; branch coverage was not collected.
The procedural/code review and independent split-author security receipts are
`final-review.json`, `security-server-review.json`, and
`security-harness-review.json`.

## Registered dose and verification

Three shards each contain ten global trials, both sizes, three sender arms,
600 targets and 40 schema seeds. The complete dose is 180 persistent sessions,
1,800 targets, 120 seeds, and 120 sender preparations. Each session sends four
matching targets, two lexical/order variants, two drift targets, and two unrelated
binary targets. Every target is initially absent in that session's private scope.
Small records and approximately 1 MiB whole-record batches retain their actual
lengths. All six arm permutations occur five times per size.

The remote runner initially records durable commit acknowledgments. After the
receivers stop, the local verifier regenerates all fixtures and checks every
byte and original length from the reopened stores. The strict summary requires
1,920 exact reopened objects, all receiver observation records, valid prepared
cache progression, and 660 matching before/after body-cost/mode pairs.

The first launcher returned after spawning background services, and its owning
execution tool subsequently cleaned up all six children. The hosted job was
cancelled before the transfer step. [All available failed-launch evidence](launch-failed-01/README.md)
is retained. A foreground supervisor then held the replacement services alive;
its new campaign uses a new run ID, capability, URLs, and stores.

## Reproduction and accounting

Use the pinned compiler and locked dependencies. Build `schema_fitting` and
`spiralcore-wan` in release mode; record the chosen target and feature tree. The
Mac uses `hardware-sha256`; the Linux sender uses default features. Preserve
the immutable schema artifact and the registered source hashes. The exact
hosted build and commands are in
[the workflow](../../../../.github/workflows/spiralcore-schema-wan.yml).
Do not run the hosted workflow with arbitrary endpoints or reuse a populated
receiver cache.

```text
SPIRALCORE_SCHEMA_SOURCE=<source> schema_fitting wan-plan RUN SHARD NEW_PLAN
SPIRALCORE_WAN_TOKEN=<private capability> spiralcore-wan serve --plan PLAN --cache NEW_CACHE --seconds 3600
SPIRALCORE_WAN_TOKEN=<private capability> schema_fitting wan-paired PLAN BASELINE CANDIDATE HTTPS_URL RECEIVER_SHA NEW_ROWS
# Stop the receiver before independent readback:
SPIRALCORE_SCHEMA_OBSERVATIONS=RECEIVER_JSONL schema_fitting wan-verify PLAN CACHE NEW_VERIFICATION
schema_fitting wan-summary PLAN0 ROWS0 VERIFY0 PLAN1 ROWS1 VERIFY1 PLAN2 ROWS2 VERIFY2 NEW_SUMMARY
```

Transfer bytes are actual request plus response HTTP bodies, including protocol
framing and acknowledgments. They exclude HTTP/TLS/IP/TCP overhead and extra
relay hops. CPU is sender process time plus receiver handler time; receiver
socket/HTTP handling, tunnel and relay CPU are outside that subtotal. Durable
elapsed already includes computation and waiting. Exchange elapsed is nested
within it and includes receiver work; it is not a network-only time estimate.
Actual schema seeding and preparation are charged in ten-target session totals.
Fixture generation, process startup, evidence writing and independent readback
are outside the operation timer. No power or energy measurement is made.

Keep compact JSON, receipts and reports in Git. Large build outputs remain in
the owned local build cache and hosted artifacts. Remove only this campaign's
verified object stores and temporary credentials/services after retaining the
evidence; preserve user runtimes, saved cards and other labs' files.
