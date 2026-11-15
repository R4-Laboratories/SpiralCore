# Installed reader/store custody and directory funding

This #258 slice adopts the reviewed bounded-directory core in the frozen MVM rc.3 contributor and qualifies its original-store destruction before credit. All five #42 outcomes remain active. Native host-process fixtures do not establish installed booted MVM, backend, WAN, fleet, FIPS/CUI, host-blind or physical-energy acceptance. Ari's compatible signed runtime and private MVMD remain external gates; no final upstream MVM contribution is submitted.

## Source and behavior

Contributor MVM `9e56be556720e427013cce4076aeecd0b561a065`, tree `42f3dbe791234790e7653446b4e9453044f77367`, derives from frozen rc.3 `611a3c76bf6f064760e5adef4fa7befbeb4eb650`. Both direct dependencies and all five locked SpiralCore packages use published `29393e9b476daeef41f51485d2fb8829c4bbefde` from PR #289, with core evidence in PR #290. The only new locked package/edge is the reviewed macOS directory helper. This campaign does not qualify newer upstream rc.4. Only the contributor fork branch is published, with no upstream MVM PR.

The factory reads the core's 32,768-byte directory-buffer payload quote and checks it against the existing 1 MiB workspace floor before booking resources or touching storage. A larger requirement refuses Unsupported rather than silently enlarging the envelope. The original shared parent plus six descriptors per store, 102 visits, exact-charge/one-byte-short checks and signed authority remain unchanged. The quote excludes allocator overhead, bookkeeping, native stacks, kernel memory and RSS; it is not a hard memory bound.

The new positive control demonstrates an actual ordering defect in the old permit destructor: the ledger lock was released and returned credit became visible while `self.store` still held the original core store. The corrected destructor performs original filesystem cleanup outside the ledger, checks all four subtraction results, destroys the original store under the ledger lock, then publishes the four debits. Independent review checked that final core destruction has no callback or pool backedge.

The test-only observer uses Weak<PoolInner>, not Weak<core Store>, preserving exclusive retirement. The RED records ledger_held=false/returned_credit_visible=true. GREEN records ledger_held=true/returned_credit_visible=false/original_owner_released=true. Original replacement/quarantine assertions remain unchanged. Cleanup uncertainty or poisoned accounting retains the original store and held resources, including its captured lease when present, buffers, handles, writer lock and debit; replacement failure can precede retirement. Confirmed filesystem cleanup can precede ledger poison detection, so names may already be absent while original resources remain held. The existing arithmetic-failure branch is different: it returns no credit and preserves fenced ledger accounting, but its confirmed-clean store drops on return; it does not quarantine that store.

SpiralCore PR #291 candidate `a0bb0dba681a2c72aa59ea2d194f9fc2ca2694a4`, tree `c209892c8e6271fc86b7344874488b0cbdb8432e`, updates active SDK references to this exact contributor, aligns both installed-tool direct core pins to the same published identity, and adds exact native controls and source/package/executable guards. It changes no product Rust source. All 35 executed checks pass; one manual genuine signed-export job is SKIPPED/NOT_RUN. Independent source/security and native artifact reviews are clear. PR #291 merged at `9688efa94dba9586fd20ec665c9fd38fed2d8f5c`, whose released tree matches the reviewed/tested candidate. The CI merge `f59f2f8a713bdda583eb18a6a71f126966911cca` has the identical candidate tree. Seven actual resolved lock maps preserve existing registry versions and dependency contents after the declared pin substitutions and helper addition. The separate historical journal source and historical evidence retain their pins.

## Wick source qualification and costs

Rust 1.97.1, two Cargo jobs and the existing shared Cargo cache/target are used in bounded user services: 6 GiB RAM, zero swap, 200% CPU. All 4,679 candidate inputs match before and after the final workspace and registered/graph checks. Genuine candidate Git provenance is supplied from operator-main metadata; source identity checks use in-process Rust/gix. No operator Git runs on Wick. Source-provenance, four reader controls, 15 payload/store controls, nine registered-process controls, format, strict workspace/all-target Clippy, gated compilation, the full serial workspace and doctests pass. Raw result rows are retained without summing nested counts.

