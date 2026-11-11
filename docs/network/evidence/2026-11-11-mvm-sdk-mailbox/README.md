# Persistent SDK mailbox grant regression — 2026-11-11

[MVM PR #5336](https://github.com/tinylabscom/mvm/pull/5336) fixes the canonical persistent launch path dropping a request's mailbox grant. This is a narrow SDK prerequisite, not the completed MVM networking deliverable. The contribution commit is [`18b58e1ad2262a1c0bfa23d51f8e5679d3376f60`](https://github.com/Casey-allard/mvm/commit/18b58e1ad2262a1c0bfa23d51f8e5679d3376f60), based on upstream `28afdfe9ef7e36518ca42788d57c54f1d7275f57`.

## Failure and change

The current canonical native witness's sixth attempt successfully launched a persistent Linux/Firecracker machine, but found no mailbox helper PID before any receive probe. Source inspection found `boot_spec` constructing `MachineStartParams` with `mailbox: None`. The public-API regression then reproduced the defect with the hermetic Mock backend: a mailbox request booted successfully instead of reaching the backend's canonical unsupported-mailbox refusal.

The fix forwards `request.mailbox` through the private boot helper into canonical admission for that start. Ordinary persistent start passes `None`; no mailbox field is added to `MachineSpec`. It preserves canonical authorization/reminting, helper ownership and failed-launch rollback. No plan file is fabricated and no helper is manually launched.

## Executed qualification

Linux x86_64 on Wick, Rust 1.97.1, release/locked, `test-support`, two build jobs, no incremental compilation, serial tests and umask 022:

| Scope | Result |
|---|---|
| New public request test before fix | Exactly one executed; fails because launch wrongly succeeds without mailbox |
| No-inheritance control before fix | Exactly one executed; passes |
| Persistent-related tests after fix | 31 passed, zero failed/ignored |
| Full launch module after fix | 55 passed, zero failed/ignored |
| Strict library and test Clippy | Both pass |
| Package formatting check | Pass |
| Independent code/Rust and security source reviews | Pass on the final two file hashes |

The 31 and 55 test scopes overlap; do not add them. Only the two listed source files differ from the executed base; the other 4,436 regular source files were unchanged. [qualification.json](qualification.json) records exact commands, source hashes, original output hashes and scope. The raw RED output contains synthetic launch-context debugging and stays private; its hash is retained, with a compact reason here. This compact record is intentionally not a full stdout archive.

## Native failure boundary

[native-006.json](native-006.json) preserves the allowlisted original result fields, resource-check outcomes and original report hashes. Private paths and process identifiers are excluded.

Native attempt 006 used the earlier `ee632fbb` API source with its rebuilt guest runtime, helper and producer, one CPU/256 MiB, no guest NIC and TTL 180. Launch setup took 715.498 ms; failed lifecycle took 766.312 ms; orchestrator CPU was 469.470 ms. Those are failed setup/lifecycle measurements, not transfer performance. Zero receive probes ran; no exact readback or transmitted-byte result was established. Canonical Stop and non-force removal succeeded; original VMM and owned VM/machine namespaces were absent afterward. The affected SDK launch file was unchanged between that executed native source and the later regression base.

The fixed SDK has not yet been rerun through the complete native witness. Native transfer, bytes, sender/receiver CPU, WAN, durable service, capacity and platform acceptance remain open. Current upstream `46d50a5f` adds containment/blog changes outside these two launch files; it is not retroactively substituted as the executed source.
