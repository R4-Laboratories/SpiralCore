# Native Mac closed-peer timeout correction

Source under test is PR72 head b24a5d0cc5eb771dea51c38e407f13bb3f3d4edb,
with exactly the two-file timeout-fix.patch overlay. The separate active
IPv6/preallocation/SmallV1 work is excluded. The frozen source was exported into
/tmp/spiralcore-flowmux-20261103/mac31/source using git archive for the FlowMux,
transfer, uor-r4-object-store and spiralcore-storage-windows crates. No branch
switch occurred. The original31 code is retained in Git and the original hosted
Mac ARM/Intel job logs are retained verbatim. Both failed at the first reverse
receive with Darwin EINVAL22; the paired scope had6 passes and1 failure.

Actual native local Mac normal release evidence: unchanged original public
half-close test RED, stronger joined-peer/buffered-data/FIN test RED, then the
bounded timeout correction GREEN33 tests,0 failed/ignored/filtered, plus one
explicit zero-test doc scope. Strict all-target Clippy exits0. CPU/wall output
is build/test qualification, not a performance measurement. This establishes
local native Mac component behavior, not new hosted rows or native MVM/WAN.

The new fixture constructs both mappings while live, joins the real peer after
it sends exact reverse bytes and FIN, then verifies both complete records and
cross-peer actual wire counters. A separate policy matrix rejects None, zero,
a longer timeout, failed readback, non-EINVAL and non-Darwin exceptions.
Read/write timeouts are initialized before application traffic. Every later
operation retains its original deadline. Darwin EINVAL is accepted only if the
same socket's actual nonzero timeout is no greater than the fresh requested
slice. All other framing/read/write/timeout errors remain fatal.

Commands used the explicit Rustup Cargo path, jobs2, incremental0 and the
root-authorized shared target /tmp/spiralcore-installed-mac-20261103/target.
Initial setup mistakes (wrong dependency directory and Cargo name shadowing)
and locked offline missing-cc refusal are retained as setup failures, not RED.
The subsequent command used --locked to retrieve existing declared dependencies;
no toolchain or tool was installed and no lock/source was changed by resolution.

```
CARGO_TARGET_DIR=/tmp/spiralcore-installed-mac-20261103/target CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 /Users/casey.allard/.cargo/bin/cargo +1.97.1 test --manifest-path /tmp/spiralcore-flowmux-20261103/mac31/source/crates/spiralcore-mvm-flowmux/Cargo.toml --release --locked --test paired_tcp exact_bidirectional_stream_and_reverse_after_half_close_use_real_tcp -- --exact --nocapture
CARGO_TARGET_DIR=/tmp/spiralcore-installed-mac-20261103/target CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 /Users/casey.allard/.cargo/bin/cargo +1.97.1 test --manifest-path /tmp/spiralcore-flowmux-20261103/mac31/source/crates/spiralcore-mvm-flowmux/Cargo.toml --release --offline --locked --test paired_tcp closed_peer_buffered_reverse_data_and_fin_remain_exact_under_original_deadline -- --exact --nocapture
CARGO_TARGET_DIR=/tmp/spiralcore-installed-mac-20261103/target CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 /Users/casey.allard/.cargo/bin/cargo +1.97.1 test --manifest-path /tmp/spiralcore-flowmux-20261103/mac31/source/crates/spiralcore-mvm-flowmux/Cargo.toml --release --offline --locked -- --nocapture
CARGO_TARGET_DIR=/tmp/spiralcore-installed-mac-20261103/target CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 /Users/casey.allard/.cargo/bin/cargo +1.97.1 clippy --manifest-path /tmp/spiralcore-flowmux-20261103/mac31/source/crates/spiralcore-mvm-flowmux/Cargo.toml --release --offline --locked --all-targets -- -D warnings
```
