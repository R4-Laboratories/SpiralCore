# Broker-scale implementation review and test record

The adapter/live product source is unchanged from measured commit
`65abaa0b68a1a0e28b8e3aa713dfb4c8addf29dc`. Later changes add recorder diagnostics,
separate the portable functional smoke from the one-second benchmark profile,
and document measurements. They do not change the object codec, cryptography,
canonical message format or production limits.

Independent code/Rust and security reviewers approved the bounded product
implementation in parallel. A wrong-kind Reply builder originally consumed a
sequence; its regression reproduced that failure before the Reply-kind preflight
was added. Four sender tests passed afterward, including proof that rejection
leaves the first legitimate sequence at one. Reviews also approved the consumed
receipt-send permit and native dispatch/I/O phase separation.

The same reviewers approved measurement-finalization diagnostics and the explicit
portable smoke profile after inspecting the captured CI failure. No benchmark
deadline or production expiry test was relaxed. The measured source/binary stays
pinned independently from later test-only changes. Reviews identified no remaining
blocking source/security finding in this bounded scope; this is not certification
or completion of the MVM delivery program.

The project's `/quality-gate` command was unavailable in the execution interface.
It is not represented as executed. Explicit parallel independent code/security
reviews supply the recorded review gate, with roster eligibility verified against
`d1927d9e7ce51354`. The full review includes shared-clock ordering, budget ownership,
FIFO/retry/epoch authority, control-rate reservations, send reauthorization,
diagnostic disclosure and measurement classification.

## Executed local checks

| Check | Actual result |
|---|---|
| Final adapter release, all features/all targets | **95 passed**, zero failed/ignored/filtered |
| Live consumer release, all targets | **49 passed**, zero failed/ignored/filtered |
| Strict release Clippy, adapter all features/all targets | Passed before the final smoke-only change; final changed example/test also passed strict Clippy |
| Strict release Clippy, live all targets | Passed |
| Portable smoke and profile-isolation regression | **4 passed** in native ARM64 debug and release |
| Compiled production source coverage | **6,022/6,938 = 86.80%** |
| Changed production source coverage vs main `6bf85f9` | **417/441 = 94.56%** |

Commands used the explicit Cargo/Rust 1.97.1 toolchain, release profile, locked
dependencies, two build jobs and the existing shared target directory. Test
execution was single-threaded for the full release suites. Coverage used isolated
per-executable profiles, not instrumented timing measurements. The
[coverage packet](coverage/README.md) names its product-source scope, lower-coverage
host/orchestrator modules, exact binary identities and test-only source delta.
It measured 143 tests before the extra profile-isolation test; final functional
validation adds that test for 144 total. Product-source coverage identities
remain identical, and no branch/function/MC/DC coverage claim is made.

Normal suite outputs, compiler-warning checks and original failed platform
artifacts are intentionally retained as compact official test evidence here.
Raw payload datasets, executable copies and VM disks are not part of this packet.

## Final platform evidence

Final [portable CI](platforms/README.md) passed 449 executions at head
`ef814b995c3b51bdf7ad17ab216c5c279115bdaa`, executed as merge
`007c9c00c027b8478acb2d308716cd934b8f79cd` with the same tree. The separate Linux
workflow passed 43 release and 103 shared-conformance tests and its build,
warnings and analyzer stages. Original archive digests and inner manifests are
verified. The subsequent publication adds only documentation and CI evidence;
the implementation and execution configuration remain identical to that tested
head. The implementation merged through PR #50 at
`495df763384dad540a6e9f5fd36b3dc70d97f67d`.

## Qualification limits

The twelve load cells use real local TCP with serial round-robin dispatch.
All completed exchanges had exact byte recovery, but six saturation admissions
remained incomplete and expired as explicitly recorded. No generic durable inbox,
all-accepted delivery, exactly-once execution, simultaneous 32-operation capacity,
fairness, five-minute tracker-cycle plateau, physical energy or real WAN result
follows from these checks. Native guest performance and final upstream acceptance
retain their separate gates under #42–#49.
