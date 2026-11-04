# Executed bounded component commands

All commands ran on Wick in `/var/tmp/spiralcore-flowmux-20261103/source`.
Only the new crate and read-only shared dependency copies were uploaded. Source
and target are task-owned; no shared build target, VM or network policy changed.
The Cargo binary was `/home/callard/.cargo/bin/cargo`, toolchain `+1.97.1`.

Each Cargo test/Clippy command used this exact environment:

```sh
CARGO_TARGET_DIR=/var/tmp/spiralcore-flowmux-20261103/target
CARGO_BUILD_JOBS=2
CARGO_INCREMENTAL=0
```

The initial API RED command, before src/lib.rs existed, was:

```sh
/home/callard/.cargo/bin/cargo +1.97.1 test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline
```

It generated Cargo.lock, then failed target resolution (exit101). Subsequent
test invocations included `--locked`. The following labels are the retained
stdout/stderr file stems; their adjacent `.exit` files contain the actual exit:

| Receipt stem | Cargo arguments following `+1.97.1` |
| --- | --- |
| paired-first | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| tcp-negative | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| tcp-negative-corrected | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| object-api-red | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --test object_tcp --offline --locked |
| object-first | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| object-extended | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| object-extended-corrected | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| transport-extended | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| transport-final | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| namespace-red | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --test object_tcp object_namespace_is_immutable_and_checked_before_read --offline --locked |
| namespace-green | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| component-counts | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked -- --nocapture |
| component-release | test --release --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked -- --nocapture |
| clippy-first | clippy --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --all-targets --offline --locked -- -D warnings |
| clippy-corrected | clippy --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --all-targets --offline --locked -- -D warnings |
| clippy-namespace | clippy --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --all-targets --offline --locked -- -D warnings |

The remote shell redirected both streams to each `.txt`, captured `$?`
immediately and wrote the adjacent `.exit`. The surrounding SSH command for
intentional REDs returned0 to preserve the session; this does not alter the
recorded Cargo exit101. No test result was inferred from that SSH status.

Successive duplex/facade receipts used the same exact manifest/environment:

| Receipt stems | Arguments |
| --- | --- |
| duplex-api-red, duplex-first, duplex-close-corrected | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --test duplex_tcp --offline --locked |
| duplex-expanded, duplex-bounds | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked |
| duplex-combined | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked -- --nocapture |
| duplex-release, mapped-release, component20-release | test --release --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --offline --locked -- --nocapture |
| mapped-api-red, mapped-first, mapped-eof-red, mapped-eof-green, mapped-reentry-red, mapped-reentry-green | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --test mapped_stream --offline --locked |
| duplex-event-bound | test --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --test duplex_tcp thirty_two_interleaved --offline --locked |
| clippy-duplex, clippy-duplex-final, clippy-mapped, clippy-component20 | clippy --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --all-targets --offline --locked -- -D warnings |

The final complete source receipt is `component20-release.txt/.exit`, all20
tests passing. `clippy-component20.txt/.exit` is the same-source strict Clippy.
`source-component20-sha256.txt` pins that exact copied source; all12 own crate
file hashes were independently checked against current local bytes after copy.
The initial 32-event fixture used raw TCP's Nagle default and took1.28s while
still producing the required Limit result. Its final version enables TCP_NODELAY
on that test peer, matching the mapping's socket configuration, rather than
widening the fixed deadline. No timing result here is performance evidence.

Formatting used this command against the standalone workspace only:

```sh
/home/callard/.cargo/bin/cargo +1.97.1 fmt --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml
```

Shared source custody was generated in that copied source using:

```sh
find crates/spiralcore-mvm-flowmux crates/spiralcore-transfer crates/uor-r4-object-store -type f \( -name '*.rs' -o -name Cargo.toml -o -name Cargo.lock \) ! -name '._*' -print0 | sort -z | xargs -0 sha256sum
```

The first tar copy emitted macOS provenance-xattr warnings. Later uploads used
`COPYFILE_DISABLE=1 tar --no-xattrs` and changed only the task's copied source.
No raw profiles, payload stores or executables were copied back to macOS.

