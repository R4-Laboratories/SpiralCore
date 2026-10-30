# Coupled ternary/Kuramoto/CA transfer: implementation and measured results

The retained coupled runtime is now part of SpiralCore's Rust object-transfer
path. A receiver loads an authorized, verified checkpoint, executes the bounded
recipe, applies exact residuals, verifies the original length and digest, and
durably publishes the object. This mechanism completed **480 local operations
and 72 real Linux-to-Mac WAN operations with exact bytes after reopening**.

The favorable 14,851,384-byte workloads require **747 or 766 bidirectional
protocol-body bytes** with an already seeded checkpoint, versus 14,883,938 raw.
The WAN pilot is faster than raw for these workloads after including encoding
and replay, but **zstd is faster and uses substantially less endpoint CPU**.
Unrelated data gets no byte reduction and pays extra prediction CPU. The byte
advantage is established for these model-generated surfaces; a compute or energy
advantage is not established.

## What was implemented

The Rust engine preserves the inspected Python equations and transition order:
ternary root folding, 16 Kuramoto oscillators, phase-sum memory feedback, and six
periodic cellular-automaton rings using rules 184, 110 and 30. It uses a shared
checkpoint and logical tick sequence. Identical physical clocks are not required
for exact reconstruction.

The observed fold averages current root values without weighting them by the
original payload counts. It is a lossy mathematical summary. The port preserves
that behavior and keeps it separate from the lossless object contract. Canonical
checkpoints retain the state needed for future transitions; the trace output
retains every complete post-step checkpoint. Exact target bytes, including
corrections outside valid trace syntax, are preserved by the residual and final
verification.

`RuntimeV1` is mode 5 in the real protocol and endpoint. The Rust API
`client::transfer_runtime_batched` adds replay to the existing byte-first
raw/zstd/sparse/prefix selector. Application-supplied checkpoint/recipe knowledge
makes this candidate available. It does not search arbitrary programs for a
compression recipe. The receiver executes only the fixed bounded Rust model,
never Python, saved-card source or received code.

The recipe itself is 24 bytes; checkpoint identity, prediction digest, residual,
framing and replies explain the larger observed total. A one-bit target edit adds
19 body bytes in this encoding. This is not a claim that all change positions,
dependencies and protocol metadata can be represented in one bit.

The exact-size preflight computes output length using the ternary fold schedule
before numerical replay. A retained regression showed a 4,678-byte trace
reserving 16,777,216 bytes; it now reserves the required capacity. Native golden
outputs did not change.

## Source, qualification and experiment identity

- Runtime, harness and local campaign source:
  `b6627deb7fb8c80a46c0e89d5e074afaf50f37f0`.
- WAN source: `35114daef384186dacd6123221ec9b5db60b3fcc`. The Rust source, harness,
  Cargo files, imported store and reference vectors have no diff from `b6627de`.
  This revision corrects the workflow's job-level build-directory context.
- The [design](../specs/2026-10-30-shared-runtime-design.md),
  [registered measurement contract](evidence/2026-10-30-shared-runtime/measurement-contract.md),
  [reference packet](evidence/2026-10-30-shared-runtime/reference/README.md), and
  [native qualification](evidence/2026-10-30-shared-runtime/qualification/README.md)
  retain scope, source hashes, commands, failures and limitations.
- Native release qualification: 220 passing tests across 26 targets, two
  historical large HTTP tests explicitly ignored; one passing doctest; 71
  passing imported-store tests; check, Clippy, build and new-file format checks.
  Focused coverage: 97.94% runtime lines and 92.59% transfer-helper lines.
  Branch/MC/DC coverage was not measured.
- Linux qualification: the frozen native-vector test passes, then **222 release
  tests pass with no ignored tests**, followed by the fresh sender build.
- The reference compares 123 coupled Python transitions, with exact CA/count
  checks and explicit floating-point tolerance. Rust compatibility vectors and
  the actual WAN fixture plans agree byte-for-byte between Apple ARM64/macOS
  and x86-64/Linux. This qualifies these builds and vectors, not every compiler,
  architecture, math provider or complete Python card representation.

## Local measured results

