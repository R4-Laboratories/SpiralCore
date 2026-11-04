# Retained catalog successor — independent Code/Rust/Security review

2026-11-04 — **PASS for the reviewed implemented component and closure of the prior source findings.** No new blocking defect was found in this scoped successor. This is not approval of recovery/adoption, a native receipt, profile activation or full delivery. I performed no source edits, builds, test reruns or cache writes.

The reviewed candidate is `bd83ba66f1bdb1ae538aa4b34a04f4be3afa66c8`, tree `90e2bfe7ebdc4d17a67e4dd92e53755689525f0e`, parent `f7d06da6b4aa148cd348dd91df3342f1bc53e399`. Independently compared all 134 submitted final tested-source entries with actual candidate Git blobs: all match. The byte-check receipt is `successor-source-verified.json` alongside this review. The final four principal delta hashes match the supplied local sources:

- `installed_retained.rs`: `f8a8540bf2de420c261ad53a672be5c32e75db4b5784112faf7b21ca4e5193da`.
- `installed_owner/retained_telemetry.rs`: `5f8e2703a9931aabf7fdbcaa990372d1ff3268614b463bed4dac5c119ef1a985`.
- `installed_owner.rs`: `69f0b9284d62edf1b14ed1d53be0e20bd2e4841542aff4a22a16dcabe175bdfd`.
- `installed_supervisor/retained_tests.rs`: `e03871cb1024d09ecc7ab2ef48f9265ab803e5a40fdceea0a656b3cf0985d090`.

## Prior findings and regression closures

**Catalog continuity:** preparation captures all other held records under the inventory mutex before arming the new record. Binding and sealing likewise capture the complete original record set before updating. Loaded catalog length and full record equality must match that set; a missing catalog can no longer silently become a successful B-only replacement while A remains held. Catalog decoding already enforces unique records, so the equality/count comparison is a complete set match rather than a count-only approximation. Preparation/binding failures close admission while retaining the original table and reservations; a sealing refusal likewise retains its original row and credits rather than refunding debt.

**Durable loss denial after process exit:** before the first catalog/marker effect, the code writes and fsyncs a bounded 112-byte denial beacon through the retained original EX-owner descriptor. It validates the original owner inode against the epoch-bound identity before and after; it neither truncates, renames, replaces nor unlinks that inode. Existing nonempty content must exactly match the expected bounded beacon or preparation refuses. Generic begin/finish/reconcile now deny any nonempty owner file, including unknown or partial content. The state.bin bytes and Core holder digest are unchanged. This closes the missing-catalog/empty-epoch path under the existing exclusive-owner and durable-file assumptions; it grants no admission, deletion or exit authority and supplies no protection against a hostile same-UID truncation of authoritative files. There is no beacon clearing operation in this successor.

**Records descriptor access:** binding now requires F_GETFL to show both O_RDWR and O_APPEND before storing handles or committing Binding/Bound. Sealing repeats that check before reading. Thus a write-only descriptor cannot be successfully bound and later fail the required positional read merely because its access mode was never established. The same original open-file description remains the canonical append writer; `read_at` does not change its shared file offset.

**Directory privacy regression:** binding and sealing repeat the existing private-directory owner/mode/ACL/filesystem validator after fresh state/protection checks. This closes acceptance of the original directory after it changed from private 0700 to 0755. The helper reuses the existing owner checks; it does not chmod, adopt a replacement or loosen the Core guard. Directory/file identity and private file checks remain in place.

The original HOLD report remains historical evidence. The missing-catalog append, actual-process-exit restart and write-only-descriptor REDs, followed by the intermediate 4-PASS/1-directory-FAIL run, must remain attached to their original source/object identities. The new code does not convert any of those failures into a previous success.

## Ownership and remaining limits

The previously reviewed same-pool R+K retention, checked C−R−K disposable subdivision, separate metadata/header reservation and bounded sixteen-entry owner table are preserved. Scope/VM drops do not refund retained debt. Nonempty-table Drop retains the entire original table, actual pins and metadata credit; global finish cannot claim zero usage from missing catalog data. No new authority constructor or deletion capability was introduced.

Lock ordering remains protection → inventory → original catalog owner for these transactions. Sealing releases the table before acquiring protection and revalidates the exact original record after reacquiring it. I found no new accounting-ledger lock spanning filesystem I/O, reverse-order cycle, premature file/credit drop, arithmetic overflow or fixed-record encoding defect in this delta. The held-set clone remains bounded by sixteen records; existing fixed path/catalog limits remain unchanged.

Root reports final focused 5 PASS (including one helper entry and an actual nested restart child), protected full library 152 PASS/2 ignored, and scoped release Clippy `--lib --tests -- -D clippy::all` exit 0 with three unconnected production-constructor warnings. These counts overlap; they are not 157 unique cases, five VMs or zero-warning workspace qualification. I did not rerun those commands. The independent Git-byte check establishes source identity, not additional execution.

Idempotent I/O-failure retry, closed debt adoption, actual positive native-retirement receipt execution, the status/child K coupling, authorized disposition and complete native/WAN acceptance remain open. The sidecar constant is still a declared logical model whose individual writer gates must be enforced before activation, not physical-filesystem/RSS proof. The component's source findings are resolved; none of these pending boundaries is reported as implemented by this PASS.
