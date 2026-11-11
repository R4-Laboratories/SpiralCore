# Original signer collection before host-agent idle exit

The fresh Linux host-process shutdown witness now passes: the actual production
agent became ready, collected its original signer child and then returned idle
exit **42**. The fixture reaped **zero** adopted children. The test completed in
**1.03 seconds**. This is a development checkpoint, not the completed MVM service.

The implementation is public at MVM fork
[`0aa190b32753b6862bed51a6c7a2d2a0b6b701e4`](https://github.com/Casey-allard/mvm/commit/0aa190b32753b6862bed51a6c7a2d2a0b6b701e4),
tree `98f7082d566c1577b2c05a51edd521b355d42290`. It retains the original service
map, mutation token/permit, connection tasks, watcher jobs and signer supervisor.
Preparation, helper/journal I/O and original task joins occur outside the outer
daemon mutex. Failed or canceled waits retain custody rather than manufacturing
a collection result.

## Executed outcomes

| Scope on the recorded runtime composition | Result |
|---|---|
| Actual daemon custody, map loans and signed replacement cancellation | 3 passed |
| Accepted-token reservation, resource lease and mutation-aware drain | 4 passed |
| Retained cell, deferred startup and cancellation-safe I/O | 11 outer cases passed |
| Shared connection budget and original service draining | 6 passed |
| Production agent/signer idle witness | 1 passed; 1.03 s |
| Strict release Clippy, all hostd targets | **Failed**, exit 101 |
| Four original restart consumers | **0 passed, 4 failed** at initial startup |

The 25 selected outer cases have zero failures or ignored cases. One Cell case
also runs in an isolated subprocess; that repeated invocation is not additional
distinct coverage. The [runtime receipt](runtime-qualification.json) records
the individual filters and executable hashes. Raw test outputs are included.

The actual idle output is:

```text
worker_code=Some(42) ready=1 original_collections=1
fixture_adopted_reaped=0 trace_bytes=833 trace_truncated=false
```

The retained earlier idle failure exited 42 without an original collection and
required fixture adoption/reaping. The corrected result uses the original
20-second worker and two-second cleanup guards; no deadline was enlarged.

The fixture supplied a registration-journal pathname; it did **not** seed a
physical empty journal. This source's `RegistrationJournal::load` treats a
missing file as empty. The successful shutdown does not qualify missing/unknown
inventory recovery or authorize that interpretation for nonfresh installations.
Those startup/recovery distinctions remain required work under
[#114](https://github.com/R4-Laboratories/SpiralCore/issues/114).

## Source identity and reproducibility

Runtime execution used a declared development composition with 4,488 regular
source-manifest entries, unchanged through both compilation and the tests.
The twelve selected product/fixture blobs exactly match the published commit.
This is **not** whole-commit runtime qualification.

[The complete projection](source-projection.json) records all six tracked-file
differences, including the older core mailbox API and crypto tests from public
commit `ee632fbb3a746709c496a95f167559a01a19fcaf`, a CLI test-only delta and
noncompiled provenance/notice differences. Eleven AppleDouble metadata files
were present; four predated the transfer, none were compiled as Rust, and none
were deleted. The full runtime manifest is retained losslessly in
[runtime-source.sha256.gz](runtime-source.sha256.gz). Current published imported
licenses remain intact; the historical manifest is an observation, not a
replacement for those notices.

The exact immutable published commit was separately exported and passed both
required gated compile commands, with all **4,478 regular source hashes
unchanged**. [Exact-commit receipt](exact-commit-gated.json). Just was unavailable;
the two commands from the retained gated recipe were executed directly:

```sh
./scripts/cargo.sh --direct cargo-zigbuild check --target x86_64-unknown-linux-gnu --workspace --all-targets
./scripts/cargo.sh check -p mvm-conformance --all-targets --features bdd
```

These are type checks. They retain the genuine unused-code warnings and do not
waive the failed strict lint gate or upgrade the runtime composition to exact
whole-commit execution. Both exact logs are included.

Rust 1.97.1, Linux x86_64 kernel `7.0.0-42-generic`, release/locked/offline,
two jobs, nice10, incremental disabled and umask022 were used for runtime work.
`MVM_EMBED=0` disabled embedded-host packaging for this process qualification.
The existing shared target was serialized. No Mac build or VM boot was used.

## Failed gates remain part of the result

The first integrated compiler attempt failed on one fixture `PathBuf` versus
`&Path` mismatch. The single borrow correction changed no product behavior or
test assertion. Original compiler JSON is preserved in deterministic gzip;
decompression reproduces the raw SHA-256 values in the runtime receipt.

Strict Clippy failed on unfinished recovery/startup wiring, unused custody and
legacy methods, a 984-byte error variant and one question-mark style correction.
No suppression was introduced. The original restart fixture did not provision
the new resource-owner namespace. All four tests timed out before registration
or crash/replay scenarios. Missing provisioning is a source-backed sufficient
readiness blocker, but the old fixture discarded earliest worker stderr, so it
is not asserted as the sole runtime cause. The [negative receipt](strict-restart-qualification.json)
and both raw logs preserve the failures, unchanged source and executable hashes.

Independent CODE/Rust and security reviews passed for the matching exercised
fresh-profile blobs. Their verdicts do not cover the unfinished recovery paths.
The `quality-gate` command is unavailable in this environment;
the independent reviews were performed directly, without claiming that command
ran.

## Remaining delivery gates

Strict qualification, canonical frontend provisioning, the four restart
consumers, stale-socket recovery, Preparing-state replay, authenticated recovery
of retained operations and the assurance `OnceLock::set` publication wait edge
remain open. Actual mailbox-worker activation/collection, native MVM guests,
cross-host WAN, the supported capacity envelope and security qualification are
separate requirements. Neither #111 nor #114 nor the parent #42 is closed.

No transfer workload was measured here. The raw GNU time counters have their
reported precision and command accounting boundary; they establish no sender/
receiver CPU speedup, simultaneous fleet memory bound, send-only time, physical
energy saving or fidelity/compression result. Existing transfer measurements
retain their original source identities.

[SHA-256 manifest](evidence.sha256) covers this compact packet. Raw compiler
JSON, source identities, positive results and failed gates are retained. MVM
source excerpts remain under its [Apache-2.0 license](MVM-LICENSE); SpiralCore's
original documentation retains its repository license.