Two separate registered plans use widths/steps 48/243 and 4096/4096. Their actual
targets are **142,738 bytes** and **14,851,384 bytes**, without padding to a
headline size. Generated, one-bit-edited and unrelated cases each have ten paired
trials and four methods. All 240 targets and 240 separately measured seeds reopen
exactly. The independent checker validates 60 paired inputs, 120 selected-cost
equalities, 240 ordered cleanup receipts and 1,799,742,000 compared bytes.

These transfers run through actual serialized batches and the durable endpoint
inside one process. Byte counts are HTTP-body equivalents, not network traffic.
CPU already includes both endpoint roles; adding the nested receiver metric
would double-count it. The table reports target-only medians in milliseconds.

| Target / case | Raw body B | Runtime-enabled body B | Raw CPU / elapsed ms | Runtime CPU / elapsed ms |
|---|---:|---:|---:|---:|
| 142,738 B, generated | 143,505 | 747 | 1.768 / 8.819 | 2.990 / 11.358 |
| 142,738 B, one-bit edit | 143,505 | 766 | 2.072 / 8.672 | 5.710 / 12.616 |
| 142,738 B, unrelated | 143,505 | 143,505 | 1.847 / 9.108 | 3.871 / 11.711 |
| 14,851,384 B, generated | 14,883,938 | 747 | 35.587 / 53.825 | 524.578 / 552.290 |
| 14,851,384 B, one-bit edit | 14,883,938 | 766 | 34.993 / 54.168 | 521.579 / 544.367 |
| 14,851,384 B, unrelated | 14,883,938 | 14,883,938 | 35.419 / 52.093 | 269.958 / 290.802 |

Runtime has no local target-only CPU or elapsed-time win against raw in these
cells. The [independent local report](evidence/2026-10-30-shared-runtime/independent-analysis/README.md)
retains all four methods, individual observations, paired comparisons,
nearest-rank p95, generation costs and seed scenarios. A separate jq calculation
agrees with all 72 metric groups across 12 metrics; 17 malformed-data variants
are rejected by the independent analyzer.

## Real two-machine WAN pilot

[Run 41062532491](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41062532491)
sent from a GitHub-hosted x86-64 Linux runner through a temporary Cloudflare HTTPS
relay to the Apple M1 receiver. All methods use the same authenticated transport
and verified durable-byte publication, with optional DAG warming disabled.
The fixed dose is three paired trials per case, four methods, and one raw seed
per method: **36 target operations plus 36 seeds**. All 72 acknowledgements join
to successful exact reads after stopping and reopening the receiver, totaling
534,771,576 compared bytes. Mac and Linux generated byte-identical plans.

Each target contains 14,851,384 bytes (about 14.16 MiB). These are target-only
medians. CPU subtotal is calculated per trial as sender process CPU plus the
reported receiver-handler CPU, then summarized. It excludes relay and remaining
server/host overhead. Elapsed time runs from input-ready through verified durable
commit acknowledgement and includes every evaluated codec candidate.

| Case | Method | Actual HTTP body B, both ways | Endpoint CPU subtotal ms | Complete elapsed ms | Exchange subset ms |
|---|---|---:|---:|---:|---:|
| Generated | Raw | 14,883,938 | 64.016 | 1,289.771 | 1,276.842 |
| Generated | Zstd | 3,513,668 | 68.321 | 395.862 | 369.532 |
| Generated | Existing adaptive | 3,201,258 | 154.410 | 442.376 | 331.009 |
| Generated | Runtime-enabled | **747** | **846.692** | **956.126** | 374.316 |
| One-bit edit | Raw | 14,883,938 | 75.637 | 1,305.359 | 1,292.513 |
| One-bit edit | Zstd | 3,513,672 | 72.507 | 441.491 | 415.916 |
| One-bit edit | Existing adaptive | 3,201,257 | 149.040 | 514.517 | 402.621 |
| One-bit edit | Runtime-enabled | **766** | **853.439** | **977.758** | 392.198 |
| Unrelated | Raw | 14,883,938 | 83.167 | 1,169.926 | 1,157.175 |
| Unrelated | Zstd | 14,883,938 | 67.983 | 1,463.649 | 1,442.349 |
| Unrelated | Existing adaptive | 14,883,938 | 118.013 | 1,417.490 | 1,364.084 |
| Unrelated | Runtime-enabled, selects Raw | 14,883,938 | 589.310 | 1,862.754 | 1,344.364 |

