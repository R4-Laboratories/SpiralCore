# Installed host owner v1: local lifecycle contract

Lead-approved 2026-11-03 against main7c220cc8, under user-authorized autonomous B6a/B6b delivery. This is local resource ownership, not a canonical MVM envelope, mailbox authority or new network protocol. The [design](../specs/2026-11-03-installed-host-owner-design.md) defines the full end state; the component below is its first prerequisite, not completion of B6a.

## Owner and epoch API

A Unix-only production module `crates/spiralcore-mvm-live/src/installed_owner.rs` exposes these semantics, using private fields and typed errors:

```rust
pub struct InstallationKey { pub installation: [u8; 32], pub profile: [u8; 32] }
pub struct Epoch(/* private u64 */);
pub enum Phase { Clean, Active }
pub struct OwnerSnapshot { pub phase: Phase, pub epoch: u64 }
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
    pub fn begin(&mut self) -> Result<Epoch, OwnerError>;
    pub fn finish(&mut self, epoch: Epoch) -> Result<(), OwnerError>;
    pub fn reconcile(&mut self) -> Result<Recovery, OwnerError>;
}
impl HolderLease {
    pub fn acquire(root: &std::path::Path, key: InstallationKey, epoch: Epoch)
        -> Result<Self, OwnerError>;
}
```

Key values come from trusted installation provisioning, never network input; zero key components refuse. They bind the installation and immutable configured profile. They are not credentials, signed guest capabilities or authority to start a VM. Only an acquired owner can issue an Epoch; copies carry identity, not arbitrary child admission. The concrete installed launcher must still reserve the existing host/tenant/worker grants.

`initialize` exclusively creates a new0700 directory, stable0600 regular `owner.lock` and `holders.lock`, and a bounded canonical binary Clean record at epoch0. Existing installations are opened, never recreated or silently reset. Both lock inodes stay unchanged forever. Refuse symlinks, hardlinked/nonregular/foreign-owned or group/other-accessible objects and identity mismatches. Hold a directory descriptor; all child accesses are descriptor-relative and final symlinks are refused. Bind directory and lock device/inode identities in the record. Revalidate path/object identities before transitions; replacement fences. Trusted same-UID/root provisioning must not forge records or bypass the installed launcher.

`open` takes nonblocking owner EX; a competitor gets Busy without changes. It reads the bounded state; missing, torn, unsupported, oversized or mismatched state refuses. Active may open for reconciliation/diagnosis, never immediate admission. `begin` requires Clean, takes holders EX, checks generation increment, durably publishes Active before returning the new Epoch, then retains an independently opened SH lease for the supervisor. It never resets retained epochs. If holders EX is unavailable, refuse before publication/activation. All actual resource holders acquire SH and freshly validate the same Active epoch while SH-held before boot. Epoch mismatch or Clean refuses without resource activation.

`finish` requires the current Active epoch, drops only its own supervisor SH and obtains holders EX. Live independently opened holders cause Busy and retain durable Active; another finish attempt is allowed. While EX-held, durably publish Clean with unchanged generation. `reconcile` on Active follows the same EX requirement; only then retire it to Clean. It emits Retired, not remote mailbox rollback. A delayed helper acquires SH either before retirement (blocking it) or after it (observes Clean/new epoch and refuses). AlreadyClean does not erase history. No TTL, PID, timeout or Child outcome substitutes for actual lease exclusion.

State is one fixed bounded record, plus at most one private publication temporary. Integers use checked canonical encoding; unknown phases/reserved fields, trailing bytes and generation exhaustion refuse. Publish through private exclusive temp, file sync, atomic descriptor-relative rename, directory sync. A leftover temp or any uncertain publication fences admission; it is not silently removed. Drop releases owned descriptors only and NEVER marks Clean. No explicit unlock on HolderLease drop. Failure paths retain original identity; inability to confirm durability is not success.

## Qualification boundaries

A focused Cargo harness may compile this exact production file as its library with libc only, avoiding the full MVM dependency graph during initial TDD. It contains no duplicate implementation. Main live-crate export/integration, full release/Clippy, actual worker/VMM handoff, native guests and installed default-path closure remain separate gates. Component lease tests are not actual MVM recovery or host-wide plaintext/cache enforcement. Preserve strict/orchestrator paths and report scope until full B6a qualifies; B6b remains required.

RED before product: absent API; two subprocess owners/same stable inode; nonterminal owner death with live SH holder; blocked reconciliation followed by holder exit and exactly one new epoch; stale late-holder refusal; key/profile/object replacement; malformed state; occupied Clean holder; drop never Clean; bounded state/durability faults. Exact subprocess synchronization and watchdogs replace arbitrary sleeps. Record component CPU/wall and memory separately from native transfer measurements. Shared holder exclusion is only a positive resource proof after every installed boot path actually retains a lease; integration must not infer that from this module alone.
