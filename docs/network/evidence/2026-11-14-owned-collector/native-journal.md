# Installed journal: original-owner native retirement

The protected-telemetry shutdown failure now has a positive two-guest Linux /
Firecracker retirement witness. Native trial 11 started both installed guests,
passed both original-owner readiness checks, acquired both required mailbox
meters, then collected both original mailbox helpers unassisted with exit 0.
Both original collector endpoints retired and their retained telemetry was
published through the shared parent. No forced cleanup or protected-file
removal substituted for original-owner collection.

**The pair transfer is still blocked by guest-runtime compatibility.** Trial 12
at contributor MVM `61f72716470c706c7ba90837d9f0c5b72e8f26c9` fixed the missing
pre-admission owner binding and dispatched the original source entrypoint. The
source workload exited 1. Its console reports that the verified signed guest
runtime refused the mailbox boot record as oversized: the source record is
2553 bytes; that runtime's exact source `ffd90f895d7080629eceb16095d1ca81c0a49b95`
limits the file to 2048 bytes. Current host code accepts up to 8192 bytes. The
source helper meter recorded a failed write and no accepted application bytes.
The tool did not retain the workload's stderr in its terminal error, so that
console refusal is recorded separately from the workload exit observation.

Both guests again passed readiness and retired both original mailbox helpers
unassisted with exit 0; both collectors retired through the shared parent.
Trial 12 lifecycle wall time was 20.013807690 s; controller CPU was
17.379395010 s. These include startup, failed dispatch and shutdown, not a
successful transfer. The final caller result has no teardown error.

The correct successor needs a compatible, canonically verified runtime. The
latest upstream release workflow inspected was the rc.3 run
[43486823143](https://github.com/tinylabscom/mvm/actions/runs/43486823143), whose
signed candidate is the older runtime used here; newer source-build releases
inspected contain host helpers, not a replacement signed guest runtime.
Compatibility qualification is tracked in [#268](https://github.com/R4-Laboratories/SpiralCore/issues/268).
No signature check, boot bound or original-owner check is bypassed to obtain a
positive row. This result closes the observed retirement behavior, not the
complete mailbox service or upstream submission.

| Trial 11 observation | Result |
| --- | --- |
| Contributor MVM source | `dad9fecc5d19bed3d0312f656ea4912fc2607d32` |
| SpiralCore witness source | `6c793c93` |
| Backend | Linux x86-64, Firecracker 1.17.0, two actual guests, no guest NIC |
| Guest request | 1 CPU / 256 MiB each |
| Complete controller lifecycle wall time | 21.431524182 s |
| Controller process CPU | 17.738456701 s |
| Application transfer | Not executed; entrypoint admission refused |
| Original helper exits | A: unassisted 0; B: unassisted 0 |
| Original collector retirement | Both succeeded through original owners |
| Retained resource snapshot | 0 guests, 0 workers; storage/workspace credits retained |

CPU is the Rust controller process, including its in-process SDK work. It does
not include all helper/guest CPU and is not a transfer speed. The wall time
includes startup, failed dispatch and retirement. No current WAN or zstd
comparison follows from it. The existing signed guest runtime and application
fixtures retain their earlier source identities; this does not qualify newly
built guest code or a release combining every development component.

The explicit witness envelope is 3 GiB host resident, 1 GiB startup address
space, a private finite 1 GiB disk-backed filesystem, and 2400 MiB shared staging
credit. Two conservative 1 GiB startup reservations coexist; this does not
allocate two physical filesystems. Original encrypted records, wrapped keys,
definitions and retained charges remain protected after process retirement.

The retained failed trials establish the corrections, rather than being
replaced by the positive shutdown row:

- Trials 1–3: missing verified kernel, then insufficient preparation memory.
- Trials 4–5: wrong default verbs and persisted workload streams refused by
  canonical registered admission; the owned path now selects its exact admitted
  verbs and ephemeral workload streams.
- Trial 6: group-writable helper refused; an orphan required assisted containment.
  This remains failed recovery evidence. Pre-helper launch recovery is still open.
- Trials 7–8: destination rollback retired its original helper; readiness failed.
  Trial 8 exposed `exec_secs = 0` incorrectly treated as an expired deadline.
- Trial 9: destination readiness passed; the helper lacked its required
  `durable-guest-io-witness` measurement feature. Missing meters remain refusal.
- Trial 10: destination readiness and measurement passed; source preparation
  refused ENOSPC on the original finite 512 MiB filesystem. The larger successor
  is explicitly funded and remains finite.

The owner-admission successor captures the actual anchor before canonical
admission and revalidates the exact owner, directed mailbox and signed deny-all
policy at final gates. Five new host-admission controls and 13 journal SDK checks
passed, with strict all-target Clippy for both affected crates. A foreign Arc,
missing owner, stale lease, absent direction admission or changed signed policy
still refuses. [Owner checks and source identity](owner-admission-checks.json).

The deadline correction keeps caller and signed-plan expiry bounds, and leaves
ordinary one-shot semantics unchanged. Twelve driver tests and seven installed
entrypoint/deadline tests passed; one existing driver helper fixture was ignored.
Strict all-target Clippy passed for the four changed MVM crates. The paired
witness, producer and installer passed 23 + 6 + 5 tests and strict selected-binary
Clippy. These are component checks, separate from native evidence.

[Compact trial results and binary identities](native-successor.json) ·
[Deadline checks and exact source verification](journal-control.json) ·
[Reproduction requirements](../../../../tools/ari-own-mailbox-witness/installed-collector.md)
