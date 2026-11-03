# Installed host owner v1: local lifecycle contract

Lead-approved 2026-11-03 against main7c220cc8, under user-authorized autonomous B6a/B6b delivery. This is local resource ownership, not a canonical MVM envelope, mailbox authority or new network protocol. The [design](../specs/2026-11-03-installed-host-owner-design.md) defines the full end state; the component below is its first prerequisite, not completion of B6a.

## Owner and epoch API

A Unix-only production module `crates/spiralcore-mvm-live/src/installed_owner.rs` exposes these semantics, using private fields and typed errors:

```rust
pub struct InstallationKey { pub installation: [u8; 32], pub profile: [u8; 32] }
pub struct Epoch(/* private installation/profile/generation binding */);
impl Epoch { pub fn generation(&self) -> u64; }
pub enum Phase { Clean, Active }
pub struct OwnerSnapshot { pub phase: Phase, pub epoch: u64 }
pub struct InstallationBinding {
    pub root: std::path::PathBuf,
    pub key: InstallationKey,
    pub generation: u64,
}
pub enum Recovery { AlreadyClean, Retired { epoch: u64 } }
pub enum OwnerError {
    Busy, RecoveryRequired, InvalidStorage, StaleEpoch, EpochExhausted,
    Fenced, Io(std::io::Error),
}
pub struct InstallationOwner { /* non-Clone owner EX; validated directory */ }
pub struct HolderLease { /* non-Clone holder SH; no explicit unlock */ }
impl InstallationOwner {
    pub fn initialize(root: &std::path::Path, key: InstallationKey)
        -> Result<Self, OwnerError>;
    pub fn open(root: &std::path::Path, key: InstallationKey)
        -> Result<Self, OwnerError>;
    pub fn snapshot(&self) -> Result<OwnerSnapshot, OwnerError>;
    pub fn profile_hash(&self) -> Result<[u8; 32], OwnerError>;
    pub fn active_binding(&self, expected: Epoch) -> Result<InstallationBinding, OwnerError>;
    pub fn begin(&mut self) -> Result<Epoch, OwnerError>;
    pub fn finish(&mut self, epoch: Epoch) -> Result<(), OwnerError>;
    pub fn reconcile(&mut self) -> Result<Recovery, OwnerError>;
}
impl HolderLease {
    pub fn acquire(root: &std::path::Path, key: InstallationKey, expected_generation: u64)
        -> Result<Self, OwnerError>;
    pub fn prepare_handoff(&self) -> Result<HolderHandoff, OwnerError>;
}
pub struct HolderHandoff {
    pub directory: std::fs::File,
    pub holder: std::fs::File,
    pub state_bytes: u16,
    pub state_sha256: [u8; 32],
}
```

Key values come from trusted installation provisioning, never network input; zero key components refuse. They bind the installation and immutable configured profile. They are not credentials, signed guest capabilities or authority to start a VM. Only an acquired owner issues an Epoch. Its numeric generation may cross a trusted local worker pipe; HolderLease::acquire validates that number against the original key/current Active record while SH-held. A number alone grants no boot permission; copies carry identity, not arbitrary child admission. The concrete installed launcher must still reserve the existing host/tenant/worker grants.

`initialize` requires an existing private0700 same-owner parent and synchronizes its new directory entry before success. It exclusively creates a new0700 directory, stable0600 regular `owner.lock` and `holders.lock`, and a bounded canonical binary Clean record at epoch0. Existing installations are opened, never recreated or silently reset. Both lock inodes stay unchanged forever. Refuse symlinks, hardlinked/nonregular/foreign-owned or group/other-accessible objects and identity mismatches. Hold a directory descriptor; all child accesses are descriptor-relative and final symlinks are refused. Bind directory and lock device/inode identities in the record. Revalidate path/object identities before transitions; replacement fences. Trusted same-UID/root provisioning must not forge records or bypass the installed launcher.

`open` takes nonblocking owner EX; a competitor gets Busy without changes. It reads the bounded state; missing, torn, unsupported, oversized or mismatched state refuses. Active may open for reconciliation/diagnosis, never immediate admission. `begin` requires Clean, takes holders EX, checks generation increment, durably publishes Active before returning the new Epoch, then retains an independently opened SH lease for the supervisor. It never resets retained epochs. If holders EX is unavailable, refuse before publication/activation. All actual resource holders acquire SH and freshly validate the same Active epoch while SH-held before boot. Epoch mismatch or Clean refuses without resource activation.

`finish` requires the current Active epoch, drops only its own supervisor SH and obtains holders EX. Live independently opened holders cause Busy and retain durable Active; another finish attempt is allowed. While EX-held, durably publish Clean with unchanged generation. `reconcile` on Active follows the same EX requirement; only then retire it to Clean. It emits Retired, not remote mailbox rollback. A delayed helper acquires SH either before retirement (blocking it) or after it (observes Clean/new epoch and refuses). AlreadyClean does not erase history. No TTL, PID, timeout or Child outcome substitutes for actual lease exclusion.

