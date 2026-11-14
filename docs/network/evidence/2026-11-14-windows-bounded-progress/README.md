# Bounded duplex pacing and Windows storage qualification

This is a component implementation/evidence slice toward #42/#44/#46/#49, not
complete MVM delivery. Reviewed source `f680fc612084dd1f2d467a65228d1e429c8e6037`
is delivered through PR #282 at `22f9fe7b` with an identical full source tree. All five #42 outcomes remain active. Ari's accepted
signed runtime, private MVMD, installed recovery, native/WAN role acceptance,
capacity envelope and security/FIPS qualification retain their separate gates.

## Retained failures and resulting changes

The header-only source `be5c84ad` does not resolve the problem. Windows2025
job 132556089860 fails default sustained delivery near blocks 80/81, while its
separate instrumented row passes. Ubuntu x64 job 132556090185 fails with the
original SenderReplyWait/Limit, progress [590,608,608,590], snapshot 3.801646 s wall /
1.526413 s process CPU. The faster direction must not consume the other pending
operation's 32-frame bound. The existing raw-peer 32-frame assertion is unchanged.

Earlier Windows2025 job 132550737017 fails near block 2068. Its actual PR-merge
source is `a1a0866e9f23f0ea7a571b200015d42aab4decf0`, with full tree
`565a4fcbfb32b88b0fe2ec28dbf01c6124892f24`, identical to candidate `be3ef571`. Its enclosing
FileSync span reaches 2.3068362 s and composite NamespacePublish 2.2120239 s. These
include clock/scheduling overhead and do not isolate kernel calls. They exceed
the entire original 2 s operation bound. Concurrent failure snapshot 93.216920 s
wall /41.359375 s process CPU, 5,447,585 actual TLS bytes sent once across the pair at that snapshot;
there is no complete 64 MiB fidelity result for that failed row. Windows2022
job 132550736945 also fails at delivery near 3853/3858, with a still-running sampler
and flat process-CPU samples. Neither a universal storage nor CPU cause follows.

The candidate preserves one original decoder, owned-error custody and the same
wire/authority. Checked inbound-handler generations pace repeated new sender
operations: one may be admitted during a busy generation, plus any operation
already in flight. Later callers event-wait without TX/RX/pending locks, using
their original deadline and cancellation/live scope checks. Admission failures
use the existing observe/cancel/receiver-state-invalidation path. Generation
exhaustion refuses rather than wrapping. The claim is bounded Whole-block burst
pacing, not a universal proof for every multi-exchange/repair operation below 32.
No frame limit, deadline, security check or service authority is widened.

The 49- and 81-byte publication headers now use one write_all invocation and an
81-byte stack buffer, preserving identical bytes. Short writes may still require
multiple OS writes. Body write, live quota/identity/authority, sync_all, durable
namespace publication and exact readback remain. The prior header-only Linux
instrumented 73.122667 s result is retained as a slower observation, not a win.

The earlier failure processes did not attest the actual Rust temporary root.
They are labelled default-temp failures, without assigning them to a drive.
The C:/D: control and final native rows below directly attest their own roots;
the placement observations do not prove the cause of every earlier timeout.

## Same-executable native Windows storage control

Manual run 43631053632 at `ce58cb2e1ce4dca5cee87a9113f0c1c4d22662f4`
builds one native executable, registers both roots before running, checks its
hash after each control and records the actual root selected by Rust. It uses
original 2 s/240 s gates; a separate 270 s process watchdog is not a passing waiver.
Both original processes exit 0 without that watchdog firing. Default then
alternate is an exploratory fixed order, not a randomized performance study.
The experimental workflow replacement remains on its separate branch.

| Registered root | Partition disk | Exact bytes per direction | Actual TLS bytes both directions | Lifecycle wall s | Both-peer process CPU s |
|---|---:|---:|---:|---:|---:|
| OS default C: user temp | 0 | 67,108,864 | 10,790,065 | 130.967258 | 115.531250 |
| Explicit D: owned scratch | 1 | 67,108,864 | 10,790,065 | 60.253825 | 89.953125 |

Both rows use native image 20261103.298.1 and retain exact readbacks, 64 publications,
1,051,712 cache bytes and 4,032 evictions per endpoint. Isolated sender/receiver CPU
is unavailable; process CPU covers both peers. TLS counts exclude TCP/IP overhead
and retransmission accounting. Scope is two local endpoints in one native process per row, no MVM or WAN.
C: FileSync max 156.8038 ms versus D: 28.5356 ms; maxima are different operations and
not a causal matched pair. These observations support testing the declared data
volume; they do not qualify arbitrary OS-temp disks or eliminate scheduling tails.

Windows qualifier workflows now explicitly select a ready native RUNNER_TEMP
volume, record configured/original paths and volume facts, and refuse an absent
root. The Rust fixture reports its actual root on default and instrumented rows;
qualification compares it with the selected root and refuses mismatch. There is
no fallback, RAM disk, AV exclusion or security/durability change. Earlier default-temp
negatives remain distinct from this scratch-volume qualification. Product callers
still provide their authorized cache root; no runtime root is silently substituted.

