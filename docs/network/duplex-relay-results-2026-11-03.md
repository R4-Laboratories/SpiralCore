# Bounded duplex canonical mailbox relay

Product [`f964c034`](https://github.com/R4-Laboratories/SpiralCore/commit/f964c034c8ac5d32ef29aa4dfab5771b6867f534)
is the original native comparison source. The final shutdown repair is
[`62cb6042`](https://github.com/R4-Laboratories/SpiralCore/commit/62cb60424201d450350395224ecb7a2a98513549),
reviewed in [PR #58](https://github.com/R4-Laboratories/SpiralCore/pull/58).
[Architecture](../specs/2026-11-03-duplex-mailbox-relay.md) ·
[original evidence](evidence/2026-11-03-duplex-relay/) ·
[continuing plan](../plans/2026-11-03-persistent-mvm-service.md).

## Implemented change

The keyless native relay now forwards each provisioned direction independently,
replacing its fixed Call → destination receipt → Reply → Reply receipt ordering.
One maximum-frame credit is acquired for each direction before either handshake
or pump starts. Two joined pumps retain these credits during idle reads and
forwarding. The configured ceiling remains **two links, two slots and 524,288
reserved serialized-buffer bytes**. Idle reservation now intentionally peaks at
that ceiling; reserved capacity is not RSS. Decoded copies remain excluded from
serialized-buffer accounting. Four cloned descriptors plus the original two
provide bounded worker I/O and supervisor shutdown authority.

An explicit EOF-aware frame reader returns `None` only when zero bytes of the
next prefix have arrived. Clean EOF propagates a Write-half close, allowing the
reverse direction to drain. Truncated prefix/payload, malformed length,
cancellation, deadline and transport errors remain fatal. The existing strict
endpoint reader still treats clean EOF as fatal and closes both halves. Route
validation, framing and ciphertext bytes are unchanged. No envelope, receipt
authority, codec, pipeline window or automatic retry is added.

Pump errors and caught panics cancel/shut down the sibling, and both workers
join before return. Partial I/O counters remain in failure diagnostics; successful
endpoint totals combine readiness and directional counters with checked addition.
Each read/write retains its fixed 30-second operation bound. This is a bounded
relay prerequisite for C8, not an indefinitely idle mailbox service.

## Executed correctness and review

Semantic RED controls reproduced reverse-write `BrokenPipe` after a clean
half-close, reverse traffic stalling before any forward Call, and a second
forward frame stalling while reverse traffic was withheld. The original failing
test/log records are retained. Six focused real-relay regressions now pass:
independent directional progress, buffered reverse drain after half-close,
truncated prefix/payload cancellation, reservation rollback before readiness,
and a blocked partial output failing after its sink closes. They check exact
ciphertext, directional counters and zero final credits. Completion channels
distinguish autonomous sibling wake-up from test-side teardown.

Four native-portable TCP tests pass on Linux, covering the canonical encrypted exchange,
new EOF/strict-compatibility behavior, malformed lengths, truncation, cancellation
and deadlines, plus the owning transport's error fence. All **73 Linux release tests** pass, including nine persistent
object campaigns. Strict release Clippy and normal host/static-guest builds pass.
Actual thread-start/panic fault injection and a real 30-second blocked-write
timeout are not established by these tests; those cleanup paths were source
reviewed. No zero-test target is counted as an executed test.

Independent code/Rust, security, test and bounded-runner reviews found no blocking
defect. Four isolated instrumented binaries passed **61 tests** with empty
test/export stderr. Changed executable production coverage is **186/204
(91.18%)**; the new module is **157/174 (90.23%)**, and the EOF helper is
**26/29 (89.66%)**. The broader three-file production denominator is
**856/1,273 (67.24%)**, not whole-crate coverage. LLVM supplied no mapped EOF
branches; branch coverage is unavailable. All uncovered changed lines remain
recorded. Coverage ran after normal native measurements and preserved their
protected binary hashes; no instrumented timing is performance evidence.

Hosted Linux and seven-platform client qualification are still pending at this
report's initial draft. They must be bound to the reviewed product before merge;
cross-platform client CI does not establish native MVM backends or WAN.

The initial hosted run failed both macOS client rows at the new raw TCP closure
checks; five other native client rows passed. Both failed job logs are retained.
A small native Darwin Rust witness reproduced the cause: after peer FIN,
`Shutdown::Both` returned `NotConnected` (57) while a raw reverse write still
succeeded. Explicit Write shutdown closed that remaining half. The owning
`StreamTransport` already fenced protocol access after errors; that fence was
not missing. The final repair attempts Write and Read shutdown separately only
when an error occurred and Both shutdown failed, preserving the original error,
wire, timers and successful path. The original raw closure assertions remain;
a fourth TCP test checks protocol refusal and unchanged counters after fatal
receive. The exact repair passed fresh independent Rust/security review, focused
TCP/live tests, all 73 release tests, strict Clippy and normal static-guest/host
builds on Linux. Fresh hosted qualification and separate final native confirmation
are required; the original four-cell comparison below is not relabeled as final
repair execution.

The next hosted attempt passed the repaired raw half-close, transport fence and
canonical TCP tests on both macOS architectures, then failed the malformed-input
test's timeout setup: Darwin rejected configuring the probe after terminal
socket closure. The fixture now installs that timeout first; every original
fatal-type, partial-count, closed-write and peer-EOF assertion remains. Both
attempts' logs and executed-source metadata are retained. This fixture-only
correction at `fdfbd2f` changes no production file from `62cb6042`. Its four TCP
tests also passed natively on the local macOS **27.0 ARM** developer host in the
existing Cargo target; that is distinct from supported hosted macOS 26 and a
native MVM backend.

Final-source isolated coverage at `fdfbd2f` passed **62 tests**: library 38,
campaign 9, adapter live 11 and TCP 4. Changed executable production is
**187/210 (89.05%)**, with new module **157/174 (90.23%)** and EOF helper
**26/29 (89.66%)**. The broader three-file denominator is **856/1,277 (67.03%)**;
branch mapping remains unavailable, and Linux did not execute the Darwin-only
fallback conditions. All 23 uncovered changed lines remain recorded. The
initial shared-target instrumented binary paths were later overwritten by normal
build reuse; their observed exports, profiles and hashes remain, but those paths
are not retained original executable custody. Final instrumented binaries were
reverified against their recorded execution/export hashes, copied and byte-checked
on Wick, then protected from later target reuse. They remain remote with raw
profiles/LCOV. Normal host/guest hashes stayed fixed. Neither coverage run supplies
performance timings.

## Registered native before/after comparison

The four fixed-order exploratory cells ran before zstd, before Combined, after
zstd, after Combined, one trial each. Both versions used the same eight target
positions of **14,851,384 bytes** each (requested 16 MiB); Combined also published
its runtime checkpoint and schema. This sequence includes repeats, generated and
residual content, unrelated/opaque edits, and schema fit/drift. It does not
represent eight unrelated cold files. All **36 publication operations** and
**36 additional keyless reboot readbacks** matched exact lengths and bytes.

| Version | Arm | Logical bytes, both directions | Timed sender CPU | Timed receiver CPU | Relay process CPU | Lifecycle wall |
|---|---|---:|---:|---:|---:|---:|
| Before | Whole zstd | 107,347,479 | 4.467365 s | 3.779523 s | 0.105661 s | 8.964040 s |
| Before | Combined | 62,535,266 | 2.974948 s | 2.529433 s | 0.065310 s | 6.448698 s |
| After | Whole zstd | 107,349,624 | 4.373510 s | 3.693244 s | 0.124562 s | 8.691518 s |
| After | Combined | 62,523,237 | 3.024322 s | 2.498230 s | 0.071792 s | 6.401458 s |

Current Combined used **41.76% fewer logical bytes, 30.85% less timed sender
CPU and 26.35% less lifecycle time** than current zstd. Before/after Combined
sender CPU increased **1.66%**, while its lifecycle decreased **0.73%**. Zstd
sender CPU and lifecycle decreased **2.10%/3.04%**. Relay process CPU increased
**9.92%** for Combined and **17.89%** for zstd. Those relay costs are retained;
the relay change is an independent-progress fix, not a demonstrated CPU
optimization. Fixed order, one trial and other work on Wick preclude statistical
or causal attribution. No codec changed; small wire-byte variation from fresh
canonical identities/numeric ciphertext encoding is retained.

Logical totals include both directions of encrypted canonical envelopes,
four-byte prefixes and readiness. Physical accounted vsock stream bytes across
two hops are twice those totals. Virtio credit/control headers and workload-exit
service traffic are excluded. Every endpoint/relay counter agreed. Timed endpoint
CPU covers campaign setup and operation intervals, including fixture/service/
codec preparation inside them; it excludes untimed per-object ownership/identity
gaps, JSON output and independent final verification outside the timers. It is
not lifetime CPU. Lifecycle starts in `session_inner` after journal/link admission
and includes boot, transfer, keyless verification and teardown. Nested worker/VMM
CPU is separate and must not be added to guest CPU. No energy or send-only time
is inferred.

At most two Linux/Firecracker guests ran simultaneously, one vCPU/1,024 MiB each,
without a NIC. The original kernel/VMM were reused; the GNU static guest has no
interpreter. Linux still reports the requested `experimental64` rate as
unenforced. All before snapshots peaked at one slot/262,144 bytes; all after
snapshots peaked at two slots/524,288 bytes. Limits were identical, no refusals or
fencing occurred, and final usage was zero. These are local native results,
not WAN, fleet capacity, total-memory or host-wide plaintext enforcement.

The 278-file source manifest and normal binaries bind the execution. All **49
public archive members** verified locally before removing four successful owned
Wick test disks, totaling **1,073,741,824 apparent file bytes**. This is not measured
freed filesystem allocation. No VM disk or payload was copied to the Mac; prior
failures and other labs remain untouched.

## Final-source native confirmation

After the Darwin shutdown repair, two separately registered confirmation cells
ran the final `62cb6042` normal host/static guest. Both passed **18 publication
operations and 18 additional exact keyless reboot readbacks**, with matching
counters, unchanged limits, peak two slots/524,288 bytes and zero final usage,
refusals or fencing. This is additional final-source execution; it does not
replace the initial four rows or turn them into repeated randomized trials.

| Final arm | Logical bytes, both directions | Timed sender CPU | Timed receiver CPU | Relay process CPU | Lifecycle wall |
|---|---:|---:|---:|---:|---:|
| Whole zstd | 107,332,739 | 4.548448 s | 3.751087 s | 0.125550 s | 8.850043 s |
| Combined | 62,538,277 | 2.967917 s | 2.508044 s | 0.069418 s | 6.349624 s |

Final Combined used **41.73% fewer logical bytes, 34.75% less timed sender CPU
and 28.25% less lifecycle time** than final zstd in these one-trial cells. The
same workload, shared-host and timer exclusions above apply; these figures do
not isolate the shutdown repair's cost or establish WAN performance. Successful
transfers do not exercise the error-only Darwin fallback. Hosted macOS checks
and the retained native Darwin witness separately qualify that behavior.

All **27 final public archive members** verified before the two successful owned
Wick disks were removed (**536,870,912 apparent file bytes**). Across this
checkpoint's six native cells, all **54 publications and 54 extra keyless
readbacks** were exact. All original losses, initial/final binary/source
identities and failure records remain in the packet; no payload disk was copied
locally.

## Remaining work

Both-peer event demultiplexing, retained Reply/acknowledgment progress and cached
Reply replay remain next. Automatic retransmission is disabled. The current
endpoint transport holds one state mutex through receive and makes its deadline
fatal; a timer writer cannot safely add retry. After the peer state machines,
review an explicit zero-prefix receive-idle outcome before timed retry, preserving
fatal partial-frame handling and the original Call deadline.

The 2,048-Call witness guard and pipeline one remain. Full-retention sessions,
installed-host exclusion/orphan recovery, aggregate plaintext/cache partitions,
fairness/capacity, canonical direct/relay WAN, remaining native platform roles and
the outside-contributor MVM delivery remain open in
[#43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49) and
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
