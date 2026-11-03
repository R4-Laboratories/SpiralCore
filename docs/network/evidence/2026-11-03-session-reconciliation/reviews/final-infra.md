# Portable minimum: scoped INFRA confirmation

2026-11-03. **PASS — no blocking manifest/qualification-scope finding.** Reviewed
the portable manifest at source `26dc93a1` against base
`e984d2d64cdf0483d50d6d59319276c85e7df644`, supplementing
[final CODE/RUST source review](final-code-rust.md). No broader audit or runtime
qualification was repeated.

```text
ad75a86870f243c62fce46724c5717e2b8f8d86df0760f768c8fc399da3b3708  docs/platforms/mvm-mailbox-portable.json
```

The complete manifest diff is service_tcp.minimum_count 11 to 14. JSON parsing
passes; normalized comparison after applying just that expected change is
identical. The actual source contains four parent TCP tests, seven included idle
tests and three included fixed-deadline tests, totaling fourteen functions. This
is a test-function minimum, not a count of every matrix subcase or fourteen
distinct real TCP recovery experiments.

Workflows have no diff in this source range. The existing portable workflow
enumerates each manifest target on its declared native Rust host, enforces the
minimum, then requires actual passing count equal the listing with zero failed,
ignored, measured and filtered tests. Row OS/architecture and dependency-pin
checks remain intact. All seven rows, toolchain, other minima, Unix storage scope,
permissions, resources, timeouts and concurrency are unchanged. The larger target
must actually execute in future hosted evidence; source review is not a new CI
PASS or artifact-custody claim.

These seven rows remain native adapter/client/component and TCP-loopback evidence,
including scripted SocketIo deadline conformance. They do not qualify the new
live reconciliation consumer on Windows, Windows durable storage, native Windows
MVM, booted guest recovery, WAN or fleet capacity. Existing platform gaps remain
explicit. Root's reported release111/shared130/object8/default/Clippy PASS and
unchanged six product hashes are separate retained execution/source records.

Only this review file was written. Manifest diff whitespace checks passed. No
builds, tests, product/manifest/workflow edits or expanded audit were performed.
