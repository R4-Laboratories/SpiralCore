# Receive reentry failure custody

Source: `92830ef26daa169e05aa923da228d2796a224afe`, PR #278.
This is a bounded cooperative-decoder repair, not complete MVM delivery.

The original worker acquires the same decoder lock as assistance and takes any
owned assisting error before polling the socket again. Thus a later OS reset
cannot replace an already published refusal at receive reentry. The original
error object, including custom I/O context, reaches the original worker join.
Independent request authorization/storage/integrity/delivery/write errors retain
their existing precedence. No generic I/O-category substitution is added. A
request-side write already in flight at cancellation is outside this rule.

The new reentry test fails with the old plain-reader implementation (0 passed,
1 failed, 46 filtered), then passes with the guarded worker entry. A first test
setup compile failure referenced the not-yet-added helper; it was not a runtime
RED. The original 32-interleaved-request join assertion is unchanged. Additional
controls preserve original custom I/O context and independent same-kind worker
I/O. Existing cancellation-before-publication and decoder retirement tests remain.

Focused independent code/security review caught the initial copied-I/O-context
loss; taking the owned slot resolved it. Final bounded review found no remaining
code/security findings. Native CI is a separate acceptance gate.

## Exact Linux source and execution

The remote overlay's 181 tracked component/core/Windows-storage/object-store
files match the local source byte for byte; see source-verification.json.
Reuse toolchain 1.97.1, shared Cargo cache/target, two build jobs, no incremental
builds, user service MemoryMax 6 GiB / no swap / CPUQuota 200%. No VM booted.

- 48 library and 37 integration entries pass; eight separate resource controls
  pass. Doctests execute zero entries. Scopes are component tests.
- Strict release component Clippy, all targets/all features, passes.
- Measurement re-execution recovers exactly 67,108,864 bytes each direction.
  Sent TLS counters are 5,395,050 and 5,395,015 bytes: total actual traffic once
  across the pair is 10,790,065 bytes. Reciprocal received counters agree.
  TCP/IP overhead/retransmission bytes are outside these TLS counters.
- Setup wall 23,298 microseconds; lifecycle wall 40,978,952 microseconds;
  lifecycle process CPU 5,018,603 microseconds. Process CPU includes both peers;
  isolated sender/receiver CPU is unavailable, not estimated. HWM 6,664 KiB.
- Both endpoints end with 64 publications / 1,051,712 cache bytes and 4,032
  evictions. Temporary payload stores are fixture-owned and removed by Drop.

Logs are compact retained evidence, not raw corpora. No matched raw/zstd or WAN
performance claim follows. The full five-part goal and signed-runtime, MVMD,
platform/WAN/capacity/security gates remain open under #42 and linked issues.
Native source run: https://github.com/R4-Laboratories/SpiralCore/actions/runs/43627410037
Timing run: https://github.com/R4-Laboratories/SpiralCore/actions/runs/43627410074
The first macOS Intel timing attempt failed before compilation on runner DNS;
one targeted infrastructure retry was requested. Historical e03c ARM reset and
Windows sustained negatives remain in the published restart handoff.

## Native successor observations

Run 43627410037 Windows2025 job 132546356009 passes both executions of the
48-library suite, including the unchanged 32-interleaved refusal assertion.
Its default sustained fixture fails: snapshot wall 110,993,144 microseconds,
process CPU 40,406,250 microseconds; progress [2737,2743,2743,2737]. Actual TLS
sent counts at that concurrent snapshot total 7,220,261 bytes. First local
failure is ReceiverDelivery/Timeout on Commit; the peer reports subsequent
SenderReplyWait/UnexpectedEof. The snapshot is not full 64 MiB fidelity evidence.
The instrumented Windows2025 timing job in run 43627410074 passes. A pass and
failure on different hosted workers do not establish a timing root cause or
resolution. PR #278 remains draft and must not merge with this sustained failure.

Windows ARM job 132546355976 reports successful native decoder/reply-progress
controls, covering the original failing assertion. Its full component job is
still running at this publication checkpoint; do not infer full platform success.

The next bounded experiment is `15f82498` on codex/windows-commit-substages,
run https://github.com/R4-Laboratories/SpiralCore/actions/runs/43628084284 .
It adds opt-in process-aggregate storage substage wall/thread-CPU counters;
392 bytes static storage plus stack guards, no heap event buffer. Default work,
authority, durability and deadlines are unchanged. Stage maxima are independent,
not correlated to an ordinal/peer. Linux focused instrumentation test, strict
transfer library and component all-feature Clippy, and instrumented sustained
fixture pass. Native results remain required before attributing Windows delay.
