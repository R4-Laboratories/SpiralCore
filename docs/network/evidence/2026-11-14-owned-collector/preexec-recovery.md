# Native recovery after mailbox helper execution is refused

The pre-helper failure that previously left a Firecracker workload running now
has a positive original-owner recovery witness. Native trial 13 deliberately
supplied a private mailbox-helper executable with mode 0775. Canonical trusted
execution refused it before exec. The SDK stopped through the exact retained
backend and anchor, obtained the actual native/collector retirement receipt,
and published the original collector's retained state through its parent.

The test deliberately made the first parent callback return an error before
publication. It then retried through the same SDK failure object and successfully
completed the real parent callback. The resulting original destination row is:

```json
{
  "name": "sc-journal-b",
  "helper_not_executed": true,
  "native_collected": true,
  "collector_parent_published": true,
  "no_helper_callback_retry_observed": true,
  "runtime_state_removed": false,
  "definitions_removed": false
}
```

This is a one-guest failed-start recovery result. No mailbox helper exit status
is fabricated. The source guest was never attempted; the complete pair command
therefore still exited 1 and retained that error. No application transfer or
fidelity/performance result follows from this fault test. The current signed
runtime compatibility gate remains [#268](https://github.com/R4-Laboratories/SpiralCore/issues/268).

| Observed scope | Result |
| --- | --- |
| Contributor MVM | `6d8b3831f31d3215055a0ef9bebe055689affada` |
| SpiralCore witness | `24fd4e52` |
| Exact source comparison | 4670 files, 64,797,159 bytes, zero mismatches |
| Native backend | Linux x86-64 / Firecracker 1.17.0; one destination guest started |
| Controller lifecycle wall | 9.764451287 s |
| Controller process CPU | 8.626887167 s |
| Controller GNU time peak RSS | 341524 KiB; maximum process RSS, not aggregate RAM |
| Whole test-unit memory peak | 655364096 bytes, including preparation/children/cache charge |
| Whole test-unit CPU | 18.998420000 s, including setup/install/controller/children |
| Application transfer | Not attempted |

The controller time includes the failed startup and recovery, not a transfer.
The whole-unit observation also includes canonical kernel preparation and
installation. Original records, wrapped keys, definitions and startup debt
remain protected. The after-stop managed snapshot still conservatively holds
reservations while the two-role witness is incomplete; those counts are not
observed live processes, and this is not a zero-reservation installation
shutdown result.

## Implemented boundary

The canonical creation boundary issues a private no-exec proof only after it
holds the original launch exclusion and boot/directory identities. Entering
spawn fences the attempt as uncertain. Only an actual spawn refusal restores
no-exec status; a returned child must be placed in its original owner before
`ChildOwned` is reported. Panic stays uncertain.

`finish_without_helper` requires that proof and retirement from the exact
original backend/anchor. It retains the retirement receipt through callback
failure, and completes only after the original parent publishes telemetry and
accepts pending retirement. It neither invokes generic reaping nor fabricates a
normal journal-stop result. Helper-present failures keep the existing original
`JournalOwner` path; uncertain helper creation cannot use no-helper recovery.

## Checks and limits

The host filter passed 35 checks, including the three new creation-boundary
controls, with three existing ignored helper fixtures. The journal SDK passed
13 checks. The witness passed 23 checks. Strict all-target Clippy passed for
both changed MVM crates; strict selected-binary Clippy passed for the three
witness tools with all feature paths compiled. Independent scoped source review
passed for the core and adapter. Initial error-type and invalid test-key
failures remain recorded.

These results do not complete uncertain-creation reconciliation, crash/restart,
independent-client or detached-owner recovery, multi-tenant scale, remaining
platforms, WAN or security/FIPS qualification. [Issue #111](https://github.com/R4-Laboratories/SpiralCore/issues/111)
remains open. [Compact results, source and binary identities](preexec-recovery.json)
retain the exact failed pair command and successful destination recovery row.
