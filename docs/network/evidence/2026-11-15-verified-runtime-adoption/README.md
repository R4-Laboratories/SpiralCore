# Verified runtime ownership and canonical gates — 2026-11-15

[SpiralCore PR #301](https://github.com/R4-Laboratories/SpiralCore/pull/301) is merged at `ecec011fbb591be15ed3d468adc01500c1a0ce2e`. It adopts Ari's current launch-owned verified-runtime path and finishes the canonical source-gate repairs tracked in [#300](https://github.com/R4-Laboratories/SpiralCore/issues/300). This is a source/component delivery under [#258](https://github.com/R4-Laboratories/SpiralCore/issues/258), [#268](https://github.com/R4-Laboratories/SpiralCore/issues/268) and [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42). All five MVM delivery outcomes remain active.

## Released source

| Identity | Revision / tree |
| --- | --- |
| Reviewed SC candidate | `c581aeef4337c51ecca93be18fc3a0ab7a8ae2b5` |
| Tested CI merge | `b264708c893af822b746ca7642c71579898abc2a` |
| Actual SC source merge | `ecec011fbb591be15ed3d468adc01500c1a0ce2e` |
| Identical candidate / CI / released tree | `dc284a0759305683fd8b7767e868763595f6fbff` |
| Contributor MVM | `3d52ff7e9f4ac8a935e0fd5fa8f25d4a73b953cd` |
| Contributor MVM tree | `6ba4e9dfb4eaff012415c2343dda523fa1ff50dd` |
| Canonical MVM base | `7eb015747545051fb5e26ae2c6d5fa4c86e4de00`, merged [MVM #5557](https://github.com/tinylabscom/mvm/pull/5557) |
| Both direct MVM SpiralCore dependencies / five locked packages | Reviewed `8994951b7c13fe5acf6f5ea3060ad4e51dc473c2` |

The CI merge's actual Git object and [public API receipt](native/ci-merge-commit.json) agree on its tree and parents: base `21569515256678cb96bd46822004196c96640702` plus the candidate. Freshly fetched `origin/main` after source merge has the same tree. [delivery.json](delivery.json) records the execution and release scopes.

The 16-file SC diff substitutes only the full MVM revision in seven manifest/lock pairs, the installed-reader workflow and the portable-platform source declaration. Eight matched dependency/feature graphs preserve packages, edges and features after validating exact source pins. The paired guest uses path dependencies; its unchanged lockfile has no MVM Git pin. Its actual MVM, SDK, application and fixture inputs are bound below. The canonical journal's explicit `48fa5b48` aliases remain unchanged.

## Runtime and authority corrections

The contribution now carries the opaque `VerifiedRuntimeSet` through the original mailbox, object/journal, host-lifetime and backend owners. Its authenticated archive/checksum/signature bytes authorize that launch's selection. Optional top-level `ExecutionPlan.runtime_identity` is signed and chain-audited data; it is not itself verification authority. Another launch verifies its own selection. Restore, fork and warm claim authenticate their retained generation. Selected-artifact hashing, Firecracker proof and lazy campaign-only assurance remain; there is no global authorization cache or signature waiver.

Three regression controls exposed checked mailbox binding dropping its original reader profile or admitting an unsupported object request before the proper refusal. The valid failing run and unchanged passing controls are retained. Two earlier attempts failed prerequisite namespace/umask checks and are not counted as functional REDs. A separate dispatch regression exposed a signed runtime identity reaching backend start after its verified owner had been stripped. Dispatch now checks the admitted plan and actual verified owner together before effects. Original readers, journals, reservations and failure/rollback ownership remain.

Exact installed-bundle verification and direct-plan admission use the existing application-aware trust composer. It retains authenticated descriptor/revocation checks, operator publisher-policy replacement, nonapplication enrolled-key behavior, local-origin enrollment, the signed archive pin and reserved decoded extent. Registered boot activation reuses the canonical command-line owner's required-grant token after the existing owner and grant checks. Its positive control uses the independent guest's exact token parser.

These changes retain canonical MVM authority, separate mailbox/FlowMux/access semantics and queues, and the same shared SpiralCore codec. They add no guest NIC, alternate egress route, opaque inspection, daemon authority or new runtime resource grant.

## Canonical gates and source accounting

All **86 canonical `check-all` gates pass** on the frozen final source. The original naming failure, later nine-gate diagnostic, corrected source and failed compile attempts remain in [operator/wick-records.zip](operator/wick-records.zip).

The 22 older sibling pairs received focused owner/caller review: 17 exact explained custody/authority/bounded-operation exceptions and five complete redundant-helper/API repairs. Four verified-runtime carrier distinctions follow the canonical rationale. The final scanner reports 33 explained pairs and **zero unexplained**; its zero allowance and stale-exception checks remain. The five repairs privatize the local retention helper, remove a pass-through journal-start wrapper, make two fixtures' owner/options explicit, and remove an unused uncounted forwarding wrapper while retaining the counted operation. Tests and assertions remain.

The ABI contract comes from a native Apple C-header probe: the 512-right control extent is 2,060 bytes; aligned storage is 2,064 bytes with alignment 16 and field offset zero. Rust assertions bind that storage geometry. The existing exact platform/kernel refusal is unchanged; SDK geometry does not identify or qualify a running kernel.

The network gate inventories four exact existing spawn seams, all using the canonical endpoint configuration, command owner and binary. Extra, missing, relocated or duplicate seams refuse. The shared-owner negative controls also cover the installed file. The exact audit key `via` permits only `ForwardPath::label()`'s host-selected `direct`, `proxy` and `no_proxy` values; it carries no payload, URL, credential or error text.

Nine oversized files were decomposed along their existing owners, retaining facade paths, state, ordering, field order and tests. No file-size limit or exception was added. The six existing shrinking exceptions remain. Review and compilation exposed two access mistakes; enclosing-module visibility was restored for the streaming writer's output and stopped-session cleanup, without exposing their state publicly. JSON envelope caps stay beside their original parser, unchanged.

Dependency budgets use the gates' documented reviewed-admission route. Canonical main's default Linux closure is 350 packages; the contribution requires 365. macOS is 347 → 363; the workspace feature closure is 534 → 545. The new ceilings equal those measurements, with no headroom. The reviewed SC core requires `sha2` 0.10 while MVM's Ed25519/signature stack requires 0.11; the exact duplicate-major declaration records that existing ownership boundary rather than changing either crypto stack or the core pin.

The complete final Rust tree has 2,995 files, **1,050,221 SLOC / 1,322,412 physical lines**, including generated, vendored and regression source. Both ceilings equal the measured totals. Freshly forced predecessor `4031a54d` accounting is 1,048,078 / 1,320,031, so the complete current-source difference is 2,143 / 2,381. The earlier nominal baseline used a cached executable compiled for another root and is explicitly invalid; the fresh baseline compiler JSON and actual executable receipt remain. Source counts are not endpoint memory funding or fleet capacity.

## Frozen-source and consumer execution

| Scope | Observed result |
| --- | --- |
| Final MVM format / strict workspace Clippy / all-target / explicit BDD / policy | Pass; all 86 policy gates clean |
| MVM outer binary harnesses | 21,810 passed, zero failed, 69 ignored; 284 targets; 19 nested summaries excluded |
| MVM doctests | 27 passed, zero failed, eight ignored; 29 summaries in 27 sections |
| SDK | 173 live, 29 owner, 74 supervisor and one addressed-journal pass; strict all-target Clippy and adapter/witness/guest compilation pass |
| Paired application | 18 passed, one explicit ignore |
| Canonical reader | Two passed, one explicit private-namespace/external-worker integration ignore |
| Native CI | All 35 executed checks successful; genuine-export skipped / NOT_RUN |

Full MVM invocation `a6d6943fda1c40e7be36b3d49340a828` retains all 4,715 before/after input hashes and an identical complete file set. SDK invocation `255e3888f47248fda32e6987dfce2ec9` retains all 485 SDK inputs and the MVM source. Final paired invocation `0654218c499243ecbe89b1b265383dae` retains 17 application inputs, the SDK/MVM snapshots, four selected executable identities for the registered-process witness (its test executable plus host-agent, signer and network-endpoint helpers) and its exact 4,152-byte frame fixture, SHA-256 `88cab4487a68269a244ee00c5ffef71868f2fdb3fbc2711cca0a79015cb426f5`.

The full test command records **16:00.44 wall / 1,001.30 s user / 193.04 s system**, including compilation. The entire qualification unit records 25:29.777 wall / 35:00.472 aggregate CPU and a 6 GiB cgroup peak, with no swap. These are qualification-work costs, not an endpoint memory grant or transfer/energy measurement.

The final registered-process fixture verifies **5,804 application bytes each direction**, with **3,657 TLS bytes sent / 3,234 received**. Setup is **67.279929 ms**; valid Open through verified half-close **16.110708 ms**; retirement including archive retry **8.036308 ms**. GNU time records **0.09 s wall / 0.02 s user / 0.02 s system** for the whole fixture. Those CPU counters are not phase-specific or sender/receiver-specific.

## Native bytes, time and resource observations

All seven native FlowMux rows pass 89 component tests and 20 ordered controls. Every row reconstructs exactly **67,108,864 application bytes in each direction** and cross-matches **5,395,050 / 5,395,015 actual TLS bytes** (10,790,065 total). The original two-second operation guards, 240-second bulk guard, security checks and test assertions remain.

| Native component row | Complete lifecycle seconds | Whole-process CPU seconds |
| --- | ---: | ---: |
| Linux x64 | 7.759047 | 10.942045 |
| Linux ARM64 | 6.786693 | 10.023281 |
| macOS Intel | 59.988553 | 39.981159 |
| macOS ARM64 | 29.355602 | 23.577215 |
| Windows 2022 x64 | 38.171264 | 43.593750 |
| Windows 2025 x64 | 63.553192 | 85.078125 |
| Windows 11 ARM64 | 125.917824 | 93.625000 |

[native-measurements.json](native-measurements.json) retains complete scalar observations, raw compound rows and identities. Setup is separate; unavailable per-side/phase CPU and RSS fields remain null. These are fresh samples, not a causal speedup claim, send-only time, physical energy or qualification of every sustained Windows workload.

All three native installed-store consumers execute eight original nonzero controls, bind all 4,715 exact MVM files and uniform five-package core `8994951b`, and preserve destruction-before-credit and poison custody observations. Their runner-recorded executable selection and before/after hashes match. Parent, endpoint and owner controls retain their distinct platform scopes. Native executable bytes are not in the ZIPs, so no downloaded-ELF rehash is claimed. These are native host-process components, not booted backend or WAN acceptance.

## Original failures and evidence limits

The packet retains the valid mailbox-profile and dispatch RED/GREEN, prerequisite failures, source-gate failures, invalid cached baseline, refactor compile errors, first paired export's missing-fixture exit 101, and three first-head CI source-identity refusals. The missing fixture was copied exactly and given its own before/after hash guard; no Rust assertion changed. CI at `d76d336b` correctly rejected the stale platform declaration. The corrected declaration accepts the new pin and the old declaration still refuses. Current passes do not relabel those failures.

The original 22-pair diagnostic retains its failed output, launcher and source/toolchain provenance; an independent digest of that original xtask executable was **not captured** before subsequent builds. The current full-workspace executable inventory is explicitly **post-execution**, not a historical prehash. Native before/after executable hashes are runner evidence, with no native binaries distributed. These limitations remain part of the result. #300's historical original-executable identity criterion stays explicitly incomplete; the current naming and full `check-all` gates pass. No missing historical digest is reconstructed or marked verified.

The [operator failure journal](operator/failure-journal.md) also records exporter/metadata preparation errors. Original file/source/driver distinctions remain explicit. Historical launch scripts retain their actual paths and commands; successful qualification is bound to the final invocation IDs and source snapshots above.

## Packet integrity and reproduction

The native packet contains **28 original ZIPs / 1,721,534 bytes / 811 members**. GitHub artifact digests and ZIP CRCs match; **644 embedded member hashes** and **15,353 source-hash rows** verify. Native inventory, checks and CI source receipt are under [native/](native/). Extracted duplicate copies are not published.

[operator/wick-records.zip](operator/wick-records.zip) losslessly retains 446 raw records totaling 63,003,157 uncompressed bytes in 9,721,140 ZIP bytes. Its SHA-256 is `6fca2a5fd6edc836fdbdd8ae8b641a303bd34b86de4b4752444f5a6fd6aab941`. Every record has a name, exact byte length and SHA-256 in its index. The archive includes the predecessor/current logs, original diagnostics, commands, resource limits, input guards and all eight graph comparisons. Compact records and original CI ZIPs are retained; payload stores, VM disks and 3.78 GB of observed Wick test executables were not copied locally.

From this packet directory, the record and count checks are:

```sh
python3 operator/verify-wick-records.py operator/wick-records.zip operator/wick-records.json
python3 operator/count-workspace-tests.py operator/wick-records.zip
python3 operator/verify-native-artifacts.py native ../../../.. operator/frozen-mvm-inputs.sha256
```

The native verifier accepts the native directory, matching SC source root and frozen MVM input manifest as its three arguments. It refuses changed source inputs. Independent source/security, command and artifact review verified the released slice and its scoped claims. No local Rust build target was created; existing user runtime/session files and shared remote Cargo caches remain preserved.

## Ari alignment and complete delivery gates

The fresh participation check covers the user's authored/commented-on MVM issues and PRs, including #1830/#1831, #4607, #4629, #5328/#5336/#5340 and #5479/#5480, plus current coordination #5057. Ari's latest runtime direction in #5515 is implemented by merged #5557 and adopted here. [#5520](https://github.com/tinylabscom/mvm/issues/5520) is a later single-entry-point design, with no scheduled implementation. [#5417](https://github.com/tinylabscom/mvm/issues/5417) remains an open contract-review gate; the fail-closed schema-9 reservation does not authorize an accepting producer or official publication. Protected producer #5425 and fresh admission/revocation #5426 retain their own acceptance. The old NIC proposal #4607 is closed as superseded by the host-mediated FlowMux path.

Ari must publish compatible signed runtime bytes. Source/component success is not that publication or a boot witness. Private MVMD integration and maintainer/contributor source-pin agreement under MVM #5057 remain external coordination gates. The keyless scheduler/VM-scoped keyholder path remains; no daemon cutover, FIPS/CUI, host-blind or physical-erasure assurance is granted here.

The complete [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42) goal remains:

1. Installed mailbox worker/VM ownership, aggregate budgets, shutdown and orphan recovery — #43, including remaining #258/#268 acceptance.
2. Final-ACK recovery, retention/expiry and sustained large/control traffic beyond the bounded session witness — #43/#49.
3. One shared-core FlowMux plus separately authorized owner access — #44/#45.
4. Canonical two-host WAN and the explicit Linux/Firecracker, macOS/HVF and Linux/macOS/Windows client/gateway role/version/architecture/backend matrix — #46/#47. Native Windows MVM hosting remains unestablished.
5. Capacity/fairness/backpressure/recovery/tails/saturation and a supported envelope before the complete reviewed upstream contribution — #49/#47.

Two guests and logical endpoints are not fleet capacity. Component tests do not close backend, WAN, FIPS/CUI or complete #258 physical/concurrency/original-enforcement criteria. No final upstream MVM contribution has been submitted.
