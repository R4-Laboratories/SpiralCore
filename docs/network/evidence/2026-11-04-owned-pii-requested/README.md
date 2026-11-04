# Owned scanner requested-allocation witness

2026-11-04, Wick Linux x86_64, Rust 1.97.1. This synthetic, single-thread
component measurement supports the installed-service resource model. It
transfers no network data and boots no VM. It establishes neither a
whole-process RSS bound nor a FIPS, speedup or energy result.

The Rust witness calls the actual canonical Core `PiiRedactor::curated_owned`.
Its measurement-only allocator delegates to `System`, accounts only successful
operations and retains sticky accounting faults. Successful realloc tracks both
resulting requested live bytes and a separately labeled conservative old-plus-new
overlap; that overlap is not an observation of allocator internals. Phase resets
preserve live storage. Callbacks do not allocate, log or panic.

The same scanner is reused without cloning or thread migration. Linux
`/proc/self/task` checks observe one thread. Fixed 32,768-byte synthetic inputs
are prepared outside measurement. Reporting and legacy comparison are outside
owned scanner windows. Thread CPU, elapsed wall and process peak RSS are
recorded separately.

## Finite constructor execution

The five canonical rules permit 325 nonempty ordered selections:
`5 + 20 + 60 + 120 + 120`. The Rust generator test checks their counts and
uniqueness. The mode verifier first rejects the older two-policy execution,
then accepts the actual 325 constructor rows and explicit completion marker.
Every selection compares actual owned and legacy masked bytes and rule-name
sequence on the same dense synthetic input.

| Observed quantity across 325 constructors | Result |
| --- | ---: |
| Exact owned/legacy comparisons | 325 / 325 |
| Maximum requested peak above its constructor baseline | 2,254,054 bytes |
| Maximum conservative realloc overlap above baseline | 2,391,826 bytes |
| Maximum compiled retained requested bytes | 1,938,125 bytes |
| Post-drop requested-live delta, every selection | 0 bytes |
| Sum of measured constructor thread CPU | 471.575173 ms |
| Median measured constructor thread CPU | 1.667529 ms |
| Maximum measured constructor thread CPU | 1.991237 ms |

There is one execution per selection. The largest requested constructor peak
occurs at selection 191. The earlier default all-five sample's 1,423,560-byte
peak therefore cannot represent the worst observed rule ordering. Independently
maximized columns need not occur in the same selection.

The full finite run, including legacy construction/comparison and reporting,
took 1.15 seconds wall and observed 9,120 KiB process peak RSS according to
`/usr/bin/time -v`. These process observations are neither constructor-only
time nor an application RSS limit. The Rust CPU rows isolate owned constructors.
No randomized timing or before/after speedup is claimed.

## Reused search sample

A separate run constructs two policies, uses each scanner for first and eight
repeated redactions of dense, Unicode/near-miss and ASCII inputs, then verifies
exact legacy outputs. Both policies return to their 99,980-byte pre-policy
baseline after the scanner and retained observations are released. The all-five
constructor peak is again 1,423,560 bytes. The full sample process took 0.07
seconds and observed 6,740 KiB peak RSS. Raw phase results are in the archive.

A phase's `peak_above_baseline` must be added to its own `live_before`. Taking
only the largest phase increment omits compiled state, accumulated search caches
and retained outputs. Finite constructor selection coverage does not exhaust
arbitrary search inputs, concurrency, other architectures, native overhead or
process lifetime behavior. The proposed total scanner allowance still needs
the complete source model and installed integration; this witness alone does
not approve it.

## Source, execution and custody

[Compact witness archive](requested-allocation-receipts.tar.gz), SHA-256
`4528b175326b3e74e6b8e7773aebe3eeeeb1373728c239058f33149f67fe8417`,
is 163,638 bytes: 48 hashed members plus the original JSON index. Every member
digest was independently checked after transfer. It includes Rust source, plan,
harness manifest/lock, run command, outputs, source manifests, protected binary
hashes and the original accounting/allocator/generator/mode failures. Original
binaries and compiler JSON remain on Wick.

Independent reviews cover the [original accounting mechanism](reviews/review-session-debug.md)
and [finite constructor extension and recorded artifacts](reviews/review-constructor-extension-session-debug.md).
Their PASS is scoped to this witness, not acceptance of the total service budget.

The final cached release build exited zero in 0.03 seconds; it is not described
as a newly emitted binary. Before execution the executable was copied outside
Cargo, protected mode 0500 and hashed. Both protected executable hashes and all
594 selected source inputs match after both executions. Snapshots include the
actual Core/Contract files, root manifests/lock and witness. This closure pins
regex 1.12.4 and automata 0.4.14.

This measurement has an explicitly recorded intermediate path closure into the
owned MVM checkout. Its harness path is not a shipping dependency or a claim
that an arbitrary clone of main reproduces the same WIP source. Repeat against
the final published all-Git release graph before accepting the installed native
profile. The full source manifests permit that comparison. No raw customer
data, payload store, VM disk or executable binary is included in Git.
