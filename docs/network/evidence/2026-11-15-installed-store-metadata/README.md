# Installed-store metadata and current-runtime delivery

[PR309](https://github.com/R4-Laboratories/SpiralCore/pull/309) merged at
`dacd7eb3185739c1de2f87906e8b3923a50a9720`. Reviewed head `d8f328f9`, CI merge
`a5a47619` and release share tree `9edebbbb8d22e473ce75e28eb6bd8104c9f0b2a9`.
The selected contributor MVM is
[`e95f1119`](https://github.com/Casey-allard/mvm/commit/e95f1119dac66e45a81c4eead8f4e747a1286a86),
tree `1642e7294aee39c51a634c9d10d0b25d7bccdb5e`, on canonical `4047e48`.
Both MVM dependencies and all five locked SpiralCore packages retain reviewed
core `8994951b`. The canonical journal bridge aliases at `48fa5b48` remain
explicit; they are not silently normalized into the contributor revision.

## Delivered boundary

Linux x64, Linux ARM64 and macOS ARM64 each execute **14 nonzero original
reader/store/credit controls**. The original ten selectors and control-6
destruction-before-credit assertion remain. Two ordinary tests change real
directory permissions to `0750`; two separately selected ignored tests run as
root and change the actual directory UID to `65534`.

Before capture, refusal retains the original parent/name/directory and all
debits, with no initialization files or core lease. After claim, refusal also
retains the original lease, writer lock and accepted object bytes in quarantine.
A bounded snapshot compares relative file bytes, identities, type, ownership,
mode and links before mutation, after explicit refusal and after permit drop.
Each negative control first runs the unchanged successful retirement assertion
under the same actor. The fixture publishes and exactly reads back **49 bytes**
through canonical frames: **268/218 serialized request/reply bytes in memory,
zero network bytes**. Its funded pool is not an original VM/Root reservation.

All three native archives bind MVM's same **4,747 source files**, its source
tree, compiler-selected test executable and equal runner-recorded before/after
hashes. All **42 exact executions** report one passed, zero failed and zero
ignored. Root privilege and the real `fchown` operation are required by the
hashed tests; an unavailable privilege grant does not become a skip/pass.

The four invoking workspaces compile the selected contributor `mvm-core` and
`mvm-crypto` at optimization level **2**, retaining debug assertions and overflow
checks. Compiler JSON, actual library digests and post-checks bind this result.
Seven SDK dependency comparisons permit only owned rc.6→rc.7 labels and the
canonical core→crypto→unchanged AES-GCM 0.11.1 split. The MVM comparison against
qualified 5819 permits only owned labels. Every other package, edge and feature
matches. The paired path lock additionally preserves its existing Windows
selections under locked offline resolution. A profile setting is not a measured
Windows speedup.

## Qualification and custody

| Scope | Executed result and limit |
| --- | --- |
| Frozen MVM e95 | 21,847 outer binary / 27 rustdoc passes, zero failures, 71/eight ignored; all 86 policy gates |
| Source/build | All 4,747 MVM inputs and 285 recompiled/executed test ELFs bind before/after; strict workspace Clippy, all-target and explicit BDD compilation pass |
| SDK | 175 live / 29 owner / 74 supervisor and one addressed-journal control pass; one helper-backed Root witness ignored in this ordinary SDK run |
| Owner/supervisor follow-up | New 29/74 executions bind all seven compiler-selected cached ELFs to actual runtime paths and before/after hashes |
| Paired path workspace | 18 passes, one ignored emitter; four recompiled/executed targets, including two zero-test binaries |
| Native CI | 35 successful checks, one genuine signed export skipped/NOT_RUN |

The frozen SDK has **490** inputs; the paired workspace has **17**, plus five
fixture files. All relevant before/after hashes and complete file-set checks
pass, including the MVM dependency root. The SDK export includes the four
already-merged Root/store-load sources omitted from the older 486-input export.
The later paired-lock repair lies outside those 490 SDK inputs; independent Git
blob comparison confirms their equivalence at both tested SDK head 180075a4 and
final source head d8f328f9.

SDK3's original owner/supervisor runs have runner logs but no original pre-run
ELF receipts. The separate custody2 executions close that gap prospectively;
they do not reconstruct the missing receipts. Library-profile artifacts and
live/addressed executables have their own recorded custody. Native ZIPs contain
no executable bytes: their executable assurance is compiler/runner-recorded
before/after hashing, not downloaded-binary rehashing.

BDD targets compile; Cucumber scenarios are **NOT_RUN**. Source-footprint
accounting remains measurement-only under canonical #5573's suspended ratchet.
The full workspace test command records **514.81 s wall, 184.33 s user CPU,
100.67 s system CPU and 4,326,316 KiB maximum RSS**. These cover the Cargo/test
command and waited descendants, not isolated workload or network costs. Wick
units request and record 6 GiB memory, zero swap, 200% CPU, 512 tasks, 8 MiB
stack and 4,096 descriptors. Those unit limits are not asserted for hosted CI.

A later [read-only upstream refresh](operator/post-qualification-upstream.json)
finds canonical main `2b1cd835`, seven commits beyond qualified `4047e48`.
That delta includes the ADR-061 bounded-framing prerequisite, an HVF standby
refusal fix and the rc.8 source version. It is not adopted or qualified by this
frozen packet. The ADR's accepted layout retains one hypervisor owner and the
explicit optimized crypto boundary; design acceptance does not establish the
later extractions. The rc.8 release-by-tag API returned 404 at this observation;
that is not proof about other publication channels or compatible signed guest
bytes. Selected source, runtime-producer agreement and live qualification remain
separate.

## Actual transfer measurements

All seven native FlowMux rows reconstruct **67,108,864 bytes per direction**,
cross-match both TLS counter directions and record **10,790,065 total TLS
bytes**. Existing 2-second readiness and 240-second bulk guards remain.

| Native component | Lifecycle wall | Whole-process CPU |
| --- | ---: | ---: |
| Linux x64 | 8.148506 s | 10.884219 s |
| Linux ARM64 | 9.924928 s | 11.006605 s |
| macOS ARM64 | 19.989442 s | 18.755098 s |
| macOS Intel | 79.176695 s | 55.735718 s |
| Windows 2022 x64 | 37.651877 s | 50.875000 s |
| Windows 2025 x64 | 52.550511 s | 67.140625 s |
| Windows ARM64 | 122.544954 s | 94.953125 s |

These are one observed component row per hosted role, not matched causal
optimization experiments, sustained-fleet acceptance or sender-only CPU. CPU
can exceed wall time across concurrent threads. Setup, memory and detailed
counters remain in [flowmux-measurements.json](native/flowmux-measurements.json).

Both Linux original-Root witnesses execute **64 distinct publications / exact
1,048,576 application bytes each direction** through the actual selected child,
populated store and retirement. All 64 samples retain one PID/start identity,
reach seven threads and record the child's **295,698,432-byte hard address-space
limit**. The existing explicit **2 GiB** profile and **24-worker** ceiling remain.
Disk and 4,096-byte metadata credit return after cleanup; other booked resources
and VM debit remain without an actual VM-exit receipt.

| Root/selected-child fixture | Linux x64 | Linux ARM64 |
| --- | ---: | ---: |
| Setup | 379.917325 ms | 106.697169 ms |
| Open through verified half-close | 1,167.618254 ms | 648.537692 ms |
| Sender-thread CPU | 109.380210 ms | 134.380176 ms |
| Peer-thread CPU | 43.133944 ms | 48.930904 ms |
| Whole child CPU collected at retirement | 488.508 ms | 470.730 ms |
| Child shutdown/receipt interval | 76.063151 ms | 49.476781 ms |
| Whole fixture wall/user/system | 1.64 / 0.40 / 0.38 s | 0.82 / 0.47 / 0.34 s |

The child shutdown/receipt interval ends **before parent cleanup**; whole-fixture
GNU time includes that later work. Each row records 1,078,631/1,078,502 sealed
request/response bytes and independent peer TLS received/sent bytes of
103,046/102,623 at a different boundary. The macOS endpoint row runs eight
fixture controls; it does not supply Linux Root selected-process enforcement.
[Full samples and clock domains](native/root-measurements.json) remain separate.

The addressed journal fixture exactly recovers **257 body / 313 frame bytes**.
Each admit/retry turn records a **3,751-byte envelope / 5,091 sent / 1,026
received local Unix-stream bytes**. Its 20.998/20.740 ms clocks end at the
`AcceptedLocalDurable` receipt. Recipient readback and acknowledgement occur
later, outside those clocks; they are not end-to-end recipient lifecycle times.
Sender-thread CPU is 0.975/0.803 ms, whole-process CPU 1.779/1.410 ms; setup is
0.560 ms wall / 0.558 ms sender-thread CPU. No receiver CPU or energy is inferred.

## Failures, cleanup and remaining gates

The records retain SDK1's controller manifest-path failure, SDK2's broad
formatting failure on unchanged dependency files, the custody1 missing-manifest
setup failure, and the original CI stale paired-lock refusal. Setup failures
prove no subsequent tests ran. No SpiralCore Rust source changed in this slice;
the extra broad formatting check was not relabelled green or used to rewrite
the pinned core. The actual changed TOML/YAML/docs and consumer compilation were
validated. The initial canonical CI observation includes failed aggregate jobs
with cancelled prerequisites; it is not presented as a passing upstream matrix.

Two verified cleanup receipts remove **3,292 obsolete `.rlib`/`.rmeta` files /
12,530,456,805 apparent bytes** from the owned shared target, excluding current
outputs. Recorded handle queries, per-file digests and deletion checks remain.
Filesystem-query warnings remain; absence is not asserted host-wide. All test
executables, source, original failure/control outputs, runtime state and user
sessions remain. Other system/host free-space changes are not attributed to
these deletions. No local build tree, VM disk or payload store was created.

Ari's [source-pin/compatible-runtime handoff](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6906088779)
retains CLI → mvm-client → hostd one-shot admission before warm selection and
the existing audit/OCI ownership and Required-tier sync overlap. Compatible
signed runtime publication remains Ari's gate; private MVMD and accepted source
coordination remain external. No signature, boot or runtime compatibility is
inferred from release tags or signed host-helper snapshots.

**Full #258 remains open:** iterator/allocator/concurrency funding, complete
negative/recovery acceptance, default-profile fit and Mac selected enforcement
are not established by these filesystem controls or Linux host processes.
All five [#42 outcomes](https://github.com/R4-Laboratories/SpiralCore/issues/42)
remain required: installed service/lifecycle; persistent recovery and sustained
traffic; shared-core FlowMux and separately authorized access; supported native
roles and two-host WAN; capacity and the final upstream contribution. No new
booted MVM, WAN, fleet, energy, FIPS/CUI or malicious-host guarantee is claimed.
The final upstream networking contribution is not submitted.

## Reproduce the record checks

The [native catalog](native/native-catalog.json) binds **28 original ZIPs /
3,652,783 bytes / 891 members / 644 embedded checksums / 24,955 source-hash
rows**. The [Wick index](operator/wick-records.json) binds **328 records /
62,932,249 uncompressed bytes** in `wick-records.zip`, **8,070,368 bytes**, SHA-256
`dd68c25af82b4fee5122136006aa7781f87891aa9827d6a54cb9545f00c6c412`.
The [packet checksums](SHA256SUMS) cover every retained file outside the checksum list itself.
No executable, private runtime home, signing key or payload store is included.

From a repository checkout containing the reviewed source commit, run:

```sh
python3 docs/network/evidence/2026-11-15-installed-store-metadata/operator/verify-records.py
python3 docs/network/evidence/2026-11-15-installed-store-metadata/native/verify-consumers.py
python3 docs/network/evidence/2026-11-15-installed-store-metadata/native/verify-native.py \
  docs/network/evidence/2026-11-15-installed-store-metadata/native . \
  docs/network/evidence/2026-11-15-installed-store-metadata/native/frozen-mvm-inputs.sha256
```

Default verifiers compare retained records without extraction, builds, archived
program execution or writes. Explicit regeneration modes are separate. Source,
test, native artifact and final packet reviews are independent advisory gates;
they do not close acceptance beyond the named scope.
