# Independent B2 CLOSED transaction review

2026-11-04. Reviewer `/root/closed_recovery_review`, independent of the B2 author. Scope: exact six-file CLOSED filesystem transaction candidate against qualified B1 `8856373d45ade643194d130e2f115fe817315dbd`, actual ownership/ledger movement, finite interrupted restart and retained evidence. No builds, source edits, new tests, native guests, publication or agents were performed. Original freeze and failures are preserved.

**HOLD: two P1 namespace/publication conditions need focused refusal tests and minimal fixes before accepting this CLOSED transaction.** These are not findings of an Open issuer, native receipt, refund or customer restart. The observed successful closed transactions remain valid bounded evidence. Incomplete activation/live-inventory/native gates are not accepted by implication.

## Exact source and evidence identity

Freeze directory: `/tmp/spiralcore-retained-catalog-20261104/stage-b/b2-closed-freeze/`.

| Artifact | SHA-256 |
|---|---|
| six-file `source.tar.gz` | `5a2c62502bdc7968eee90bb01568e16f0ed7ee5148968fb3fa8c665f81638830` |
| `candidate.diff` | `7678c96568245c827ca38346bb8780ae13455b046a0c015dc3247ca86cec9a46` |
| `source.sha256.json` | `829cad5ae84e5c4139fb22893aff01d3c468b430e4657225364c175fd79b95e6` |
| `compiled-source139.json` | `841cdde1722d3793a0ef24f3e23e38d3707d2e59b2890b098150eb3805ff7ba1` |
| `receipts.tar.gz` | `97c393503fcf2ce2048e9813ae0d2a316a0b3cb240386772b91c635fc420435d` |
| `metadata-worksheet.md` | `5935bd38468678e2adac53e19276d24c2723d5e992eea3d96fa6d3f380304737` |

All six current mapped sources and six source-archive members match their frozen hashes. All 139 current compiled inputs match the frozen closure. All 286 receipt members match their archive index; the archive has 287 members including the index. The actual final protected executable on Wick matches `c03e950c0a6793c17a339ee912c161c07dc042cddd0b53f0cbb9ae806b1157ac`.

Source files below are relative to `crates/spiralcore-mvm-live/src/`:

| File | SHA-256 |
|---|---|
| `installed_owner.rs` | `0f773411912745414c44ca6229bb237cd7530f59a32f3b266b9c728783b8873a` |
| `installed_owner/retained_telemetry.rs` | `f7f064df1835f632da88538ff9db88c251da4cd1ae9699263f1cad3ee1f35011` |
| `installed_owner/retained_recovery.rs` | `4f29228901da0cddbe8184de8a76c31feb3733fcd9a7fbecc036605551a40ccd` |
| `installed_owner/retained_transition.rs` | `2e0ce3d43a546f67f42a80fab5149ae2421966d6da00f7e39baaee1e1cd14e7a` |
| `installed_resources.rs` | `378ee1f09665c42857b0ee4f8b13643d0090aa9776e70917c50edf651424bd15` |
| `installed_supervisor.rs` | `047033f4ebbffaeaa78aae7932083265157cd338dfe76c6963a6ff7a32df98ea` |

## P1 — Fresh commit can overwrite an unexpected immutable canonical intent

**File:** `installed_owner/retained_transition.rs:250-258, 345-351`; `rename_exact:165-181`.

`prepare_kind` checks that TRANSFER_INTENT and TRANSFER_NEXT are absent. Preparation returns a controller and commit can occur later. Fresh `commit` revalidates source rows/catalog and alias eligibility, but does not recheck canonical transfer-name absence. It creates TRANSFER_NEXT exclusively, writes/syncs the expected intent, then calls ordinary `renameat` through `rename_exact`. That helper checks only the source inode and permits replacing an existing destination.

A private canonical TRANSFER_INTENT introduced after successful preparation can therefore be overwritten rather than refused. This is a deterministic source-level gap: no racing thread or hash collision is needed to insert the unexpected intent between the two API calls. It can discard unknown transaction metadata and continue to successful catalog/state publication. Actual owner/holders EX still remain held, and this returns no open service; those properties do not authorize replacing an unknown immutable intent. The existing replaced-intent test covers a resumed controller's pinned intent, not this fresh preparation/commit gap.

**Smallest focused RED:** prepare a valid synthetic phase4 CLOSED transaction; create an unexpected private canonical TRANSFER_INTENT before commit; assert commit refuses, preserves its exact inode/bytes and source catalog/state/beacon, retains exclusions and full debt across failure Drop. Include a valid-looking identical-content candidate if practical: bytes alone cannot grant replacement authority.