State is one fixed bounded record, plus at most one private publication temporary and one bounded private intent marker. Integers use checked canonical encoding; unknown phases/reserved fields, trailing bytes and generation exhaustion refuse. Publish only after exclusively creating `.state.intent`, writing its bounded intent and syncing both intent file and directory. Abort before any state mutation if that first barrier fails. Keep the marker through private exclusive temp write, file sync, atomic descriptor-relative rename and state-directory sync. A leftover intent/temp or uncertain STATE publication fences admission; neither is silently removed. Only after the state-directory barrier succeeds may the intent be unlinked, followed by cleanup-directory sync. A post-commit unlink/cleanup-sync error still errors/fences the current owner; a successor may accept an absent marker, because its removal can happen only after the state barrier succeeded. A marker that reappears after power loss conservatively fences. No claim that an unconfirmed initial intent creation survives power loss. Drop releases owned descriptors only and NEVER marks Clean. No explicit unlock on HolderLease drop. Failure paths retain original identity; inability to confirm durability is not success.

`active_binding` is a fresh metadata snapshot for library consumers that already
hold an owner and its original Epoch. It reuses canonical current-state validation
and requires the exact original key/generation and Active phase. It returns
provisioning facts, without an owner, lock file, Arc, mutex or boot permission.
It allows consumers to construct local worker messages without duplicating the
state parser; live-controller authorization and actual holder acquisition are
still separate requirements. Caller changes to returned values cannot change
the installation or authorize a resource.

## Holder handoff extension

`prepare_handoff` is an additive local resource export, planned after component
qualification. It freshly checks the same canonical Active generation originally
acquired by this guard while SH remains held, validates current namespace/object
identity, then duplicates the exact held directory and holder descriptions with
CLOEXEC. It returns exactly176 as state_bytes and full SHA-256 of the complete
verified canonical176-byte record as state_sha256. This supplies the expected
current state for the MVM launcher; the record's domain checksum is unchanged.
Wrong generation/phase, corruption, storage replacement or duplicate failure
returns an error without a handoff.

The non-Clone result owns its Files. Closing the original guard must not release
SH while its duplicated holder remains. No explicit unlock is provided. Owned
files support the typed MVM binding, not boot admission; trusted same-UID/root
callers must still honor installed worker/resource grants. The guard remembers
its original generation rather than exporting whichever epoch is present. No
keys, payloads or network envelopes are serialized. This extension needs its own
RED/GREEN and consumer evidence; the earlier54 component entries do not prove it.

## Qualification boundaries

A focused Cargo harness may compile this exact production file as its library with libc and sha2 only, avoiding the full MVM dependency graph during initial TDD. It contains no duplicate implementation. Main live-crate export/integration, full release/Clippy, actual worker/VMM handoff, native guests and installed default-path closure remain separate gates. Component lease tests are not actual MVM recovery or host-wide plaintext/cache enforcement. Preserve strict/orchestrator paths and report scope until full B6a qualifies; B6b remains required.

RED before product: absent API; two subprocess owners/same stable inode; nonterminal owner death with live SH holder; blocked reconciliation followed by holder exit and exactly one new epoch; stale late-holder refusal; key/profile/object replacement; malformed state; occupied Clean holder; drop never Clean; bounded state/durability faults. Exact subprocess synchronization and watchdogs replace arbitrary sleeps. Record component CPU/wall and memory separately from native transfer measurements. Shared holder exclusion is only a positive resource proof after every installed boot path actually retains a lease; integration must not infer that from this module alone.

## Fixed state representation

The first implementation stores exactly176 bytes: magic `SCOWNER1` (8), phase byte (0Clean/1Active) plus7 zero reserved bytes, big-endian generationu64, installation32, profile32, directory(dev,inode) two u64, owner-lock(dev,inode) two u64, holders-lock(dev,inode) two u64, trailer `ENDOWN01` (8), then SHA-256(domain `spiralcore/installed-owner/v1\0` plus preceding144 bytes). Check exact length, reserved bytes, phase, markers, digest, key and all identities before use. This checksum detects damaged storage; it is not a MAC against trusted same-UID/root operators and makes no FIPS claim. A stale `.state.next` or `.state.intent` fences open; no automatic deletion/reset. State file is `state.bin`, stable locks named above. No payload, key material or guest capability is stored.

## Exact-descriptor binding

Path validation alone does not bind a later open: a temporary A→B→A replacement must not make a lease lock B while subsequent checks see A. Every actual opened holder descriptor must match the pinned holders device/inode before locking; freshly validate its namespace after locking. The lease owns that exact checked descriptor. Apply to both SH and EX paths, preserving refuse-without-state-change semantics. Test-only deterministic open/restore and durability fault hooks exist only under cfg(test); release builds contain neither hooks nor global test controls.

## Filesystem admission and macOS ACLs

The next Linux integration seam is specified separately in
[Firecracker host lifetime lease v1](firecracker-host-lease-v1.md); it is proposed
for lead review and does not turn this component into qualified VMM recovery.

V1 rejects unknown/network filesystem types. Linux local admission candidates are ext4, XFS, Btrfs, tmpfs and overlayfs; macOS requires a local APFS or HFS filesystem. This allowlist is not executed qualification of every filesystem; record the actual native filesystem in each result. Unsupported platforms/filesystems refuse rather than fall back to a different lock scope. macOS mode0700/0600 does not rule out extended ACL grants: this initial profile therefore rejects any extended ACL entry on installation directories/files, including its required private parent. It uses descriptor ACL inspection; it does not silently remove ACLs or attempt fine-grained permission inference. Linux POSIX ACL effective group/named-user permissions are masked by the already-required zero group mode bits; NFSv4/network ACL profiles are not admitted. Trusted provisioning must keep path ancestors fixed; final-component NOFOLLOW and held-directory/object identities do not claim hostile same-UID/root namespace protection.