## Focused source checks

Independent bounded code/security reviews found no remaining custody, header or
pacing findings. The new held-handler control proves first independent 1 KiB
progress, second-caller enrollment without request publication, exact additional
readback after unconditional release, and original-worker collection before
assertions. Cancellation, deadline and generation-overflow controls also pass.
The early runtime RED used an uncommitted predecessor; it is not a pinned release
baseline. A separately pinned no-admission mutant control is recorded alongside
its patch, routing-file hashes and restoration check (50 tests pass), not presented as the historical release source.

Local prepublication component suite passes 50 library and 37 integration entries;
strict all-target/all-feature component Clippy and eight resource controls pass.
The final fixture-root diagnostic is separately rebuilt, checked and executed.
Final 182 tracked component/core/storage files match the Linux overlay; see the
source-verification record. That overlay is not advertised as a clean Git checkout.
The final-root local fixture recovers 64 MiB/direction with 10,790,065 total TLS bytes,
42.653418 s lifecycle wall /3.775914 s both-peer process CPU, setup 24.913 ms, HWM 6,592 KiB.
No energy, send-only time, matched codec or WAN speed claim follows.

Final-source native component run:
https://github.com/R4-Laboratories/SpiralCore/actions/runs/43632954356
Final-source timing run:
https://github.com/R4-Laboratories/SpiralCore/actions/runs/43632954428
Both runs are terminal SUCCESS on the final source. All seven native component
rows and all four timing rows pass; all 25 executed PR checks succeed and three
manual workflows are skipped. PR #282 merged at `22f9fe7bcd22352e1a24f097fe27b854a5983d7a`.
Its full tree equals candidate and CI merge tree `0d20b8181fa23f912cdd38dc510ab90fbe12c7c6`.
PR #278 is superseded by this delivery; no full program issue is closed.

## Final-source native sustained rows

Each row recovers exactly 67,108,864 bytes per direction and transmits
10,790,065 TLS bytes counted once across the pair. Setup, exact readback,
original-worker retirement and verified fixture-store removal are included in
the reported lifecycle. These are separate native runner observations, not a
matched cross-platform speed comparison or MVM/WAN qualification. CPU is the
combined native process CPU for both endpoints; isolated sender/receiver CPU is
unavailable. Hardware/image/source and configured/actual Windows roots remain
inside the original archives.

| Native row | Setup ms | Lifecycle wall s | Process CPU s | Attested Windows scratch root |
|---|---:|---:|---:|---|
| macos26-arm64 | 63.231 | 32.064704 | 29.972467 | not a Windows row |
| macos26-intel | 53.527 | 95.047114 | 68.815679 | not a Windows row |
| ubuntu24-arm64 | 9.142 | 8.004314 | 12.242895 | not a Windows row |
| ubuntu24-x64 | 6.822 | 4.320000 | 6.816134 | not a Windows row |
| windows11-arm64 | 134.625 | 190.423173 | 97.750000 | `C:\a\_temp` |
| windows2022-x64 | 93.762 | 29.905135 | 53.296875 | `D:\a\_temp` |
| windows2025-x64 | 48.384 | 35.089183 | 60.671875 | `D:\a\_temp` |

The eleven original component/timing archives total 180,267 bytes. All ZIP/API
digests and 144 listed member hashes verify. `native/verification.json` records
each actual checkout SHA and independently fetched Git tree; all equal the
released tree `0d20b8181fa23f912cdd38dc510ab90fbe12c7c6`.
`native/sustained.json` retains the table values in original microseconds.
Twenty-five CI checks succeeded, including setup/matrix jobs; three manual
workflows were skipped and are not counted as passing test execution.

The two original pre-fix component failure archives are also retained under
`pre-fix/`, with their sizes, ZIP/member hashes and actual checkout source/tree
in `pre-fix/verification.json`. Both executed trees equal candidate `be5c84ad`.
They preserve the Windows default-temp timeout and Ubuntu 32-frame refusal;
those failed processes did not attest their actual temporary roots.

## Original artifact custody and storage

- `windows2025-negative.zip`: original artifact 14127485784, 5,278 bytes,
  SHA256 `cdc84147ad7de13d8912d6327419e007e77c480152a277231de05356dbd96e1c`.
- `storage-layout.zip`: original artifact 14128720062, 8,405 bytes,
  SHA256 `f9e6d49acca484b00f08f889d0d7be07b3bff913fcc12c9a34e50648838b0d28`.

Both downloaded ZIPs match the service digest; every SHA256SUMS member verifies.
Original CRLF bytes are preserved. No raw corpus, VM disk or local Cargo tree was
created. Builds reuse the bounded Wick target: two jobs, 6 GiB memory/no swap,
CPUQuota 200%. Owned payload stores are removed by fixture Drop. The 1.7 MiB copied
fixture staging TAR was deleted locally/remotely only after all three inputs
matched their checkout hashes. Historical evidence and user runtime/saves remain.
