# Owned storage and resource-accounting delivery checkpoint

PR #260 integrates the descriptor-rooted Unix SmallV1 store and original host/tenant resource funding with qualified main. Final source is `0a3a2f696ad918dcb46f1192c2750e1d8d88d2c7`, tree `e80dfec21720b9f0ab49dcd5034359a8a85601bc`, based on main `415c9fd1560506a7a1a2d7e5a46b4645cc3ac584`. PR #260 merged to main as `adcc7197aa277277be5d074383c6267476163206`; the released tree matches the reviewed candidate and CI merge tree. All 38 GitHub checks passed (including matrix/setup jobs) and four manual jobs were skipped, with no pending/failing check. These counts are CI check states, not unique test-suite counts. This is a component implementation checkpoint, not complete MVM/WAN/FIPS acceptance.

Independent code/security review found one integration defect: collector-only profile 1 was charged seven descriptors for a codec store it does not create. The corrected claim derives and charges store resources only for Object-TCP profile 2. The original collector resource claims and uncertain-retirement custody remain. A supported zero-store-FD installation fails with `managed resources: Full` before the correction and passes afterward. The original-parent serial/tenant/debit guards, private storage checks, durability barriers, exact readbacks, unchanged 32-frame assertion and deadlines remain.

The endpoint merge retains main's one-write 49-/81-byte publication header and routes both Path and owned storage through the same header and file sync implementation. Owned filesystem operations retain directory/owner descriptors. Quota checks occur independently at Begin and Commit; failed initialization/cleanup does not invent a refund. Existing Windows storage remains its supported Path implementation. Source review preserves the private mount/exclusive same-UID lifecycle assumptions; descriptor custody alone cannot exclude a hostile same-UID writer or establish authority.

## Executed local controls

`report.json` records individual raw summaries rather than adding nested or overlapping test counts. Wick passed 18 owned primitives, five public store controls, one retained descriptor lifecycle control, eight endpoint policies, all four collector policies, 173 canonical library tests, the 74-test supervisor/resource harness across six targets, 50 duplex library tests and 37 integration entries, five producer refusal controls and five private-handoff controls. Strict transfer and FlowMux all-target/all-feature Clippy and canonical-host all-target Clippy pass. The final signed-export selector executed zero tests and ignored one: **NOT_RUN** in this campaign. Earlier genuine signed-export/one-guest packets retain their original revisions.

The separate empty two-store lifecycle observes 3 baseline, 8 persistent and 3 final process descriptors, excluding one enumeration descriptor. The real traversal control observes four transient descriptors per serialized walk, then pauses two simultaneous walks at actual file verification: 9 persistent → 17 observed → 9 persistent. Both original threads are released and joined before assertions. Neither observation alone establishes allocator/libc bounds or fleet capacity. MVM's enforced memory/workspace quote and broader installed/fleet qualification remain separate.

| Local operation | Exact body bytes | Actual request / reply bytes | Wall | Process CPU |
| --- | ---: | ---: | ---: | ---: |
| Owned cold publication through readback | 16 | 235 / 218 | 9.342166 ms | 0.172613 ms |
| Owned warm probe through readback | 16 | 102 / 88 | 2.588808 ms | 0.074440 ms |
| Sustained TLS component, post-cleanup lifecycle | 67,108,864 each way | 5,395,050 / 5,395,015 TLS sent | 40.924472 s | 3.839583 s |

The first two byte scopes are in-process encoded Object-v1 frames, not network traffic. The sustained row is two peers in one native process, 4,096 exact 16 KiB blocks in each direction; sum sent counters once for **10,790,065 TLS bytes**. Receive counters mirror that traffic. The lifecycle ends after original workers retire, fixture stores drop and path absence is asserted. Setup is 21.979 ms wall / 1.305 ms process CPU; process HWM is 6,940 KiB. Separate sender/receiver CPU is unavailable. TCP/IP/retransmission bytes and energy are not measured.

The earlier full-suite observation is retained separately: 68.786058 s wall / 4.619766 s process CPU through a successful snapshot before store cleanup, setup 26.998 ms / 1.395 ms CPU. It is not the lifecycle row above. These are uncontrolled repeated local observations, not a speedup, WAN or matched zstd comparison. GNU-time command logs include Cargo/build and whole-test costs; they are not transfer CPU measurements.

## Source and failure custody

