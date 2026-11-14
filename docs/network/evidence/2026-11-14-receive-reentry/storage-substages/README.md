# Bounded native storage-substage experiment

Source `15f824989be41d8ec6e3b13fdac9b22c097bb8ec`, tree
`40dfe5b76ee2fb27f0e4515b02af411fd596e96a`; draft PR #282 stacks on #278.
Run https://github.com/R4-Laboratories/SpiralCore/actions/runs/43628084284 .
The later main-evidence merge changes no component source. This experiment does
not resolve the retained Windows sustained failure or qualify an MVM/WAN role.

## Design and qualification

The existing witness-timing feature enables seven fixed substage counters in the
same Rust transfer core, compiled out by default. Process-static storage is 392
bytes, with stack-only span guards; no new dependency or event buffer. Errors
retire through Drop. No ownership/identity/authority, live quota, durability,
verification/readback or original 2 s/240 s deadline check is changed.

Counters aggregate both endpoint workers over process lifetime: completed calls,
active spans, total/max wall ns, separate total/max thread CPU ns, unavailable CPU
count. Concurrent fields and maxima are independent, stages overlap and cannot
be added into a causal timeline. Active counts have no ordinal, direction or
start time. Windows thread CPU resolution is coarse (observed 15.625 ms ticks).
No per-ordinal causal attribution is claimed.

Focused independent code/security review identified unused guard names, explicit
static-storage inventory and one missing format argument. All were corrected.
Retained Linux intermediate warning/build failures remain in the shared overlay;
final strict transfer library and component all-target/all-feature Clippy pass.
The focused RAII/error-return/unavailable-CPU/saturation control passes. Final
instrumented Linux fixture passes exact 64 MiB/direction in 41.646845 s wall and
5.062792 s both-peer process CPU; total actual TLS bytes 10,790,065.

## Windows2025 x64 executed result

Job 132548372197 on native image 20261103.298.1 passes the unchanged full sustained
fixture: 4,096 distinct 16 KiB blocks/direction, 67,108,864 exact bytes/direction,
reciprocal actual TLS counters 5,395,050/5,395,015 (10,790,065 bytes sent once across
the pair). Lifecycle wall 206.066600 s; both-peer process CPU 60.390625 s; setup
wall 68.070 ms. Isolated sender/receiver CPU is unavailable. Final caches each
retain 64 publications / 1,051,712 bytes, 4,032 evictions, zero pinned entries.

| Substage | Calls | Total wall s | Maximum wall ms | Total thread CPU s |
|---|---:|---:|---:|---:|
| Quota | 16,384 | 29.772553 | 9.4195 | 17.578125 |
| Victim | 8,064 | 5.464237 | 161.9379 | 5.671875 |
| Create/write | 8,192 | 104.324639 | 740.5239 | 10.093750 |
| File sync | 8,192 | 205.176229 | 914.5595 | 1.093750 |
| Namespace publication | 8,192 | 52.315025 | 47.7928 | 6.437500 |
| Verify publication | 8,192 | 2.687111 | 1.5066 | 0.531250 |
| Readback | 8,192 | 2.523954 | 7.1168 | 0.890625 |

All active/unavailable-CPU counters are zero at the final snapshot. Large elapsed
create/write and sync costs with much smaller aggregate thread CPU make blocking
storage/scheduling worth isolating next. This is an inference from a passing
run, not a reproduction or cause of the earlier >2 s failure. Maxima are not
necessarily the same block and cannot be summed. No security check or durability
operation can be removed on this evidence.

## Original artifact custody

`windows2025.zip` is the original 4,861-byte Actions artifact 14126664493,
SHA-256 `05d001d26bc597811003cf02840ab9e2f76fcfaa7bcebc98e1485f90bd054130`.
The service digest and downloaded ZIP match; all three members listed in its
SHA256SUMS independently verify. Original CRLF bytes are preserved inside ZIP.
It contains identity.json, rustc.txt, tests.txt and SHA256SUMS. No executable,
VM disk, raw corpus or payload store is copied locally.

Windows2022 stall and macOS Intel artifact-stall rows also pass. The macOS Intel
stream row remains in progress at this checkpoint. The Windows2022 stall feature
does not enable the new substage counters; do not present it as such evidence.
Complete all required final-source gates before any production merge. The five
#42 outcomes, external signed-runtime/MVMD and WAN/capacity/security gates stay open.