The full workspace requires the recorded exclusive short private TMP root and finite soft NOFILE 4096 / hard 524288. GNU time reports 558.78 s wall, 272.18 s user CPU and 111.38 s system CPU; the enclosing service records 559.932 s wall, 384.471 s CPU and 6 GiB peak. These scopes differ and do not measure an installed operation's resource allowance. The active unit receipt records the configured limits, while raw journal and command exit/source checks support successful completion; no garbage-collected default properties are used as configuration evidence.

The final isolated registered direct-TCP fixture verifies exactly 5,804 application bytes each way and observes 3,657/3,234 TLS bytes (6,891 total). Setup through Hello is 70.388550 ms; valid Open through verified half-close is 20.489519 ms; retirement including archive retry is 26.071250 ms. Whole-fixture GNU time is 0.12 s wall, 0.02 s user and 0.02 s system CPU, maximum RSS 24,164 KiB. Executable hashes match before/after. These are host-process fixtures with authenticated half-close, peer join and original registration retirement. Separate sender/receiver CPU, transfer-only CPU, TCP/IP retransmission bytes and physical energy are NOT_MEASURED. Displayed CPU granularity is 0.01 s, not proof of zero cost.

Eight matched graph pairs cover hostd normal/build/features on Linux x64/ARM, macOS x64/ARM and Windows x64/ARM, plus Linux mvmctl and builder-guest executable closures. After core-pin normalization, only macOS adds the helper and its existing libc edge. The VMM normal/build/dev/feature closure is byte-identical. Graph resolution occurs on Linux and does not substitute for native execution.

SDK checks pass: eight canonical endpoint controls, four collector controls, production-file supervisor and owner harnesses, both paired guest feature configurations, installed-tool/guest checks and strict checks. All 586 SDK input hashes match before/after and the committed source. The SDK service records 267.839 s wall, 444.699 s CPU and 3.7 GiB peak; build/test cost is distinct from transfer cost.

## Native consumer and SDK observations

Linux x64, Linux ARM and macOS ARM each execute eight distinct original reader/store controls. Each original ZIP binds contributor commit/tree, all 4,679 actual source hashes before/after, an unchanged executable, all five locked core names/sources, exactly three Linux or four macOS active core packages, and eight nonzero test results. The positive probe confirms original-store destruction under the ledger lock with returned credit still unavailable. The three whole-retirement wall observations are 915,291 ns (Linux x64), 854,678 ns (Linux ARM), and 1,098,541 ns (macOS ARM); these are fixture lifecycle measurements, not transfer timings. Per-control real/user/system costs are retained at 0.01-second reporting resolution.

Inherited host NOFILE receipts read 65536/65536 on both Linux rows and 10240/unlimited on macOS. The workflow records these values without changing them. These focused custody tests do not execute the separate VMM high-index concurrency fixture and do not qualify its descriptor-pressure envelope.

All six native parent/endpoint ZIPs bind the exact SC candidate and actual source hashes. Parent footers remain 38/3/11, canonical endpoint8, and Linux collector4; they are separate scopes, with no nested-count sum. All nine ZIP digests match GitHub API digests and the exact candidate workflow identity. Every ZIP member is CRC-checked and SHA256-inventoried. These are native host-process components, with no booted guest or WAN.

## Fresh seven-platform component observations

All seven native FlowMux rows pass 87 controls and exact 64 MiB application readback in each direction. Each observes 5,395,050 TLS bytes sent and 5,395,015 received at endpointA, cross-matched to peer counters (10,790,065 total); these are transmitted TLS bytes, not planned bytes or full TCP/IP/retransmission accounting. The original 2 s operation and 240 s workload guards remain. Seven original ZIPs match GitHub API digests, with 198 member hashes and 191 embedded checksums; their recorded CI merge identity has the released candidate tree. Windows lock hashes exactly match the candidate lock with checkout CRLF conversion; Linux/macOS match its LF bytes. This is an explicit byte-format binding, not arbitrary content normalization.