**Minimal fix:** revalidate the fresh finite namespace at commit and publish a newly created canonical immutable intent without replacement. Use the supported platform's atomic no-replace operation or an equivalently specified/tested primitive; a check followed by overwriting rename alone leaves a replacement window. Keep exact acquired original/candidate File custody on refusal. Do not route this through the resume branch or accept arbitrary preexisting intent bytes as permission. Preserve the established durable intent-before-catalog/state ordering and finite crash behavior.

## P1 — Interrupted restart omits fresh path's excluded namespaces before effects

**File:** `installed_owner/retained_transition.rs:250-259, 554-608, 682-901`.

Fresh preparation refuses CATALOG_INTENT, endpoint-disks.bin/intent/next and disposable namespaces through `endpoint_disks::no_namespaces`. The interrupted opener does not repeat those eligibility checks. Its finite observer classifies TRANSFER_NEXT, CATALOG_NEXT, owner INTENT and NEXT, but does not reject CATALOG_INTENT or the disposable inventory/namespaces.

Thus an otherwise recognized interrupted transfer can be resumed with an untracked disposable namespace or endpoint inventory present. Commit can roll catalog/state forward and return committed CLOSED custody without rejecting that unaccounted namespace. A stray CATALOG_INTENT is eventually rejected by final `held.revalidate`, but only after catalog/state publication. The durable intent binds the retained source/target transaction, not absence or retirement/accounting of newly observed disposable resources. A prior process's successful preparation cannot substitute for checking the actual namespace now observed after process exit.

**Smallest focused RED:** crash the actual synthetic child at the durable-intent barrier; introduce one name excluded by fresh preparation (cover inventory metadata and an existing disposable namespace separately); call resume/commit and require refusal before source catalog/state change, preservation of all original/introduced bytes and actual locks/observations, with no admission/refund. Add a CATALOG_INTENT case to distinguish early eligibility refusal from late post-publication failure.

**Minimal fix:** factor/reuse the fresh path's exact finite no-untracked-namespace eligibility guard in the interrupted path before any rollforward mutation and revalidate it at the effect boundary. Retain checked observations on unknown metadata refusal; do not delete or reconcile the excluded namespace under retained-transfer authority. No new disposable cleanup authority or full recovery scan is needed.

## Additional source/claim limits

These are not additional Open-authority findings:

- `commit` opens current STATE into a closure-local `Option<File>` at 431-434 and validates it before moving it into a transaction slot. If a new malformed/replaced candidate is observed at that late boundary, that newly opened candidate File can drop on error. Previously pinned source Files and EX locks remain retained. The declared all-acquired-observations custody contract should be applied here as well, using a bounded separate slot before fallible checks. A focused post-state-intent mutation/read-failure test can check it; no new reproducer was executed in this review.
- The README states full fixed metadata and row funding precede heap catalog decoding. Actual resume source funds fixed metadata at 814, then reads/decodes/encodes catalog data at 816-835, and funds rows at 863-866. Fixed metadata does precede heap decoding; row funding precedes row data acquisition and all rollforward mutation. Correct the attribution, or implement/test a bounded predecode row-funding scheme if the stronger requirement is intended. The current closed metadata parsing is not a demonstrated refund or admission window.
- Same-process failed-I/O retry and complete partial-write/unwind/replacement permutations remain explicitly unqualified. Successful barrier exits do not establish failed file/directory sync recovery. No broader fault matrix is inferred from the six selected cases.

## Ownership and transaction properties supported by source

- The migration target is privately derived from the actual current source/model; supported source profiles are the fixed old Linux/x64 profile plus the actual current profile. Source/target epoch/key, embedded owner records, installation/object IDs, source/target full-file digests and reverse proof are checked through the existing canonical codec. The beacon's unchanged first-debt lineage and each row's original marker epoch are preserved. V2 origins do not recreate old scopes/grants/receipts.
- The fresh transaction durably writes/syncs immutable intent before catalog publication, then uses target catalog/state staging, file sync, rename and directory barriers. Original Storage.key and Record are replaced as a pair inside the original owner Mutex after target bytes are checked; no Clean record is published. The original owner EX and holders EX remain physically held; no holder conversion occurs in this checkpoint.
- Resume accepts the intended old-state/old-catalog, old-state/target-catalog and target-state/target-catalog digest pairings. Target-state/old-catalog refuses. Canonical intent replacement after initial pinning refuses by inode even if contents match. Partial/unknown fixed intent refuses. The CLOSED observer does not bypass generic current/read/no_temp, and ordinary reserve still fails on admission_open=false.
- Strong and Weak aliases of the actual pool/authority are refused before normal transaction preparation/rebind. The actual Mutex<Ledger>, slots, serial state, authority owner/holder/worker objects and imported flag move into a NEW closed authority, with no default ledger or duplicate fixed-metadata charge. Alias-free Arc unwrap is checked again. The partial-move guard now owns actual ledger storage in a Box; failure retains it and the actual authority/owner objects. The initial non-Drop forget implementation was a real defect; final source fixes rather than masks it. The comprehensive injected partial-move/unwind matrix is not established by alias-count probes or Clippy alone.
- Historical original state/marker/records/key/status pins and FULL R+K remain held. Fresh validated duplicates are released only after positive comparison; failed observations and actual controller custody remain quarantined on Drop. The successful duplicate-slot correction fixes the observed second-revalidation failure without replacing original pins.
- Successful commit returns only committed CLOSED debt custody, and metadata-only retained stop is explicitly distinct from native stop/finality. No InstalledSupervisor/Open issuer, deletion, expiration or refund was found. Existing global finish semantics are not weakened.

