# Resident service custody and real resource drain

2026-11-04. Linux x86_64, Rust 1.97.1. **Implemented and tested as a lifecycle
component; no MVM guest or transfer was run by these tests.**

The resident owns one original `LocalBackend` and two fixed slots. It installs
the actual parent provider once. Each slot retains its original parent owner,
canonical launch/stop future and any returned certificate/outcome/receipt.
Dropping a driver's await preserves the operation. Stop requests latch intent;
cleanup uses that same backend and original binding. A caught operation panic
permanently fences the resident and quarantines the whole original owner before
resuming the panic. It cannot be polled, refunded or reported finished afterward.

Hostd's install-once registry now holds a Weak reference. Each admission call
upgrades and retains the SAME provider while it runs. The resident keeps the
strong owner through shutdown. Expiry refuses and the consumed registry cannot
accept a replacement. This removes a genuine permanent 4 KiB ledger charge:
clean shutdown releases real owners instead of exempting their bookkeeping.

Known inline table/Arc storage is reserved before allocation. The concrete
maximum of launch, outcome-stop and pending-stop future storage is reserved
before arming a slot and retained through cleanup. Shutdown needs no fresh
operation-storage admission after close. These are requested Rust storage
claims; captured heaps, bootstrap/configuration, runtime, native memory and
whole-process RSS remain separate integration/model gates.

## Executed results

| Scope | Observed result | What it proves |
|---|---|---|
| Original strong-registry regression | Build 0, test 101 | The permanent registration retained the last actual provider and prevented resource drain |
| Weak-registry successor | 1 passed | Real 4 KiB stays charged during a scoped call, then releases; expired lookup/reinstall refuse and original global finish succeeds |
| Existing cancellation/refusal harnesses on successor graph | 3 unused +1 Mock-refusal passed | Same-owner cancellation and refusal boundaries survive the registry change |
| Missing resident API | E0432 | The additive facade was absent before implementation |
| Initial resident policy/external cases | 2 policy +1 external passed | Ordinary driver cancellation, fair polling, whole-owner Drop custody and real two-slot unused completion |
| Caught-operation panic regression | Test 101 | An embedding could previously catch a panic and admit another slot |
| Final resident policy cases | 3 passed | Driver cancellation; caught-panic fence; whole-owner Drop quarantine with exact Busy exclusion |
| Final external resident case | 1 passed | Two actual native-profile grants, third refusal, real canonical unsupported-backend/unused certificates, charges 2 → 1 → 0, stale IDs, close and genuine global finish |
| Final release builds | Both exit 0 | Actual locked public dependency graph builds the tested binaries |
| Clippy `-D clippy::all` | Exit 0 | Final component passes that lint command; six existing unconnected endpoint/disk dead-code warnings remain recorded |

These scopes overlap. They are not a summed number of distinct tests, a full
crate suite, booted guests, or positive native retirement. The external fixture
uses the actual canonical client with an unsupported Mock backend; it never
manufactures a native receipt. The private controlled futures test driver
policy; their extra wrappers do not qualify production allocation bounds.

Final product source SHA-256:
`d09fb8eac46a48246a894645a2ac763e810196dead0f5f481e4f112bcacb998e`.
Final private tests: `3f58b74f1b3524533afa45c1cfa9cdde11b7ff9d816863795b503fcc4c2cf824`.
The external test is independently authored. Nine selected source inputs and
two protected final executables match before/after execution and lint. Earlier
registry tests separately retain their nine-input/three-executable identities.

The actual graph has one MVM revision
`f708539f0c2317acf2e74f8f7833d6072a5abafe`, shared core
`ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1` and adapter
`dee6e52a712c7b3dc12ea09be8bc16d3754b08f7`; no dependency path overrides.
The harness selects its actual local Live crate. Tokio 1.53.3 / Mio 1.2.1 remain
fixed. [Resolved versions/features](resolved-features.json) are recorded:
this public graph's serde_json 1.0.154 is **not** the separate intermediate
RawValue/bootstrap model's serde_json 1.0.151 closure.

## Records and limitations

- [Exact commands and measurement scope](commands.json)
- [Development/RED receipts](development-receipts.tar.gz), SHA-256
  `720e246cbcb3b5effb07016b90b5aa36b8a8cc308ed34e12374be11a8669dd7d`
- [Final receipts](final-receipts.tar.gz), SHA-256
  `0329f78131f2e126be8ebc27e933891650ad76ca40ab90e0d4a88040e3677fbb`
- [Nine source identities](source-before.json)
- [Resident Code/Rust/Security review, including original HOLD](../2026-11-04-resident-parent/reviews/installed-resident-code-rust-security.md)
- [Weak-registry review](../2026-11-04-resident-parent/reviews/weak-parent-registry-code-rust-security.md)
- [Resident API/implementation contract](../../../api/installed-resident-service-v1.md)

The initial Clippy failure is retained. Two private fixed-slot enum allowances
preserve prepaid inline custody; boxing solely for the lint would introduce
cleanup-time allocation. The other corrections preserve condition evaluation
order. The caught-panic failure and earlier source HOLD remain alongside their
successor evidence.

There is no new byte/CPU/transfer-time/RSS/energy result here. Logical guest
reservation of 2 GiB is not measured RSS or two booted guests. Compilation time
and test-runner elapsed time are not networking performance. Heavy builds/raw
diagnostics stay on Wick; these archives contain no binary, VM disk or payload
dataset.

The concrete endpoint partition provider still refuses before effects. Actual
endpoint/store ownership, full bootstrap/runtime/shared budgets, original
positive native stop/retirement, sustained Calls/ACK recovery, capacity/fairness,
platform/WAN and upstream delivery remain open. This checkpoint implements a
required owner, without activating an incompletely qualified endpoint profile.
