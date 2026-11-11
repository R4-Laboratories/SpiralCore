# Current canonical mailbox witness

This standalone Rust consumer pins the current MVM client to `18b58e1ad2262a1c0bfa23d51f8e5679d3376f60`, the focused persistent-mailbox SDK fix on upstream `28afdfe9ef7e36518ca42788d57c54f1d7275f57`. It is designed to exercise one real guest-to-host mailbox delivery through MVM's signed-plan grant, canonical agent port 5257, per-VM host helper and audited host receive. It adds no socket, envelope, codec or guest NIC. Native execution requires Linux/Firecracker. It is an intermediate baseline; it does not qualify object-v1, warm reuse, WAN or fleet capacity.

The producer writes a deterministic 1,024-byte binary body with embedded zeros through current `mvm_agentd::mailbox_outbox::write_outbox`, then waits for 180 seconds so the host can receive and stop the admitted workload. Install it as `/usr/local/bin/spiralcore-mailbox-producer` in a root/current-user-owned rootfs file protected from group/other writes, or an explicit current-user-owned 0700 unpacked rootfs directory. Top-level symlink inputs refuse. The normal SDK accepts that directory and performs its canonical runtime injection and image materialization during launch. The witness does not assemble ext4 itself; SDK materialization is included in measured setup and lifecycle costs.

Before executing, prepare a current source-fingerprinted guest runtime using MVM's `resolve_or_build_source_guest_runtime` and `assemble_runtime_pieces`, a current canonical `mvm-mailbox` helper, a verified kernel cache and an explicit prepared rootfs file or private directory containing the producer. Old SDK agents do not provide this current mailbox implementation. Precharge and record preparation separately from the measured launch. Retain source/runtime/image and binary hash records. For this source-prepared witness, launch must explicitly set both `MVM_RUNTIME_OVERLAY_ACQUIRE_MODE=build` and `MVM_RUNTIME_OVERLAY_SOURCE_ROOT` to the same verified source checkout used for preparation. The source-root variable alone does not select the source runtime: this SDK defaults to published-artifact acquisition and refuses launch if that separate cache is absent. The build-mode launch boundary probes the prepared source cache; this is not permission for an unbounded hidden preparation step.

The Linux caller must also run in its existing legitimate systemd user-session bus context so MVM can attach the signed-plan CPU, memory and task scopes. On Wick, the UID 1000 session is active and `/run/user/1000/bus` belongs to UID 1000; `runuser` can clear its environment, so supply `XDG_RUNTIME_DIR=/run/user/1000` and `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus` explicitly for that same user. A reachable user manager reporting `degraded` is not proof that a particular scope will succeed; the SDK still validates and enforces the required scope. Do not disable quotas, enable lingering, reset unrelated failed units or alter global configuration to bypass a refusal. Attempt 002 reached canonical preparation/materialization but refused the missing session-bus context before a successful launch; it is not a boot or mailbox-transfer result.

Use an isolated, current-user-owned 0700 MVM_HOME with no prior VM or persistent-machine namespace, and a unique unused name beginning `spiralcore-`. The tool creates an exclusive witness claim there and retains it on failure for explicit recovery. The host refuses existing runtime state for that name, launches one 256 MiB/one-vCPU persistent guest with a 180-second TTL with default-deny networking, and retains the successful original LaunchOutcome before authorizing stop/cleanup. A failed launch without that outcome reports `NOT_AUTHORIZED` cleanup and retains uncertain SDK state. After a successful original launch, canonical plan identity and the held original definition fence authorize stop; a failed or changed fence retains state. Cleanup then verifies the same original definition and invokes canonical `remove_machine_with` with `force: false`. Stop refusal retains cleanup. No plan is synthesized or written by this tool. Process observations arm pidfds before stop, hold executable files while hashing, compare the helper against its protected expected file/digest, and recheck Linux start ticks/device/inode and executable identity before CPU sampling; missing CPU or wire measurements are labeled unavailable. Cleanup observations are not an opaque historical retirement certificate.