Platform | Setup wall us | Lifecycle wall us | Lifecycle process CPU us
--- | ---: | ---: | ---:
Linux x64 | 9489 | 9164490 | 13540934
Linux ARM64 | 8727 | 7321878 | 11258559
macOS ARM64 | 53140 | 24985428 | 23166930
macOS Intel | 64480 | 82475730 | 62516192
Windows Server 2022 | 86663 | 38329348 | 58015625
Windows Server 2025 | 61534 | 39963827 | 61859375
Windows ARM64 | 116153 | 110686199 | 95468750

The setup and lifecycle intervals have their original instrumentation scope; lifecycle includes protocol cleanup and reader join, with whole-process CPU potentially spanning both endpoints. Separate sender/receiver CPU and physical energy remain unmeasured. These are native component/client fixtures, not installed MVM backend/WAN qualification or a performance distribution. Raw records retain optional/unavailable cost fields without inferring zero.

## Retained failures and resource prerequisite

The packet preserves the intentional RED, the first misplaced-drop compile failure, the new poisoned-ledger fixture's incorrect named-state expectation and its correction, and the test-only Clippy let-and-return failure. No published revision contains those intermediate edits. The production cleanup order and original assertions remain intact.

A long private TMP path produced a 105-byte socket shape exceeding the unchanged 103-byte cap. The fixture selected a missing hashed fallback and eight registered controls refused before effects. A new exclusive 0700 short TMP root corrects the campaign environment; no product/test source, permission check, socket cap or deadline changes. One preliminary source hash attempt used the wrong physical cwd and is explicitly discarded; the correct absolute-worktree manifest contains all 4,679 files.

The first full workspace under inherited soft NOFILE 1024 failed one unchanged four-worker/16-iteration VMM prepared-disk fixture (1,111 passed / one failed / four ignored in that suite). Its upward-search allocator starts at index 1000. Bounded syscall traces show F_DUPFD_CLOEXEC at 1023 returning EMFILE and at 1024 returning EINVAL while lower slots become free. Genuine baseline and candidate hashes/closures isolate this preexisting boundary: each passes 9/10 at 1024 and 10/10 at finite 4096, with the hard limit unchanged. All 40 paired controls, binary hashes and original failures/traces are retained. This does not fix the allocator or qualify arbitrary descriptor pressure. The supported capacity envelope must record descriptor-index prerequisites as well as counts; #49 remains open. No assertion, concurrency, signed budget, deadline or security check was relaxed.

The missing historical local Git object warning is outside the current candidate tree; every current tree blob exists and archive/push identity verifies. No history rewrite or broad repair was attempted. Required local ponytail/rust-skills and Trail cache were unavailable after search; the available secure-execution-review workflow and independent review were applied. No unavailable skill or cache is claimed as executed.

## Acceptance boundary

#258 remains open for complete allocator/workspace/concurrency and installed memory-funding acceptance. These controls demonstrate bounded core buffers and original resource lifetime/accounting; they do not prove an isolated workspace or hard aggregate RSS envelope. The complete #42 program retains installed mailbox ownership/budgets/shutdown/recovery, persistent final-ACK recovery/retention/expiry and sustained traffic, shared-core FlowMux and separately authorized owner access, canonical two-host WAN and Linux/macOS/Windows role/version/backend qualification, and supported capacity/fairness/saturation plus the final upstream contribution. Ari must publish the compatible signed runtime; private MVMD remains external.

Compact source-bound logs, graphs, identities and all nine original native ZIPs are retained; independent final native artifact review is clear. No compiled executable, Git object pack, VM image, payload store or credential is published. Owned disposable exports are removed only after evidence verification/publication and terminal/handle/ownership checks; shared Cargo caches, user state and prior historical evidence are preserved.

Baseline comparison is independently bound to the exact 4,679-file manifest extracted from the previously published consumer packet (baseline-reference.json). Both new before/after receipt name sets match it; three bracketed public-page names are single-quoted by GNU checksum display and are normalized only for receipt-name comparison. The baseline manifest bytes and prior archive digest are retained.
