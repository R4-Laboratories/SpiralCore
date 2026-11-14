# Paired mailbox harness on the current MVM SDK

The paired native qualification harness now builds against contributor MVM `468661f47e01d0f59c6f1da4e35a968ceccb3ad2`. SpiralCore source `f76923fa` makes the basic journal settlement selection explicit and preserves the existing admitted ordinal selection. This fixes a witness compatibility error; it does not introduce another protocol or change canonical admission.

## Failures and corrections

The copied historical lock did not match the current local MVM graph. The original locked offline build refused to update it. Cargo's normal offline resolution preserved all **425 existing registry name/version/checksum rows**, updated 20 local MVM package versions from rc.2 to rc.3, and added the eleven registry dependencies and four exact-pinned SpiralCore packages required by the current graph. No blanket dependency update was used. The original lock SHA-256 is `1b7de9074ec9247f2bddf43a45a99b9226e4f89e4e1362b4725024c761014883`; the resolved lock is `0c77d9494d38c2c8f646be5ab3192733c5d7e7d2cffc271aa756efb03e3ecef2`.

Unfiltered offline metadata then refused an uncached Android-only dependency. Full metadata filtered for the actual Linux target passed with the resolved lock unchanged. This establishes Linux resolution, not all-platform cache availability. A subsequent compilation reached the actual `E0063` failure: the canonical journal-direction struct always has `sender_settlement`, while the old harness initialized it only behind its ordinal feature.

The correction selects `None` for the basic arbitrary-message-ID mode and preserves `Some(OrdinalV1)` for explicitly selected ordinal campaigns. It does not disable sender settlement in ordinal mode. Compilation then passed. Strict paired-target Clippy subsequently exposed an unused four-field counter wrapper. The legacy caller now uses the existing full parser directly, preserving its output; the paired meter still retains both error counters. Tests assert the full tuple, including the existing nonzero read/write-error case. No lint suppression, duplicate parsing, frame, recorder bound, deadline or lifecycle-owner change was introduced.

## Final executed qualification

All commands used Rust 1.97.1, the locked offline graph and the shared Linux target. The source verifier matched **38 host-input entries** against the published SpiralCore source; this is not a claim that every file in both repositories was requalified by this harness run.

| Target | Actual result | Complete command wall |
| --- | --- | ---: |
| Basic paired, image and preparation executables | Build exit 0 | 9.32 s |
| Basic paired component tests | 21 passed, 0 failed or ignored | 8.91 s |
| Ordinal paired component tests | 22 passed, 0 failed or ignored | 2.12 s |
| Legacy own-mailbox meter tests | 5 passed, 0 failed or ignored | 0.74 s |
| All-features paired target check | Exit 0 | 0.73 s |
| All-features paired target Clippy, `-D warnings` | Exit 0 | 0.62 s |
| Enclosing bounded runner | Exit 0 | 22.482 s service runtime |

Commands were `cargo +1.97.1 build --locked --offline --features paired-journal-qualification --bin paired-journal --bin image --bin prepare`, the three corresponding `cargo test` invocations with `--test-threads=1`, and `cargo check` / `cargo clippy` with `--all-features --bin paired-journal`. Clippy included `-- -D warnings`. Cargo JSON output was enabled for the build to retain executable provenance. The [machine-readable receipt](qualification.json) pins every stage's log, stderr and time record.

These are per-target reported counts, with overlapping shared parser tests. The 0.24 s / 0.25 s / 0.00 s test-runner times exclude compilation; the table uses complete command time. The service's total CPU was 28.494 s and peak memory 1.3 GiB as reported by systemd. Resource limits were two CPU cores, 6 GiB memory and 1,500 seconds. These are qualification costs, not application transfer timings or energy measurements.

Three pre-existing Cargo unused-patch warnings remain. Basic-profile normal compilation also reports three pre-existing unused items in feature-specific research helpers. The strict success is specifically the **all-features paired target**; it does not establish warning-free compilation of every feature combination or every binary.

## Custody and remaining work

Original lock, metadata failures, missing-profile compilation and strict-Clippy failure are retained separately from the final successful records in private Linux directory `paired-0941-468661.ra3f9xwc`. Formal logs stay there; only compact source-bound results are committed. Current-source component/code/Rust/security review is separate from native runtime acceptance.

The next step is preparing current-source role images and running the original two-MVM mailbox route, including lost-receipt recovery. This qualification launched no guest and supplies no WAN, sustained-session, compression-speedup, production, fleet-capacity or FIPS claim. The [delivery issue](https://github.com/R4-Laboratories/SpiralCore/issues/42) remains open. The separately completed [installed one-guest FlowMux witness](../2026-11-13-owned-unix-store/native-rc3-fixture.md) retains its own executable and timing identities.
