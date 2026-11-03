# Plan: qualify portable MVM endpoints and supported platform roles

Authority: [platform contract](../specs/2026-11-02-mvm-platforms.md),
[#46](https://github.com/R4-Laboratories/SpiralCore/issues/46), and final WAN
[#47](https://github.com/R4-Laboratories/SpiralCore/issues/47). The user requires a
complete tested MVM contribution, including supported native Linux/macOS/Windows
endpoint roles. This plan changes no MVM backend or admission architecture.

## 1. Freeze the support contract and execute native baseline jobs

1. Add a checked support manifest with separate host, guest, client, object
   gateway and NIC-attachment roles. Include exact OS/version/architecture/ABI,
   backend, toolchain/features, storage/durability and evidence status. Include
   blockers for supported-but-unavailable libkrun/HVF/KVM environments.
2. Add an opt-in native CI matrix with explicit labels for the proposed supported
   rows. Begin with `windows-2022`, `windows-2025`, `windows-11-arm`,
   `ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-26`, `macos-26-intel`; extend to the
   other exact versions before claiming them. Record image/build identities.
3. On Windows, check/link/run the default canonical MVM core/contract closure and
   the portable mailbox service/receipt tests. Turn off `hardware-sha256` there;
   test identical cryptographic/object vectors with the portable feature profile.
   Native Windows execution is mandatory even though the Linux-to-MSVC check
   already passed. Keep process/job/storage limits bounded.
4. Diagnose observed failures with minimal changes. Do not create a new canonical
   crypto crate merely because the old core has Unix modules: the executed
   default core cross-check already succeeds. If native behavior reveals a
   necessary split, record the failure and justify a single shared implementation
   with existing `mvm_core::mailbox` and key-type compatibility preserved.

## 2. Remove actual endpoint portability gaps with RED/GREEN tests

1. Add native Windows tests for the currently unsupported private store/open and
   durable publish path before implementing it. Test permissive ACLs, reparse
   points, concurrent writers, interruption at publication boundaries, recovery,
   exact lengths/bytes and current authorization. Do not replace security checks
   with no-ops to make tests pass.
2. Isolate platform filesystem primitives used by
   `crates/spiralcore-transfer/src/endpoint.rs`; retain the shared object state
   machine and wire identities. Implement a Windows backend with demonstrated
   native private access control, writer exclusivity, publication and durability.
   Document concrete OS/filesystem semantics and limitations. Keep unsupported
   storage modes fail closed rather than downgrading Published semantics.
3. Replace `crates/spiralcore-gateway/src/measurement.rs` Unix-only process CPU
   measurement behind a tested native platform implementation with equivalent
   units and accounting domain. Do not compare thread CPU against process CPU.
4. Replace Unix-specific input-file identity/open checks in
   `crates/spiralcore-gateway/src/object_service.rs` with equivalent checked native
   operations. Test input replacement, truncation, reparse and race refusal.
5. Keep Linux TUN and Unix native guest socket modules target-specific. Add native
   TCP/mTLS service tests on Windows; merely disabling Unix tests is inadequate.
   A successful remote gateway build does not prove a Windows NIC attachment.

## 3. Lock wire and semantic parity

1. Share immutable canonical envelope/signature/receipt vectors and object-v1
   exact-byte vectors across every native job. Cross-verify each implementation's
   output on other supported architectures, including the software-math
   ternary/Kuramoto/CA fixtures. Preserve original length and trailing zeros.
2. Run the same bounded service suite for Call, Cast, Reply, encrypted admission
   receipt, full queue, stable retry, duplicate/conflict, wrong scope/key/epoch,
   revocation, expiry, cancellation, late delivery, shutdown and reconnect.
3. Run cache commit/reopen/crash cases for every advertised receiver role. Report
   platform feature profiles and acceleration choices with CPU results; never
   attribute hardware differences to a codec change.
4. Fail matrix aggregation on missing artifacts, zero-test targets, a skipped
   required cell, source mismatch or unlinked evidence. A required hardware gate
   can be blocked, but cannot be converted into a passing skip.

## 4. Qualify actual MVM backends and WAN combinations

1. Reuse the existing bounded Wick Linux/KVM test path for current source, then
   actual AArch64 KVM and claimed Apple Silicon HVF/libkrun environments. Local
   macOS currently reports 27.0; record that value and separately obtain any
   required macOS 26 or older libkrun witness.
2. Register every advertised ordered WAN pair and workload before execution.
   Run native Windows/Linux/macOS clients against actual MVM boundaries and the
   cooperating gateway role; include opposite directions and explicit relay
   paths. Do not call a relay path direct or infer an unrun pair from vectors.
3. Use the same authenticated transport and durability for raw, equally tuned
   whole-object zstd and SpiralCore. Include paid setup and receipt traffic,
   retries/repair and relay hops; record exact bytes, sender/receiver CPU, verified
   elapsed/lifecycle and latency distributions. Preserve all losses and failures.
4. Execute restart/reconnect/epoch fencing, three-peer recovery, service
   saturation/isolation and relevant MVM load gates before final acceptance.

## 5. Release and upstream contribution

Publish the support manifest and exact source/binary/patch/run identities with
compact evidence. Update #46/#47 and the parent roadmap at meaningful milestones.
Close only rows/issues whose complete criteria have executed evidence. Submit
the authorized outside-contributor MVM PR after the declared supported matrix
and upstream acceptance criteria are fulfilled; do not advertise a finished
cross-platform MVM integration while a required native/WAN/hardware cell is open.