## Executed evidence inspected independently

Raw prefix: `/var/tmp/spiralcore-retained-catalog-20261104/validation/retained-v2-b2-*`. The compact archive was verified against all 286 indexed receipts; final raw executable/maps/results were independently inspected on Wick. These results were not rerun by the reviewer.

- Real-I/O publisher RED: build0/test101; before/after catalog hashes differ while owner state is unchanged and transfer intent is absent. The final normal publisher rejects cross-epoch publication before mutation.
- Commit API absence: E0432/E0599; resume API absence: E0432; stop API absence: E0599. These are genuine missing-interface REDs, separate from behavioral execution.
- First actual resume test failed101; test-only diagnostic localizes refusal to final held revalidation. Final source clears only successfully matched fresh row observations before a second pass, and the same barrier behavior passes. This is source/record support for that bounded correction, not proof of every resume permutation.
- Initial scoped Clippy failed101 on `let_and_return` and `forget_non_drop` for the actual Mutex<Ledger>. Final partial-move guard boxes that Mutex; no lint suppression is used.
- Final protected CLOSED successor: six selected B2 parent tests PASS; one includes four fresh child exits77 at durable intent/catalog/state-intent/state barriers and same-target generation2 completion. Current-source committed child exits78 and reopens historical V2 debt before generation3. These are ordinary synthetic metadata/filesystem child processes, not MVMs or native retirement.
- Final observer/ledger ABI test PASS. Full same protected Root library: **172 PASS / 15 ignored**, 12.51s, including B1's 17 codec tests and the ABI test. Those are not extra unique tests. Explicit ignored StageA acquisition five, malformed-marker one and actual old phase2 one pass separately.
- Scoped release Clippy `--lib --tests -- -D clippy::all` exits0. Actual JSON has **73 ordinary diagnostics** (72 dead_code, one unused_imports); build JSON has two ordinary unread-error-field warnings. Not zero warnings, `-D warnings` or all-target qualification.

Final source-before/after maps equal frozen 139-input SHA `841cdde1722d3793a0ef24f3e23e38d3707d2e59b2890b098150eb3805ff7ba1`. Historical four-file fixture before/after maps equal SHA `3ac4ac70557be27b85dcafb4e07a73154a557a54c091972456a9278d1d17d81c`. Final selected and broad commands use protected executable `c03e950c0a6793c17a339ee912c161c07dc042cddd0b53f0cbb9ae806b1157ac`.

Selected final CLOSED successor raw hashes:

| File | SHA-256 |
|---|---|
| `test.stdout` | `ef2405b6d79d677021c0e7da1eb60724481cae5cd922cd92e89039f61fa8ff16` |
| `size.stdout` | `c2d0f0b4a5b543762809915e0ee1e443d23f29fc6de9c817286595e53e9a9027` |
| `build.jsonl` | `4fa5114e2f4131e695384f1638f53a2797ad0d018d5d337b49b5dc5e19c2cb88` |
| `strict.jsonl` | `a0cee78567b183b81bddb7cae5b363bb616d75a8424e6a395e2655d2bcdc386e` |

## Capacity and delivery boundaries

The same 16 total retained-row limit, 8 MiB+32 KiB per row and 1 MiB+8 KiB fixed allowance remain. B1 logical disk overlap arithmetic is unchanged. The provisional 930,152-byte worksheet plus 16,384 observer bytes and 24,576 transient ledger-Box bytes totals **971,112**, leaving **77,464** under 1 MiB. ABI tests constrain fixed assigned objects; they do not prove full container capacities, allocator/native pthread/kernel FD/TLS or RSS costs. No new quota or platform capacity is accepted.

Actual live V2 inventory/historical membership and one-use activation, 16-row mixed-origin filesystem/live saturation, holder EX→SH conversion, interrupted retained-stop opener, complete I/O/unwind matrix, genuine native phase4 producer/EndpointRetirement, useful transfer after restart, platform/WAN/capacity, regulatory qualification and upstream customer delivery remain open. The two focused P1 corrections should be resolved independently before accepting this CLOSED checkpoint; none requires broadening into activation.