## Authenticated component30

Current final commands ran from
`/var/tmp/spiralcore-flowmux-20261103/source/crates/spiralcore-mvm-flowmux`, with
`CARGO_TARGET_DIR=/var/tmp/spiralcore-flowmux-20261103/target`,
`CARGO_BUILD_JOBS=2`, existing `/home/callard/.cargo` Cargo home and no incremental
build. The source and target remain this task's separate owned paths.

```sh
/home/callard/.cargo/bin/cargo +1.97.1 generate-lockfile --offline
/home/callard/.cargo/bin/cargo +1.97.1 test --release --offline --locked -- --nocapture
/home/callard/.cargo/bin/cargo +1.97.1 clippy --all-targets --offline --locked -- -D warnings
```

The first line is current-dependencies-lock, the second is
current-dependencies-complete-release and the third current-dependencies-clippy.
All have captured stdout/stderr and immediate actual exit files. The complete
current shared sources were copied before those last two commands. The older
current-dependencies-release refusal and current-dependencies-release-locked
missing-module failure are retained separately. Earlier authenticated-component
receipts used the historical copied dependency manifest, not these current bytes.

Other focused receipts use the same environment and `test --release --offline
--locked`, with these selected targets/filters:

| Stems | Target/filter |
| --- | --- |
| tls-api-red-deps-present, tls-first | --test tls_carrier |
| full-scope-api-red | --test full_scope |
| scope-policy-api-red-ready | --test scope_policy |
| authenticated-stream-first | --test authenticated_stream -- --nocapture |
| authenticated-direction-red, authenticated-direction-green | --test authenticated_stream authenticated_direction_cannot |
| buffered-revocation-red, buffered-revocation-green | --test authenticated_stream revocation_fences |
| full-facts-first, full-facts-second, authenticated-component-final | full test set, final includes -- --nocapture |

The first tls-api-red also lacked its ring test dependency; it is not a pure
missing-API proof. tls-api-red-deps-present is the isolated E0432 proof. The first
scope-policy-api-red was blocked by an incomplete internal Carrier::peek
refactor; scope-policy-api-red-ready is the isolated E0432 proof. The intermediate
Clippy style failure and raw-peer/header conversion failures remain distinct
from actual behavior REDs. No warning allowance or ignored test was introduced.

## Canonical MVM overlay commands

MVM qualification uses the frozen7d base archive in separate task-owned
`/var/tmp/spiralcore-flowmux-20261103/mvm-source`, overlaid only with authorized
network files and the two matching admitted_plan_json forwarding fields in the
base runner parent. It does not contain the concurrent access-agent changes.
Final integrated exact-source qualification is therefore still required.
Wrapper `/var/tmp/spiralcore-flowmux-20261103/mvm-cargo.sh` exports:

```sh
MVM_HOME=/var/tmp/spiralcore-flowmux-20261103/mvm-home
CARGO_TARGET_DIR=/var/tmp/spiralcore-flowmux-20261103/mvm-target
CARGO_HOME=/home/callard/.cargo
CARGO_BUILD_JOBS=2
CARGO_INCREMENTAL=0
```

It changes into that mvm-source and execs `/home/callard/.cargo/bin/cargo
+1.97.1` with these arguments, retaining output/actual exits in validation/mvm:

```sh
test -p mvm-hostd --bin mvm-network-endpoint selected_payload_mapping_requires --offline --locked
test -p mvm-hostd --lib selected_payload_mapping_never --offline --locked
clippy -p mvm-hostd -p mvm-runtime -p mvm-vmm -p mvm-contract -p mvm-core --lib --bin mvm-network-endpoint --offline --locked -- -D warnings
```

These are projection-missing-plan-red/green and projection-binding-matrix-green;
selected-transform-red/green; and seam-clippy-first. The latter is observed PASS
without compiler/Clippy warnings despite the historical first label.
The complete old workflow and build failures remain in mvm-failures.md and raw
receipts. Baseline canonical policy-hash test passed1/1 with2020 filtered; its
first invocation was captured in the tool output but has no retained standalone
raw log, so that observation is not represented as a sealed command receipt.