`wick-logs.tar.gz` retains the original logs, graph captures, 606-file source manifest, compiler identities, command scripts and scope observations. All 606 unique source/fixture hashes match the final Git objects and post-run files. Earlier 548-row checks covered code/tools; the additional 58 rows bind compile-time evidence inputs. The actual campaign uses Rust/Cargo 1.97.1 through its recorded PATH. `host-default-rustc.txt` is separately labelled host inventory (1.93.1), not the campaign compiler. Scope records preserve actual 6 GiB/zero-swap/200% settings where the original systemd unit still exists; successful transient units can be garbage-collected, so missing/default placeholder properties are not terminal resource proof for those stages. Scripts record two Cargo jobs. Builds reuse the existing shared target; no local build tree, guest disk or corpus was created.

The retained negative chain includes:

- The initial collector fixture used the wrong concurrency profile and failed `Profile`; this is not behavioral RED. Applying `collector-regression.patch` to the two test files at production `10b0e3a0456aa7d40bfbbdefdc89f3420f2047b9` reproduces the corrected test source. That behavioral RED fails `managed resources: Full`; the corrected implementation passes.
- The combined duplex suite initially passed 49 tests and failed the obsolete `SenderReplyReceiveTimeout` diagnostic assertion. Main's cooperative mechanism reports `SenderReplyWait`. The successor restores that exact expectation, retains all timeout/backpressure/frame/cancel/join assertions and removes the unmerged obsolete enum variants. Routing and deadlines are unchanged.
- Initial strict source staging omitted compile-time evidence fixtures. Exact tracked inputs were restored; no product/test assertion was relaxed.
- Producer controls initially passed three and failed two because the temporary ancestor inherited group-write permission under Wick's `0002` umask. Tests now explicitly create that ancestor as `0700`; the `0750` unsafe-parent refusal and existing-file preservation checks remain.
- `collector-stack-refusal.zip` is the original hosted artifact at source `5e1babec`: two collector controls pass and two refuse the existing main-stack envelope. Final qualification records the inherited soft limit, configures 8 MiB and rechecks it. The existing MVM guard remains; no unsupported stack or deadline is admitted.

Original earlier Windows failures and signed/native evidence remain in their dated packets. Superseded CI campaigns were cancelled to release runner capacity after concrete corrections; cancellation is not a passing result.

## Remaining delivery boundary

All five outcomes under #42 remain active: installed mailbox ownership/budgets/shutdown/recovery; persistent final-ACK/retention/expiry and sustained sessions; canonical FlowMux plus separately authorized owner access; actual two-host WAN and supported platform/backend roles; capacity/fairness/saturation and the complete upstream package. Fresh MVM consumer-pin/installed lifecycle qualification is separate from publishing this SpiralCore implementation. Ari must publish the compatible signed runtime; private MVMD integration remains external. Windows native component/client results do not establish Windows MVM hosting. No new final upstream MVM contribution or FIPS/CUI qualification is claimed.

## Native source and sustained qualification

The seven native FlowMux component rows pass on the same reviewed tree. Every row recovers 67,108,864 exact bytes in each direction and transmits 10,790,065 total TLS bytes. Each reports 87 executed component checks, including all 50 library controls; ordered/resource/schema, metadata, owned-storage and platform-specific guards are separately recorded and overlap the component scope. Five timing rows and all six original-parent/canonical-endpoint rows pass. Windows scratch roots are explicitly selected and attested by the fixture; no fallback or security exemption is introduced.

| Native runner | Post-cleanup lifecycle wall (s) | Both-peer process CPU (s) | Setup wall (ms) |
| --- | ---: | ---: | ---: |
| macos26-arm64 | 23.742156 | 22.338144 | 50.081 |
| macos26-intel | 61.178849 | 44.951033 | 50.978 |
| ubuntu24-arm64 | 11.668031 | 13.902357 | 9.947 |
| ubuntu24-x64 | 8.826298 | 13.774521 | 9.025 |
| windows11-arm64 | 116.688586 | 98.531250 | 126.533 |
| windows2022-x64 | 24.717084 | 43.078125 | 94.318 |
| windows2025-x64 | 67.150812 | 86.187500 | 69.038 |

The 19 original native ZIPs total 316,857 bytes. Every outer API digest and all 246 member hashes verify; 180 embedded checksum entries and 1,158 source-file checks verify. Explicit head-checkout jobs use the candidate commit; default checkout jobs execute CI merge commit `6234b65ccbc47ad3e4562fe802c375a6730162cd`. Its Git tree equals the reviewed candidate tree, and its parents are the recorded main and candidate. [Native receipts](native/source-and-measurements.json) retain raw lifecycle lines, platform/source identities and byte counters. Both native Linux collector steps record 16,384 KiB inherited soft stack → 8,192 KiB configured supported stack. These are platform component/client results, not booted MVM backends or WAN. Separate sender/receiver CPU remains unavailable.
