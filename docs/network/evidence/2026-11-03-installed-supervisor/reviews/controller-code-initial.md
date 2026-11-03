# Installed supervisor first component: CODE/RUST

**PASS — no blocking source finding at112eeeab.** Reviewed
112eeeabaa730f0909abc1bc286ef58f4c9be6be against its immediate predecessor and
the controller portion of installed-supervisor-v1. This is only the first
controller/accounting component; no worker activation or full installed service
is implemented or approved. No product/test edits, builds or tests were run.

start consumes a non-Clone owner, freshly validates its profile and begins one
epoch before constructing one private installed budget. The fixed profile hash
uses the specified domain and eight big-endian u64 values. Config is private and
fixed; failure/unwind after begin leaves Active rather than publishing Clean.
The new owner getter reuses current() validation, without another storage parser.

Shared→InstalledAuthority→Mutex<InstallationOwner> has no reverse budget edge.
Budget/scope/link/frame ownership therefore transitively retains owner EX,
including after controller drop or successful finish. The authority/Mutex and
installed factory are crate-private; public independent budget construction
continues to attach no installed authority. No public owner extraction/rebind
or Clone controller was added.

Scope/link/frame charging checks admission under the existing ledger mutex.
close irreversibly stores false with Release then takes/releases that mutex:
earlier admitted charges complete before the barrier returns; later charges
observe closure. Snapshot and releases retain their existing accounting roles.
try_finish drops the snapshot lock before owner Mutex/storage work, requires
unfenced zero usage, then uses the same retained Epoch and real holders-EX gate.
Busy preserves the closed controller for retry; storage/poison failures do not
reset usage or epoch. No filesystem or owner lock is taken while holding the
transport ledger. Drop only closes admission; last authority drop retains the
owner's existing no-Clean-on-Drop semantics.

The legacy session_with_budget guard is before plan/file/activation effects and
rejects any installed-bound budget. The optional authority field/import, private
factory, construct argument and module export use matching native-host cfgs;
guest/Windows paths retain independent budgets and false installation binding.
This is source inspection, not executed guest/Windows/MacIntel compilation.

Read actual Mac focused API-absence RED exit101, and boundary-green exit0:
six existing private unit tests plus three controller lifecycle tests, all9PASS,
zero failed/ignored/filtered. The new boundary validates profile rejection before
epoch mutation and real live-SH Busy at zero transport usage, unchanged176-byte
Active record, closed-clone refusal, then same-epoch successful finish after
holder exit. Other cases prove sole-frame transitive retention and idle-budget
clone exclusion after finish. Scope/link retention is additionally clear from
the shared ownership graph, not counted as extra independent executed cases.

Current reviewed SHA-256 pins:

```text
a58b833f02ddf7e51bec6b8c746efe5aa62a79520ef9799cbcbb38bc1daf6ae0  src/installed_supervisor.rs
0a717f421138f864952366550188e0efdbfb2530f4adb6030d035f1e7f034af1  src/host_budget.rs
cb22012ceeba38688c966b9d8b4d3733eeb2dd669984bdf039780aa967934b02  src/installed_owner.rs
b093a042bd84ba07f12a4acc71cfed26b31207c37e3de189d1492d0f7e3a2b9b  src/lib.rs
9053833bfb00af0ad8bce024fbf1215093d396a1d2a0c24fd18235f58c9d39e5  src/orchestrator.rs
dde742094728caa5ea5bd71397212c1c3bbe95a6413aa033e28583adf77189a4  docs/api/installed-supervisor-v1.md
```

The finish comment correctly limits this slice: a future private owned/reaped
worker ledger must gate finish once workers can be admitted. Single-session
launch context, worker grants/channel authentication, actual backend lifetime,
consumer/default-path closure and native positive recovery remain required.
Clippy/Linux qualification was pending at review; no PASS is inferred for it.
No full B6a/B6b, wider budget/scale/WAN or performance claim follows. Earlier
published packets and the frozen separate PR70 source remain untouched.