The following commands belong on the authorized Wick host. Review preparation and source records before the final command. The final command starts a VM.

```sh
export CARGO_TARGET_DIR=/var/tmp/spiralcore-installed-controller.MDjnre/target
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
cargo test --release --locked --manifest-path tools/mvm-canonical-witness/Cargo.toml --lib --test validation --test ownership --test output_failure --test rootfs_input --test canonical_request --test owned_definition --test probe_schedule
cargo build --release --locked --manifest-path tools/mvm-canonical-witness/Cargo.toml
cargo build --release --locked --target x86_64-unknown-linux-musl --manifest-path tools/mvm-canonical-witness/guest/Cargo.toml
# Prepare/verify the rootfs and current runtime through canonical MVM tooling.
XDG_RUNTIME_DIR=/run/user/1000 \
DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
MVM_HOME=/absolute/owned/prepared-home \
MVM_RUNTIME_OVERLAY_ACQUIRE_MODE=build \
MVM_RUNTIME_OVERLAY_SOURCE_ROOT=/absolute/verified-current-mvm-source \
MVM_MAILBOX_PATH=/absolute/current/mvm-mailbox \
"$CARGO_TARGET_DIR/release/spiralcore-mvm-canonical-witness" \
/absolute/prepared-rootfs.ext4 spiralcore-canonical-UNIQUE \
abababababababababababababababababababababababababababababababab \
/absolute/new-report.json
```

Abrupt process death, panic or externally canceled execution does not guarantee a final report or completed cleanup. The retained witness claim and SDK state require explicit recovery in those cases.

The report contains exact expected/received lengths and hashes, direct equality, actual plan identity, setup/receive/lifecycle wall time and host-orchestrator process CPU. This CPU excludes the guest/VMM/helper processes. Raw Linux process tick samples are observations through the last live sample, not full post-exit CPU. No send-only time, energy or wire counts are inferred. Report output is created exclusively with mode 0600 outside MVM_HOME. Preflight errors write explicit failure JSON with no invented receive count, and unsuccessful native work returns a nonzero status. pidfd support is required; there is no process-timer polling fallback. An exited zombie is not a successful reap.

A guest-side refusal or lost verdict currently consumes the upstream outbox. This witness sends one fixed message and records failure rather than claiming reliable retries. Future admitted return/retry work remains governed by the current-native mailbox integration plan.

The [upstream refresh](../../docs/research/2026-11-10-mvm-mailbox-upstream-refresh.md) records Ari's new spool, route, epoch and key-custody changes. Both host and guest use the same source pin. Earlier fixture/preparation evidence in `evidence/` retains its original `241a2fe9` pin. Failed native attempts on that earlier pin did not achieve a message readback; the latest pin requires its own native qualification. A successful component build is not a successful guest boot or transfer.

The persistent lifecycle is required because canonical audited `mailbox::recv` reads the plan persisted by the normal persistent launch. The historical `241a2fe` attempt005 booted after the explicit guest-content `/etc` fixture correction, but its transient request lacked the persisted plan and delivered no bytes. It also retained an immediate helper-reap failure. Neither historical result qualifies this newer source.

Receive waiting uses at most five external probes at offsets 0, 100, 500, 2,000 and 8,000 ms, omitting offsets beyond the original configured receive deadline and refusing late probe starts. Each canonical receive retains its own upstream I/O bounds. The report records actual probe count and labels this a functional waiting mechanism, not performance-qualified event delivery. The SDK currently exposes no canonical blocking/event receive API through this facade. The old 20 ms reconciliation loop is removed. Cleanup still does not substitute a later observation for an original helper-reap receipt.

The current dependency pin uses the ordinary upstream mailbox consumer API and does not require the earlier experimental verified-open API. Every direct MVM dependency in the host and guest manifests shares the same source revision. Current preparation must resolve the matching source fingerprint and build or verify its runtime through the canonical API; the historical EE archive is not relabeled as this source. Native acceptance for this dependency checkpoint remains pending until the exact binaries, runtime and launch preflight are reviewed.