The exchange subset includes serialization, request waiting, receiver execution
and reply processing. It is **not pure time spent transmitting bytes** and must
not be added to complete elapsed time. Body counts exclude HTTP headers, TLS,
TCP/IP, retransmissions and the relay's other network leg. There was no packet
capture in this pilot; complete wire-byte and pure wire-time claims are absent.

Generated and edited runtime transfers save approximately **99.995% of body
bytes versus raw**, and over 99.97% versus the configured adaptive control. Their
median completion times are roughly 0.96–0.98 seconds versus raw's 1.29–1.31
seconds. They remain slower than zstd's 0.40–0.44 seconds, with much greater CPU.
Unrelated content selects raw but retains the losing prediction's CPU cost.

These are three-trial pilot results. They do not satisfy the larger 30-trial
acceptance protocol or establish stable p95, statistical significance, direct
peer performance, controlled WAN impairment or energy savings. Every trial,
including unfavorable comparisons, remains in the
[WAN evidence](evidence/2026-10-30-shared-runtime/wan/attempt2/remote/observations.jsonl).
The [independent WAN analysis](evidence/2026-10-30-shared-runtime/wan-analysis/README.md)
joins all 72 operations to receiver verification, checks 50 source-manifest files
and 29 artifact members, rejects 27 malformed-data variants, and cross-checks all
36 metric cells and 24 paired comparisons with a second implementation.

## Checkpoint cost and application generation

The large checkpoint contains 3,382 bytes and costs **3,883 bidirectional body
bytes** to seed. Practical cold-start runtime cost is therefore:

| Case | Runtime seed + target body B | CPU subtotal ms | Complete elapsed ms |
|---|---:|---:|---:|
| Generated | 4,630 | 851.216 | 1,061.108 |
| One-bit edit | 4,649 | 861.661 | 1,078.554 |
| Unrelated | 14,887,821 | 597.976 | 1,960.758 |

Compare these rows against **raw/zstd target-only** rows above, since those
methods do not require a checkpoint. The equal-setup seed rows are still retained
for all methods, and adaptive's checkpoint remains part of its cold-start cost.
The favorable cold-start runtime trials all beat raw elapsed time in this pilot,
while all lose to zstd. No seed amortization is silently assumed.

Fixture generation is recorded separately once per paired input. It includes
construction of the model trace even for the unrelated control, so it is not an
unavoidable cost of sending an already existing file. Runtime-enabled transfer
then pays its own candidate replay at the sender and, when selected, replay at
the receiver. Those costs are included in the tables.

## Interpretation and next implementation focus

The confirmed mechanism is shared deterministic state plus a small operation
recipe and exact corrections. The receiver did not already possess the full
target trace. Its supplied base was the small checkpoint. A conventional cache
already containing the complete target or a previous full trace is a different
comparison; the results do not claim to beat such an unmeasured baseline.

The coupled runtime remains the main implementation focus. The next compute
experiments should measure retained prepared predictions, incremental replay,
packed/word-parallel CA evolution and bounded candidate selection against this
frozen scalar baseline. Each must preserve canonical bytes, include preparation
and state/cache costs, and retain losing cases. These are next optimization
targets, not improvements claimed by this result.

MVM integration follows the core optimization. Its adapter must bridge the
canonical mailbox/vsock service and support a small orchestrator, with authorized
plaintext codec placement, separate encrypted-envelope handling, tenant/recipient
and epoch binding, bounded queues and backpressure. This delivery does not yet
attach to MVM or establish CUI/FIPS deployment qualification.

## Retained failures and cleanup

The first WAN launch, run 41061178351, failed before creating any job because
GitHub rejected `runner.temp` in job-level `env`. The one-line workflow repair,
actual annotation, zero-job response, new negative gate and independent rereview
are retained. It produced no WAN transfer measurement. The successful run uses a
new source pin, public run ID, capability and fresh receiver cache.

Both temporary receivers and tunnels were stopped. The dedicated GitHub secret
and four experiment variables were removed, as were the two local capability
files. The verified receiver cache and immutable downloaded artifact remain in
task-owned local storage for review. Public evidence contains no capability.
The original local Python wrapper, sessions and saved cards remain preserved.
