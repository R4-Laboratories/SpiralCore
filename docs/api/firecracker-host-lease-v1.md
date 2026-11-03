# Firecracker host lifetime lease v1

Status: lead-approved additive Linux host proposal; focused helper implementation
and tests have started. Driver/daemon integration and native recovery remain open.
Depends on [installed owner v1](installed-host-owner-v1.md). Base MVM source is
`ae12e9deb41cc9a3264196ce22ebc72d4042883d`. This local process resource seam grants
no boot permission, mailbox authority or new guest/network protocol. Full B6a
requires actual holder-death and positive readmission qualification afterward.

## Decision and concrete Rust seam

Use the existing fixed privileged launcher with a typed lease and fixed shell
preflight. A Rust privileged helper would add an executable/dispatch ABI to
deploy and qualify; SCM_RIGHTS would add a rendezvous lifecycle. Neither is
needed for this first Linux profile. No arbitrary callback, executable path,
shell fragment or environment-selected verifier is accepted.

Add `crates/mvm-backends/src/driver/fc/host_lease.rs` beside the concrete FC
driver, with Linux-only public types re-exported by `driver::fc`:

```rust
pub struct HostLeaseExpectation {
    pub state_name: String,
    pub state_bytes: u16,
    pub state_sha256: [u8; 32],
    pub absent_names: [String; 2],
}
pub struct PreparedHostLifetimeLease { /* private owned files + expectation */ }
impl PreparedHostLifetimeLease {
    pub fn bind(directory: std::fs::File, holder: std::fs::File,
                expected: HostLeaseExpectation) -> anyhow::Result<Self>;
}
impl FcDriver {
    pub fn boot_custodied_with_host_lease(
        &self, spec: &VmmSpec, kernel: Option<&VerifiedRestoreFile>,
        disks: PreparedWritableDisks, lease: PreparedHostLifetimeLease,
    ) -> anyhow::Result<Box<dyn RunningVm>>;
}
```

`bind` validates held directory and regular holder descriptors, same trusted
owner/private permissions, distinct objects, known IDs and CLOEXEC; it retains
SH on the exact holder description. Names are distinct single ASCII basenames,
1..64 bytes, drawn from letters/digits/`._-`, excluding `.` and `..`. State byte
bound is 1..4096. The expectation is trusted host configuration, never a remote
request. No data file is fabricated to carry the lease.

SpiralCore exports the two original resources only from a current HolderLease
through an additive typed `prepare_handoff()` result (owned directory/holder
Files, expected length and digest). Retain the originally acquired generation
inside the guard; the export does not choose a newer generation. Perform
fresh canonical storage/key/Active-generation validation while SH-held, then
duplicate its exact pinned holders and installation-directory descriptors. Its
expectation is `state.bin`, exactly176 bytes, SHA-256 of the complete canonical
current record, and absent `.state.intent`/`.state.next`. The existing record's
own domain checksum remains unchanged. This digest is an exact current-state
comparison, not a new signature/credential. The installed-owner component stays
independent of MVM crates; `linux_host.rs` converts its local handoff into the
MVM type. Do not expose raw descriptors as a replacement for boot admission.

## Fixed launcher gate

The pinned `fc/daemon.rs` launcher already writes a private script, launches it
through the canonical scope and sudo (elided for root), records its own PID and
execs Firecracker. Its writable-image handoff reopens procfd objects across
sudo. A reopen is a new open-file description: it does not inherit an old flock.
Preserve existing disk handoff and scope enforcement. Render the same fixed
gate at outer-script entry before its mkdir/rm/socket/console/script mutations,
retaining SH through that work; render it independently in the privileged bash
script before its PID marker/Firecracker exec. This also prevents stale outer
launchers from mutating a reconciled run, rather than gating only VMM exec:

1. Reopen the two live parent objects using integer `/proc/PID/fd/FD` paths.
   Hold source descriptors for the entire launch operation; missing parent or
   failed reopen refuses. Verify the actual reopened directory/holder IDs,
   types, owner and private modes against the prepared values. Never trust only
   a pathname or proc PID. Allocate bounded collision-free descriptor numbers
   with the existing CLOEXEC duplicate pattern; coexist with every disk FD.
2. `/usr/bin/flock --shared --nonblock HOLDER_FD` locks the newly opened holder
   description. Bash retains that same description after the utility exits;
   never pass `--close`, explicitly unlock, or reopen it after this gate.
