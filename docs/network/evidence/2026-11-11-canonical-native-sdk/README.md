# Canonical mailbox SDK native diagnostic — 2026-11-11

**Exact delivery passes; the complete lifecycle gate fails.** One real Linux x86_64/Firecracker guest delivered all 1,024 synthetic binary bytes through MVM's canonical mailbox after the persistent SDK grant fix in [MVM #5336](https://github.com/tinylabscom/mvm/pull/5336). The original helper remained a zombie after canonical Stop/remove, so this is not a successful installed-service retirement or a completed MVM deliverable.

## Source and execution

MVM source is `18b58e1ad2262a1c0bfa23d51f8e5679d3376f60`, based on upstream `28afdfe9` with only the two-file persistent grant fix. All 4,438 regular source files were verified. Host SDK, canonical helper, guest producer and rebuilt guest runtime use that same revision. The public SpiralCore witness checkpoint is [`f1dba876`](https://github.com/R4-Laboratories/SpiralCore/tree/f1dba8763cb81bdc3f8e000a091394de9cd12b7a/tools/mvm-canonical-witness), with 18 distinct focused tests, strict host/guest Clippy, locked builds and independent code/Rust/security source review. Its component tests are not native-transfer tests.

The single guest had one vCPU, configured 256 MiB RAM, no NIC, a 180-second TTL and no idle pool. The original 30-second readiness and 60-second receive limits were preserved. The private source fixture used explicit guest-content directory permissions; this does not fix canonical injection under restrictive umask. The normal signed-plan grant, port 5257 helper and audited SDK receive were used. No plan was fabricated and no helper was manually spawned.

## Observed result — one local case

| Measurement | Result | Scope |
|---|---:|---|
| Expected / independently received payload length | 1,024 / 1,024 bytes | Direct byte equality, including embedded zeros |
| Expected / received SHA-256 | `3cafacf7f3f4628dcc4da6be8a679c3d283911100632476924fc76fa0e2a20ad` | Equal exact synthetic payloads |
| Setup wall time | 658.948 ms | Canonical preparation/materialization and launch for this case |
| Receive-phase wall time | 104.022 ms | Two external probes; includes the deliberate 100 ms probe schedule |
| Full attempted lifecycle | 2,821.558 ms | Launch through receive, Stop/remove and failed collection check |
| Host-orchestrator process CPU | 475.934 ms | Excludes VMM, helper, guest and earlier source-build children |
| Guest CPU / actual wire bytes | Unavailable | Not inferred from payload length or process ticks |

This is a functional diagnostic, not a throughput benchmark. It has no zstd control, no SpiralCore object-v1 compression, no warm-cache comparison and no WAN hop. The receive time is not send-only time. Configured VM RAM is not measured peak RSS. Process tick samples in the JSON are partial/coarse and are not complete guest or helper CPU.

## Retained teardown failure

Canonical Stop and non-force Remove returned success. The original VMM was observed exited, but the held original helper identity had an exit event and remained `Z`; its status was not collected. The overall report therefore remains `failed` despite exact delivery. [Issue #111](https://github.com/R4-Laboratories/SpiralCore/issues/111) tracks original child custody, startup rollback, bounded long-lived ownership and collection before resource/state release. The [later resource snapshot](resource-check.json) found both processes absent and the VM/machine/idle namespaces empty, with the witness claim retained. That later absence does not replace the failed synchronous receipt. No external waitpid, kill or repair was used.

The witness guards Stop/remove with the retained successful LaunchOutcome and original definition identity. Those guards and process observations do not supply the missing long-lived native backend/helper owner. This does not qualify orphan recovery, durable admission, sustained sessions, capacity, other native backends, Windows, WAN or FIPS/CUI deployment.

## Reproduction records and one-time preparation

[native-007.json](native-007.json) retains original result/timing fields and the hash of the private original report; private paths, numeric process identities and plan ID are excluded. [artifacts-and-preparation.json](artifacts-and-preparation.json) records source, binaries, kernel, runtime archive/assembled hashes, configured limits and preparation scope. Original private reports, source identity and diagnostic console captures remain on Wick. Only compact records are stored here.

Canonical source-runtime preparation took **801.292 seconds wall time**. GNU time reported **1,487.97 seconds user + 57.21 seconds system CPU**, including children; parent-process CPU was separately **2.363 seconds** and overlaps that inclusive total. Maximum single-process RSS was 2,722,344 KiB, not aggregate peak memory. This one-time compiler/setup cost is separate from the native timing table and is not a transfer cost or energy measurement. Earlier EE runtime artifacts were not relabeled as this source.
