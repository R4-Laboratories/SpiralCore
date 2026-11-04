# SmallV1 component measurement and source freeze

The shared-core profile is implemented and frozen for independent final review.
General source baseline is2a9d2197d7c73fed46203d00440555665660ff13. Final
product/test files are in the shared worktree; root owns the publication pin.
Before/after test executables and SHA-256 custody remain in the owned Wick
directory `/var/tmp/spiralcore-small-profile-20261103/{bin,validation}`.

This is a single-host, in-process core-frame/private-file publication comparison.
Each arm executes3 trials ×5 sender modes ×7 identical workloads, with105 exact
object IDs/lengths/readbacks. CPU is current-thread time; the callback's receiver
dispatch is separately timed and subtracted from total to label sender work
outside dispatch. Target wall, setup/seeding and verification are separate.
Bytes are actual encoded core request/reply frames, excluding batch/carrier/TLS/
IP framing. Fixture synthesis/fitting and compiler time are excluded from target
timers. Whole test-process lifecycle/RSS include its normal fixture/drop overhead.
No native MVM, WAN, energy, causal CPU attribution or universal speedup is claimed.

The initial default-General comparison is retained. A final General control
uses its existing ColdGraphPolicy::Disabled, matching mandatory SmallV1 graph
policy; this control was documented before execution. All General selected
modes and core-frame bytes remain identical before/after across105 targets.
Initial records are not replaced by the final control. A shared host and only
three trials make microsecond CPU and filesystem wall differences exploratory.

Final SmallV1 target totals versus before General with graph construction
disabled,21 targets per mode (setup/seeding excluded from this table):

| Mode | Before frame bytes | Small frame bytes | Small target thread CPU us | Small target wall us | Median paired CPU change | Median paired wall change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Raw |183756|183756|2412|121949|−23.49%|−22.21%|
| Whole |72006|72075|2427|127603|−34.78%|−21.84%|
| Warm |66039|66057|2655|105960|−44.00%|−33.31%|
| Runtime |61107|61143|3596|108817|−2.25%|−1.20%|
| Combined |60600|60585|3507|99505|−22.22%|−4.48%|

Retained losses matter: Whole adds69 bytes, Runtime36 and Warm18 across their
three trials; Raw is equal and Combined removes15. Runtime's aggregate target
CPU increases3548→3596us (+1.35%) despite its slightly lower paired median;
receiver dispatch rises2553→2767us (+8.38%). Combined setup+seed CPU rises
1067→1229us (+15.18%). Every paired row and individual CPU/wall/byte loss is in
summary.json; no slow result was dropped.

Small Combined uses64416 frame bytes including3831 seed bytes, versus Whole
72075 (10.63% fewer), but spends4736us setup+seed+target thread CPU versus
Whole2723us (+73.93%), and175442us corresponding wall versus163429us (+7.35%).
These local fixture observations preserve the byte/compute tradeoff.

Observed maximum test-process RSS is4020KiB before General-no-graph,4464KiB
after General-no-graph and4672KiB Small: Small is16.22% above before and4.66%
above current General. The earlier default-policy processes measured4364/4352/
3892KiB respectively; those observations remain retained. Neither set proves an
allocation upper bound. The **8,284,092-byte declaration** comes from the pinned
C/Rust source model and covers one prepared sender+receiver pair, not four
endpoints, process RSS, carrier/TLS memory or an installed VM. All live bytes
must keep their appropriate shared outer reservation.

Final source qualification:144 passed/0 failed in library plus11 selected
integration targets, including existing600-byte and16MiB General regressions.
One preexisting ignored test, exact_repeat_sender_cpu_profile, is an explicitly
opt-in CPU diagnostic requiring a coordinated window; it is not counted as
executed. All7 new Small boundary tests ran. Strict release Clippy all-targets
passes with warnings denied. Initial API-absence REDs, oversized old-base RED,
public-selector prune bypass RED and prediction-later-target RED are retained,
along with environment/missing-fixture and Clippy failures. Root approved the
two-line example-only Box fix inside its constructor timing interval; no lint
suppression or General product allocation was added by that fix.

Scope remains a bounded shared-core prerequisite. Actual parent lease ownership,
FlowMux/Access factory execution, native platform qualification and the broader
B6b/capacity/WAN release gates require their own evidence.

Explicit sustained-use blocker:32 retained attempt states can refuse after
roughly512KiB of unique16KiB blocks (earlier when seed states count), and64
publication records after roughly1MiB, before TTL expiry; published files have
no progress-based eviction in this slice. This profile is NOT production
FlowMux sustained-ready. The factory's persistent128-block regression and a
reviewed, replay-safe shared-core progress-retirement/cache-eviction mechanism
remain gates. Increasing caps, advancing clocks or resetting namespace/cache
is not a substitute.
