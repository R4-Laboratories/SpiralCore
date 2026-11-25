# Original-instance lifecycle checkpoint

The fork development commit
[`f8a1c1cd9`](https://github.com/Casey-allard/mvm/commit/f8a1c1cd9d63ec1ec3e2915b9dcde793bcb67bd4)
integrates canonical [MVM #5658](https://github.com/tinylabscom/mvm/pull/5658) at `e51ddb646`
into the previously qualified `853bfeac` development source. The tested tree is
`639e367902a5f9dd266ccc67a882b1c66db85aa2`. The installed consumer selection remains MVM
`f2ddbcbf` and shared core `8994951b`.

## Lifecycle composition

- Named launch and recovery hold the canonical per-machine lifecycle lock before acquiring the
  original paired-mailbox reservation. Confirmed rollback reuses the held guard. Initial launch can
  proceed when the runtime namespace has never been created.
- Automatic HVF recovery checks the expected instance before effects. If the name refers to an
  installed original-owner slot, automatic stop refuses before invoking its stop path or attaching
  to the driver. Installed retirement remains bound to the original native handle.
- Mailbox startup retains both the startup cause and the result of stopping the original VM.
  An unresolved stop retains the persistent volume preparation, definition and state and returns
  recovery-required. The transient caller also retains runtime state. Confirmed rollback can reap
  ordinary stopped state after failure recording. The public mailbox startup signature is preserved.

Independent lifecycle and storage review found two guard-related defects. The first caused 23 named-launch
test failures, including on a fresh home. The correction produced 74 passing focused tests. The second
could leave stopped runtime behind because cleanup tried to reacquire the caller's lock. The final
reviewed correction passes the original guard through cleanup and adds regressions for guarded reaping,
both original stop outcomes and recovery-required through the outer definition rollback. Compiler
failures from a missing import and an incorrect result alias remain in the development records.

## Executed evidence

Linux x86-64, Rust 1.97.1, locked dependency graph, two build jobs, incremental compilation disabled.
The final Cargo build selected four unit-test binaries from the candidate source root. Three were fresh.
The unchanged runtime ELF matched its recorded SHA-256 before reuse. The path inventory and executable
bits matched the staged Git index. The verified manifest of all 4,845 file contents has SHA-256
`c349e637a331450de4559cb3a31affffca4342436c0c1efcb9227bb228a62391`.

| Component scope | Executed | Failed |
|---|---:|---:|
| Runtime instance recovery, installed-slot stop refusal and warm rollback | 4 | 0 |
| Hostd original paired reaping and mailbox stop outcomes | 2 | 0 |
| Client named launch, retained journal, first launch, guarded rollback and definition rollback | 71 | 0 |
| CLI machine lifecycle | 6 | 0 |
| **Total across 12 scopes** | **83** | **0** |

Cargo reported 53.32 s for the test build and 39.72 s for strict all-target Clippy on `mvm-client`,
`mvm-hostd`, `mvm-runtime`, and `mvm-cli` with test support. Complete command wall times were
53.36 s and 39.98 s, respectively. Formatting and staged
whitespace checks passed. The serial test processes summed to 2.31 s wall, 1.04 s user CPU and
0.16 s system CPU. The highest process RSS was 64,604 KiB. These are local component costs.
End-to-end transfer time, network traffic bytes and sender/receiver transfer CPU were unmeasured.

All actual ELF hashes were checked before and after execution. The four binaries and their individually
verified archive members remain on the HDD. `sc-lifecycle-resume-final6-elfs.tar.zst` is 126,986,390 bytes
with SHA-256 `19ee3e68b1dd450dbdc0557c87d2b4fffc83cb962a119cb0287bf8a6eb2d6222`.
Raw runner, toolchain, selector, exit, timing and source receipts stay with the HDD evidence.

## Canonical path and open acceptance

Canonical main was observed at
[`4be3c88a`](https://github.com/tinylabscom/mvm/commit/4be3c88a8332f07a0ffdb926af5c6a161e4a1455).
[MVM #5668](https://github.com/tinylabscom/mvm/pull/5668) has since merged. Its signed intent
and durable receipt recovery changes are outside this development tree. Ari confirmed that
coordinator observation uses the existing detached coordinator directly and requires no systemd
or D-Bus service. [Coordinator decision](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-7056911902).

The audit-interface request remains unanswered. The original installed readiness refusal
`FlowMux requires a bound audit sink` is retained from the previous full-workspace run. This
checkpoint did not rerun that Linux address-space-enforced integration case, the full workspace
or native macOS/Windows. The rc.11 candidate failed documented Linux/Firecracker and macOS/HVF
lanes. Release verification, publication and installed promotion were skipped. A compatible
published signed guest runtime was unavailable. [Audit request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842).
[rc.11 run](https://github.com/tinylabscom/mvm/actions/runs/44278080837).

The complete installed resource envelope and remaining storage lifecycle witnesses are open under
[#258](https://github.com/R4-Laboratories/SpiralCore/issues/258). [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42)
retains installed mailbox ownership, sustained persistent delivery, admitted FlowMux and owner
access, platform and two-host WAN qualification, and measured capacity. Private MVMD integration
remains external. The upstream contribution follows qualification of the complete supported deliverable.