3. While SH-held, validate the current state through the held directory
   (`/proc/self/fd/DIR_FD/BASENAME`), not an old state-file descriptor. Refuse any
   forbidden marker, including dangling symlinks; refuse a symlink/nonregular,
   wrong-owner/mode/link-count state file. Require its exact expected length.
   Read/hash at most expected length+1 bytes and compare with the prepared
   SHA-256. Current epoch/key/phase/object identities are thereby pinned by
   the exact full-record digest supplied by the canonical owner factory.
4. Use fixed system `stat`, `head`, `sha256sum`, `flock`, `find`, `timeout` and bash
   paths from the admitted Linux profile, not caller-provided tool paths.
   `LC_ALL=C`, strict bash error/pipe handling, validated integer/hex literals
   and quoted basenames prevent shell interpretation. A bounded preflight
   subprocess (<=1 second, SIGKILL on expiry) shares the already opened holder
   description with the launcher; its successful flock therefore remains held
   by the launcher. It must not wrap the eventual Firecracker exec in that
   timeout. Missing tools, timeout or any failed pipeline refuse before exec.
   Check forbidden markers by a successful bounded depth-one directory scan
   through the held descriptor and require empty output. Negated existence
   predicates cannot distinguish an inspection failure from true absence.
   Every security guard explicitly exits on failure; do not rely solely on
   shell errexit behavior for compound conditions.
5. Close the directory descriptor after validation; retain the holder across
   the existing PID marker and exact Firecracker exec. SH persists until final
   close by the actual VMM. Do not introduce a supervising process which closes
   this descriptor while the VMM continues.

Parent/worker SH covers handoff. A successor cannot retire while this launcher
holds SH. If retirement wins before SH acquisition, the freshly opened current
state differs/Clean and the delayed launcher refuses. Core publication requires
holders EX, so state cannot legitimately change during this SH-held comparison.
Trusted same-UID/root namespace mutation remains outside the admitted profile;
current installation ACL/filesystem/private-object checks still apply.

## Driver and consumer integration

- `driver/fc.rs`: preserve original `boot_custodied` semantics via shared
  implementation with no lease. Add the concrete method above; do not extend
  every backend's trait. Carry optional typed lease through the existing cold
  boot helpers, including optional verified-kernel preparation. Lease present
  **always disables idle-pool claiming**, even with zero writable disks.
- `fc/daemon.rs`: compose disk and lease preflights once in the existing script;
  preserve sudo elision, original scope placement, PID capture, API socket
  handling and error cleanup. No sudoers/closefrom change or new NIC route.
- Verify actual VMM holder FD identity before configuring/starting the guest,
  using the existing exact process/socket binding and bounded privileged FD
  inventory mechanism. Missing/inaccessible descriptor refuses activation.
  Inventory is corroboration; native EX-contention tests prove lock lifetime.
- SpiralCore `installed_owner.rs`: current guarded resource export, no alternate
  parser; `linux_host.rs`: select the concrete lease method for installed
  workers, including sender/diskless and verifier; `host.rs`: retain worker SH
  throughout in-process HVF. `orchestrator.rs`/`main.rs`: trusted local epoch
  bootstrap and fixed grants remain required. No installed boot falls back to
  an unleased method. Preserve explicit orchestrator-only development paths.
- Record the additive MVM source delta in `mvm-patch/` preparation/provenance and
  exact file/bundle hashes before building. This proposal does not authorize
  modifying a shared dependency checkout without the lead's selected source path.

## Focused RED and qualification gates

1. Missing concrete API RED, then fixed-script fake exec tests on Linux:
   exact holder retained after parent descriptors close; wrong ID/hash/length,
   marker, stale epoch, absent parent/tool and preflight timeout all execute no
   target and write no successful Firecracker PID marker. Diskless lease path
   must use the gate; disk custody descriptors must still reach the target.
2. Delayed launcher across owner death: retire before child SH, then release
   delay; it refuses obsolete state. If child SH wins, successor EX is Busy.
   Deterministic subprocess barriers, including death during handoff, replace
   sleeps. Inspect actual process identity; never signal an unknown PID.
3. Actual Firecracker: kill only the owned supervisor/worker while VMM remains
   live; successor EX stays Busy. Observe actual owned VMM exit, then prove
   EX, durable retirement and one next epoch. Descriptor inventory alone is
   insufficient. Total native allowance remains2guests/2vCPU/2GiB, no NIC.
4. Run the existing strict cold-boot/disk custody/scope/PID-marker tests with
   lease None, plus lease Some under root and existing sudo profiles. Runtime
   qualification must record tool versions and actual local filesystem.

No native execution, performance win, full B6a, native Windows MVM, B6b or
increased retention is established by the ABI or fake-launcher tests.
