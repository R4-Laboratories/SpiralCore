# Ordered Small cache and authenticated scope

This component port builds on the Windows wake correction in PR #104,
`8535b10d3596c903a1225917326f46fdbe68b69f`. The source originated in installed
candidate `228331df7d8d57d31dd267a246964cb5435043c9`; this extraction excludes
the unrelated Unix publication/directory-sync change and installed-service
work. The [mapping contract](../../../api/mvm-flowmux-v1.md) specifies scope
versions, ordered lanes, retention and failure semantics.

The earlier regression published four distinct 4 KiB applications, admitted
one 2 KiB application, then refused an artifact while that transfer was active.
Before the fix, refusal evicted two verified publications: retained count fell
from three to one and evictions rose from one to three. The test now requires
unchanged publication counts, byte readbacks and replay after refusal.
Victims are planned without mutation and removed only after canonical Begin
admission. Post-admission I/O uncertainty can partially evict and fences without
a reply; rollback and recovery are not claimed.

General profile 1 preserves frozen IPv4 and unscoped IPv6 scope commitments.
Small profile 2 uses distinct v2 serialization and commitment domains, binds the
profile and complete IPv6 address fields, and rejects a capability/enrollment
mismatch before creating the object channel. Existing experimental Small peers
must update together.

## Executed paired TLS check, 2026-11-10

Wick/Linux x64, Rust 1.97.1, locked offline debug build, two build jobs and the
existing shared target. **55 FlowMux tests passed, zero failed or ignored.**
The full list includes exact bytes, authority, half-close, failed-wake and
deadline controls, plus ordered Warm/Runtime/Combined artifact retention.

The 4,096-block test used one persistent paired loopback TLS connection. Both
directions carried 16,384-byte blocks and compared every recovered byte.

| Recorded quantity | Result |
|---|---:|
| Logical bytes, each direction | 67,108,864 |
| Actual TLS bytes A sent / received | 5,395,050 / 5,395,015 |
| Peer sent / received | 5,395,015 / 5,395,050 |
| Setup elapsed | 8,177 microseconds |
| Complete lifecycle elapsed | 15,751,471 microseconds |
| Process user / system CPU | 17.75 / 1.16 seconds |
| Process high-water RSS | 9,356 KiB |
| Declared core pair charge, each endpoint | 8,284,092 bytes |
| Explicit reader stack, each endpoint | 2,097,152 bytes |
| Retained attempts, each receiver | 1 |
| Retained publications, each receiver | 64 |
| Retained private files / bytes, each receiver | 64 / 1,051,712 |
| Evictions, each receiver | 4,032 |

CPU is the Linux process counter delta at 100 ticks/second, covering both
endpoints in this test process. CPU can exceed elapsed time because threads
run concurrently. RSS differs from declared admission charges. Actual TLS
counters are cross-checked with the peer and include setup; they do not count
TCP/IP headers or retransmission. There is no send-only time measurement,
raw/zstd timing control, WAN impairment or booted MVM in this fixture.
These compressible synthetic blocks establish sustained bounded component
operation; they do not establish comparative speed or a fleet capacity envelope.

Warm, Runtime and Combined each additionally preserved exact 8,616-byte
outputs across 129 blocks in each direction while keeping their setup artifacts
pinned through eviction. At completion their cache contained 64 publications,
with respectively one, one and two pinned artifacts.

## Qualification scope and retained failures

The first copied-source build lacked the restored `v1/mod.rs` exports after
an interrupted file sync. Completing the copy and invalidating that source's
Cargo timestamp corrected the build. This was a source-assembly failure, not
a product regression. An all-target shared-core attempt then lacked a schema
fixture and, after that was copied, the workflow fixture used by three workflow
gate tests. Those failures are separate from the focused product checks;
the broad attempt is not reported as a passing full suite. It also encountered
two pre-existing ignored tests.

After restoring the immutable schema/vector/plan fixtures and workflow files,
the focused shared-core command passed **86 tests with zero failures or skips**:
47 library, 21 endpoint, seven ordered-retention, eight Small-resource and
three workflow-gate tests. Both shared-core and FlowMux strict all-target
Clippy passed, and FlowMux formatting passed. The executed product source
was unchanged after those checks.

```sh
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --lib --test endpoint --test ordered_retention --test small_resources \
  --test bounded_selection_workflow
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml \
  --all-targets -- --test-threads=1 --nocapture
cargo +1.97.1 clippy --locked --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --all-targets -- -D warnings
cargo +1.97.1 clippy --locked --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml \
  --all-targets -- -D warnings
```

The hosted FlowMux matrix additionally executes all 15 ordered-retention and
Small-resource regression tests on each supported native process role, with
nonzero enumeration, no skips and strict failure checks. Native results and
merge identity are recorded separately after execution. Installed MVM adoption
of the v2 Small scope, canonical direct/relay WAN, recovery and scale remain
open under [#44](https://github.com/R4-Laboratories/SpiralCore/issues/44),
[#46](https://github.com/R4-Laboratories/SpiralCore/issues/46) and
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).
