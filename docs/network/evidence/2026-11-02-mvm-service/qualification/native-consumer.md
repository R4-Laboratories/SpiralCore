# Canonical native consumer qualification checkpoint

Date: 2026-11-02. Tracking: #43 under #42. Source was developed on
`codex/spiralcore-mvm-network` against the published MVM fork `5766754`.
Independent focused Rust/security review approved this frozen source for bounded
actual guest qualification; no native guest or WAN result is implied here.

| File under `crates/spiralcore-mvm-live/src/` | SHA-256 |
|---|---|
| `campaign/canonical.rs` | `790983bb36157505554a2a65063beec802f1fe7a73c92bbb93ec325d756651f4` |
| `campaign.rs` | `f233413684a2b55f3b526cffed3df9a730d13febf14902914c8cfcef622affaa` |
| `config.rs` | `e98d63a44e7f102798e1207d52bebd379d680f78856d934097b8b8ca6e8d2ff0` |
| `guest.rs` | `17a9159b30738c1098dd8854d8f812d247850b7910e61661d92289bb45917de7` |
| `orchestrator.rs` | `4f8575b4676417b695b4091e688477a3004be9e3cdfd9dde27483e9226467d29` |

The current consumer requires `canonical-ephemeral-v1` for normal sender and
receiver roles. Absent profiles cannot silently select the historical path;
credential-free disk verification retains its distinct provisioning. Bootstrap
shape checks happen before guest UTC resynchronization; authorization happens
after it. One Call uses one absolute deadline, with refreshed time for
post-compute authorization. Each application Call/Reply has its own encrypted
signed destination receipt; there are four envelopes per Call.

The local implementation tests reported 22 library tests passing and strict
library/test Clippy passing. Focused RED cases covered missing service profiles
and authorization attempted before guest clock resynchronization. Existing
1 MiB campaign tests previously passed 7/7 before the final bounded-profile
regressions were added. These are test results, not performance measurements.

The explicit `native-16m-v1` identity tier has host/tenant/mailbox/sender ceilings
65,536 / 32,768 / 32,768 / 16,384, with a quarter reserved for control **inside**
these totals. It caps one session at 2,048 Calls; library defaults are unchanged.
The bounded raw workload calculation requires 1,947 Calls. A source-reviewed
budget regression checks the complete data/control identity allowance.

An actual release-mode paired Unix-stream test ran on Wick Ubuntu 24.04 x86-64
with Rust 1.97.1, two build jobs and incremental disabled:

```sh
cargo test --locked --release \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host --test campaign \
  qualified_16m_whole_mixed_service_session_reopens_exactly -- --exact --nocapture
```

Result: **1 passed, 0 failed, 7 deliberately filtered**, 6.12 seconds test runtime.
All eight objects reopened exactly; the registered 16 MiB-tier generated trace
is **14,851,384 bytes**, with the other objects defined by the shared fixture.
There were **843 Calls**, **1,686 sent and 1,686 received envelopes** at the
sender endpoint. This exceeds the previous failing capacity threshold near 512
Calls without changing replay retention or bypassing admission.

Recorded sender counters: 99,172,301 envelope bytes sent, 8,156,311 received,
6,752 framing bytes sent and 6,752 received. Recorded sender wall was 5,876,097
microseconds and process CPU 6,793,660 microseconds. **Both endpoint threads run
in the same process**: this CPU includes their concurrent work and is not an
isolated sender CPU measurement, guest benchmark or WAN result. The relatively
large total includes the complete eight-object whole-mixed baseline session.

The initial debug test completed transfers/readbacks but failed its final
metadata assertion because it incorrectly expected a trace larger than 15 MiB.
That failed run lasted 223.05 seconds. The assertion now pins the registered
14,851,384-byte trace; no production code or fidelity check was weakened. Debug
fixture-generation runtime is not being presented as transfer performance.
