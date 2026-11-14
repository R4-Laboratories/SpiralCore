# Implementing the Unix owned-directory store

**Goal.** A SmallV1 receiver on Linux or macOS can publish, read, reopen and retire its objects under the directory originally admitted by MVM, even if the pathname previously used to reach its parent changes. The project owner approved the architecture on 13 November 2026. Implementation and native qualification are outstanding. [Approved contract](../network/proposals/2026-11-13-unix-owned-store.md) · [Issue #258](https://github.com/R4-Laboratories/SpiralCore/issues/258)

**Baseline.** SpiralCore proposal commit `9a96b169` contains the public design with SHA-256 `a80da5537399d44b4a3c445d06387baf1dfff082d19f170a24f9ce0cfb166961`. The original design digest is in the proposal. On macOS the two installed hostd relay fixtures returned `NotFound` (OS error 2) before the prepared-reader hook. MVM creates and opens the child relative to its parent and subsequently passes a synthesized procfs pathname into the Path endpoint. Correcting the fixture or changing that prefix would leave the path-based reopen and retirement problem intact.

**Architecture.** Rust storage primitives in the transfer crate retain the original directory and lock. The existing endpoint continues to own framing, authorization, metadata parsing, quota decisions and eviction. The MVM resource permit supplies the admitted lane IDs and budgets and retains custody across setup, endpoint drop, reader drain and retirement. Standalone Path constructors on every platform keep their existing bootstrap, checks and behavior. Preserve the Windows NT implementation, object/manifest bytes and negotiated v1 message formats.

**TDD route.** Strict, under the project's focused-tests-first rule and the approved storage contract. Add one focused test per new behavior, retain the original failure, implement the smallest change and run that test again. An unavailable symbol is a useful initial API failure; follow it with semantic controls on real descriptors. The tables below describe small changes. Budget at most 50 new or changed source lines per step, including a test where it fits. Where a row lists multiple controls, implement each control as a separate step with the preceding green state as its dependency. Split test setup or platform implementations into further changes at this same bound. Generated lockfile updates are verified separately. Only integrated, reviewed, tested slices should ship.

## File ownership and private interfaces

Paths in lanes A and B are relative to `crates/spiralcore-transfer/`. Lane A owns `Cargo.toml`, `Cargo.lock`, the declaration in `src/endpoint/storage.rs`, and new `src/endpoint/storage/owned.rs` with its modules `owned/{io,walk,retire,tests}.rs`. Lane B owns `src/endpoint.rs`, `src/v1/mod.rs`, `src/endpoint/metadata_tests.rs`, `src/endpoint/ordered_eviction_tests.rs` and new integration targets `tests/owned_store.rs` and `tests/owned_store_resources.rs`. Lane A can develop primitives while lane B writes public API failures and maps Path call sites. Join on the private signatures before wiring the first mutation. Lane B makes no changes to lane A's files. The existing Unix and Windows Path backends have no implementation edits in this plan.

| Boundary | Private operation and returned evidence |
| --- | --- |
| Original directory | Lease holds parent and child `Arc<File>`, validated leaf, private policy and a slot for the original owner file. `check_named_child() -> Result<()>` compares held child metadata to a no-follow stat relative to the held parent. |
| One namespace | `open_namespace(ns: Id, create: bool) -> Result<File>` enforces frozen lane membership before lookup, bounded preflight before create, and post-open ownership, device and named identity. Directory creation is exclusive under the original writer. |
| One file | `open_read_at(dir: &File, name: &OsStr) -> Result<File>`, `create_new_at(dir, name) -> Result<File>` and `quota_len_at(dir, name) -> Result<u64>` mirror the existing private reads, creates and accounting. Use actual held identity for removals. Metadata accounting never grants content access. |
| One bounded traversal | `walk_entries(lease, visitor) -> Result<()>` streams root and namespace entries, with a shared remaining count. The callback borrows the parent FD and exact single-component name for the current visit. Separate scalar accounting from metadata readback. |
| Publication | `publish_at(namespace, stage, destination, root)` renames under the verified namespace and syncs namespace and root. The endpoint still writes and syncs the staged file and independently verifies the reopened publication. `unlink_checked_at(dir, name, identity)` refuses observed substitutions. |
| Metadata | In `endpoint.rs`, carry a locator for the existing Path or the authorized namespace/object ID pair. Give `read_meta_file` a checked handle and the object identity inferred from the original filename. Keep exactly one header/body decoder and digest verifier. An owned locator never materializes an ambient path. |

## Lane A — descriptor primitives and exclusion

Implement rows in order unless an independent dependency is stated. All FILE operations below are local to this new route.

| Step | File | Failure to retain, then smallest implementation |
| --- | --- | --- |
| A1 | `owned.rs`, `owned/tests.rs`, declaration in `storage.rs` | A default SmallV1 walk offered 101 observations must refuse. Add closed `OwnedLaneNamespaces::{One,Two}` and `OwnedStorePolicy::small_v1`; accept 102 funded observations for L=101. Separately reject duplicate IDs, overflow, unsupported limits and a third lane at the caller. Derive L as 2 markers + 1 profile stage + 2 directories + publication and state limits. |
| A2 | `Cargo.toml`, `Cargo.lock` | Capture normal target graphs before adding target-specific rustix fs/process support for Linux/macOS. Resolve through normal Cargo on the existing target/cache. Compare the resolved version/features to MVM's 1.1.6 and check the Windows normal graph. |
| A3 | `owned/io.rs`, `owned/tests.rs` | Give capture a regular file, insecure child, symlink name or replaced child. Check direct component grammar, fstat and parent-relative statat on the real tuple; refuse mismatches. Then rename the original parent, install a sentinel at its old path and require capture of the held original. |
| A4 | `owned.rs`, `owned/tests.rs` | Capture a fresh private tuple and assert its directory remains empty. Construct cloneable lease over immutable originals, policy and empty acquisition state. It performs reads only. |
| A5 | `owned/io.rs` | FIFO, symlink and hardlink candidates must refuse before content I/O. Apply NOFOLLOW, NONBLOCK, CLOEXEC, UID, mode, regular type and single-link checks; add DIRECTORY for directory opens. Reject observed device changes; activation requires an owner-controlled private mount tree excluding nested/substitutable mounts. |
| A6 | `owned.rs` | Attach a lease clone in a fixture permit. Inject failure immediately after opening .owner. Place the actual returned FD into the preallocated retained slot before any further fallible work. Preserve the opened descriptor and mark the single-use claim quarantined on error/unwind. |
| A7 | `owned.rs` | Inject failure immediately after locking that retained FD. A separate real open must contend while either permit or endpoint retains the original. Record the lock as attained before validating the named inode or profile. A failed lock must never authorize deletion. Preserve initialized-only bounded wait and the existing 50 ms maximum. |
| A8 | `owned/walk.rs` | Two walks must each see the first and last entry. Open `.` relative to the held directory with fresh open-file description, then use `Dir::new`. Recheck named/held identity and link count before treating EOF as completion, including an unlinked namespace. |
| A9 | `owned/walk.rs` | At L entries finish; at L+1 refuse with `Error::Limit` before classifying the lookahead. Count empty namespace names, unknown files, dotnames and stages before filtering. Ignore only `.` and `..`. Refuse an unlisted well-formed namespace before descent. |
| A10 | `owned/io.rs` | Require private exclusive profile/object staging, checked rename, file/namespace/root sync and identity-checked stage removal under substituted names. Test each operation separately. Keep root marker and namespace stage grammars distinct. |
| A11 | `owned/retire.rs` | Endpoint clone still live or original child replaced must refuse before unlink. Preflight all entries under locked owner. Reject unknown content, hostile links, mount changes and excess entries before destructive retirement. |
| A12 | `owned/retire.rs` | Remove verified data/stages, namespaces and profile first; .owner last, retaining the locked inode through parent removal. Inject a partial failure and require sticky custody at the attained phase. Cap one cleanup attempt at L+2 bounded walks, or 103 × 102 observations at default. A stat/unlink pair cannot exclude independent same-UID concurrent renames; this boundary depends on actual exclusive lifecycle and mount ownership. |

## Lane B — one endpoint implementation

B1 can begin alongside A1. B2–B8 require the corresponding primitive and its passing controls. Keep new APIs unavailable to installed dispatch until the entire FD operation set and the resource gate pass.

| Step | File | Failure to retain, then smallest implementation |
| --- | --- | --- |
| B1 | `tests/owned_store.rs`, `src/v1/mod.rs` | Express capture → attach permit → v1 open → exact trailing-NUL readback on the real public surface. Re-export only approved policy, lane and lease types on Linux/macOS. List and run the actual named test. |
| B2 | `src/endpoint.rs`, `src/v1/mod.rs` | Add the owned variant for root/lock custody and v1 `open_at_with_resource_profile`. Reject a mismatched profile/limits or duplicate endpoint claim. Reuse existing negotiated checks and common endpoint state construction; keep old Path construction and its owner FD on that arm. |
| B3 | `src/endpoint.rs` | For owned setup, validate initialized marker bytes and original lock, or perform the fresh-root check before exclusive marker creation. Implement verified stage recovery as a separate bounded step. Inject corruption/truncation and refuse without adopting foreign content. |
| B4 | `src/endpoint.rs`, `src/endpoint/metadata_tests.rs` | Adapt `open_publication`, `find_base` and `read_meta_file` separately to checked locators and files. Reject wrong ID, altered length/header and digest on independent readback. Exercise the former-parent sentinel. |
| B5 | `src/endpoint.rs` | Route both calls to `disk_usage` to bounded FD traversal in the owned arm. Add an eligible unknown-stem .object between Begin and Commit; confirm fresh scoped/global byte and count admission, no published result on overflow, and retained original data. |
| B6 | `src/endpoint.rs`, `tests/owned_store.rs` | Move each of namespace ensure, exclusive stage write/sync, rename/sync and failure cleanup to the owned primitive at its existing publication point. Add one failing control at each boundary. Compare cold publication and cache hit with the exact same binary body. |
| B7 | `src/endpoint.rs`, `src/endpoint/ordered_eviction_tests.rs` | Convert ordered scan, then ordered read, then single/multi-victim prevalidation and removal, one method per step. Keep active pins, fresh authority, content/length/digest and checked arithmetic. Exercise changed victim, active pin and revoked read; accepted siblings must survive refusal. |
| B8 | `tests/owned_store.rs` | Use a subprocess under the existing test harness to interrupt staged publication before rename, then after rename. Reopen under fresh authority and verify the original bytes or recognized recoverable stage according to that exact boundary. Run Path regressions including ordered retention and original Windows storage gates. |

## Resource proof and MVM handoff

Publish the reviewed SC primitives and endpoint as one coherent dependency revision before changing the MVM manifests. MVM paths below are relative to the contributor repository. Work on the existing resource owner and canonical relay.

| Step | File | Failure to retain, then smallest implementation |
| --- | --- | --- |
| R1 | SC `tests/owned_store_resources.rs` | Observe one serialized store at deterministic pause points using a separately accounted observer. Then exercise maximum declared concurrency. Require shared parent once, two persistent FDs per store, at most four transient FDs per serialized store and release/quarantine counts. Add FD and memory observations separately. |
| R2 | MVM `crates/mvm-hostd/src/supervisor/flowmux/spiralcore_payload.rs` | Before enabling, derive the pinned iterator/name/vector/stack allocation bound for each supported platform; debit it and FDs at actual concurrency in the original pool. Insufficient funding must refuse before creation. If the allocator/libc bound cannot be established, retain refusal on that platform until enforceable memory funding covers it. Document measured RSS separately from the bound. |
| M1 | MVM `crates/mvm-hostd/Cargo.toml`, `Cargo.lock` | Advance both `spiralcore-transfer` and `spiralcore-mvm-flowmux` from 7458294c9 to the same reviewed public revision. Verify normal Cargo package identities at the actual installed features and target; preserve exact prior-run pins. |
| M2 | MVM `crates/mvm-hostd/src/supervisor/flowmux/spiralcore_payload.rs` | Change the installed `OwnedPayloadStore` arm to keep the original parent Arc, validated leaf and child Arc immediately after original creation. Keep existing qualification-only Path storage on its own arm. Cover partial creation with retained debit. |
| M3 | MVM `crates/mvm-hostd/src/supervisor/flowmux/spiralcore_payload/installed.rs` | In `reserve`/ `prepare`, retain funded visits and bind one or two exact authenticated `Context::namespace()` IDs. Call lease capture, attach its clone to the original permit, then call endpoint open. Remove the installed synthesized path. Verify original scope checks, provisional retention and reader registration still run. |
| M4 | MVM `crates/mvm-hostd/src/supervisor/flowmux/spiralcore_payload.rs` | On confirmed join with exclusive lease, retire verified contents, recheck named child, rmdir relative to original parent, fsync parent and confirm absence. Test each transition separately. Keep debit/FDs on poison, contention, failed fsync or substitution. |
| M5 | MVM `crates/mvm-hostd/src/supervisor/flowmux/owned_relays_tests.rs`, `crates/mvm-hostd/tests/support/installed_factory.rs` | Execute both original close and poison fixture selectors. The prepared-reader hook must actually be reached. Close collects the original reader; poison retains it without an asserted drain. Preserve refusal controls, absolute deadlines and the previous macOS errors. |

## Verification and delivery

Run each new test by its full discovered libtest name and require a nonzero exact execution count. Use already assigned native targets. Examples from the respective repository roots follow.

```sh
cargo test --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --test owned_store
cargo test --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --test owned_store_resources -- --exact maximal_admitted_owned_store_lifecycle --nocapture
cargo test --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets
cargo clippy --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets -- -D warnings
cargo test --locked --offline -p mvm-hostd --lib supervisor::flowmux::owned_relays::custody_tests:: -- --nocapture
```

Compare baseline and candidate `cargo tree --locked --offline -e normal,features --target "$T"` in the transfer crate and for the installed MVM host package at identical features. Use Linux and macOS native fixtures, and the existing native Windows Path regressions. Record exact pins, closure changes, binary hashes, original resource debit, FD peaks/release, memory, CPU, elapsed time, byte counts and exact readback for the operation actually measured. Retain failed results. A subprocess termination does not exercise power-loss recovery. Native component tests do not establish guest, fleet or WAN behavior.

After independent code and security review, link the tested SC change and evidence to #258, publish it through a PR, then qualify the exact MVM consumer revision. Update current status with implemented versus qualified platform roles; keep unsupported roles and scale/WAN gates open. The project owner's approval covers execution of this contract. No implementation or native result is recorded by this plan.
