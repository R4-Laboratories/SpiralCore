# Owned collector component and the remaining persistent integration

**The registered collector component is qualified; the original paired-MVM retirement failure remains open.** Contributor MVM [`841fec80e`](https://github.com/Casey-allard/mvm/commit/841fec80e56f7f65271c099b551afe431e5ffb60) contains the implemented successor. This is a Linux registered-process and runtime-ownership checkpoint, without a newly completed MVM/WAN witness.

## Implemented behavior

The existing profile-1 telemetry-only program now uses the same original registered endpoint transaction as the network program. It retains the actual Child, original host-service/audit registration, parent partition and original records/key files. The controller starts the owned collector, closes its original transport and joins its actual workers before Drained. The parent additionally requires the actual Child's collected successful exit before issuing an endpoint-retirement receipt. That receipt retains the original partition; it does not delete telemetry or refund its retained storage.

Collector mode requires one matching signed `Policy/network_policy` asset, exact deny-all policy and redaction continuity, no ingress/secrets, and retained credit. Runtime selection derives from the captured original admission. An unselected lifetime does not acquire this service; a selected collector with unsupported registered startup refuses before native dispatch. Poisoned admission selection conservatively keeps registration required. Mailbox/telemetry boot identity still uses the original registered guest-identity drive. Collector mode creates no FlowMux/network listener or egress grant.

The generic `RuntimeStateProtection` guard is unchanged. Records, wrapped keys, status and retention markers still refuse generic removal.

## Executed results

| Check | Result | Test execution time |
| --- | --- | --- |
| Registered endpoint suite, including six collector controls and existing exact network I/O | 9 passed | 0.37 s |
| Installed runtime ownership, startup failure and selection controls | 16 passed | 0.01 s |
| Host lifetime, original retained-file callbacks and poisoned selection | 11 passed | 0.00 s as printed by libtest |
| Existing runtime-state protection controls | 12 passed in the preceding component qualification | 0.00 s as printed by libtest |
| Strict Clippy, all targets of `mvm-core`, `mvm-vmm`, `mvm-hostd`, `mvm-runtime` | Passed | 40.64 s including checking |

The final 36-test/Clippy service completed with exit 0, 318.948 s wall and 583.865 s aggregate CPU, including compilation; reported peak memory was 3.9G. These are validation costs, **not transfer or codec timings**. Two compilation jobs, two CPU equivalents and a 6-GiB ceiling were enforced. The existing shared target was reused; no GPU, VM, raw payload corpus or local Mac build was used for this component qualification.

The recorded commands use Cargo 1.97.1 with `--locked --offline`:

```sh
cargo test -p mvm-hostd --test registered_direct_tcp -- --test-threads=1
cargo test -p mvm-runtime --lib workload_runner::runner::installed -- --test-threads=1
cargo test -p mvm-core --lib host_lifetime -- --test-threads=1
cargo test -p mvm-core runtime_state_protection -- --test-threads=1
cargo clippy -p mvm-core -p mvm-vmm -p mvm-hostd -p mvm-runtime --all-targets -- -D warnings
```

[Exact result rows, log hashes, source verification and limitations](results.json). The source snapshot matched all 4,659 tracked files, 64,689,193 bytes, at tree `7684672b40edffa64c7198a89d26b56ae9a0b30c`, with no mismatches. Transported Git metadata on Wick remains the original `468661f4` baseline; source identity here comes from the complete tracked-file/blob verification, not a substituted version banner. This is not final release packaging.

## Retained negative evidence

The first test failed compilation because the registered collector API did not exist. The first implementation compiled, then exposed two fixture construction errors: reuse of the bound broker socket and an unchanged empty redaction list. Their corrected controls passed. New validly signed wrong-policy and duplicate-policy cases then reproduced real missing binding checks; both refused before effects after the correction. The first runtime selection expansion also selected unadmitted collector mode; its executed regression failed before selection was narrowed to captured original admission. The machine record retains these failures separately from passing runs.

The next public SDK compile-contract was also executed and failed because the funded persistent method and owned start result do not yet exist at this source. Its [immutable fixture](sdk-api-contract-red.rs) checks signatures only and never polls a future. It is not a running guest or admission witness. A preceding fixture edit used the wrong `AssetIdentity` field name and failed compilation; that construction failure is retained separately and is not counted as a semantic regression.

## Remaining work for the original failure

The ordinary persistent journal API still lacks the actual funded host-lifetime context. Its stop path also reconstructs a backend and attempts generic runtime reaping; those operations cannot substitute for original installed native/collector retirement and protected retention.

The next implementation must carry the actual prepared lifetime and original backend through canonical persistent admission/dispatch, without starting a duplicate mailbox helper. SpiralCore's real shared parent must issue profile 1 with no codec store, retain the actual protected generation and accept the original endpoint retirement before finalizing journal stop. Failed stages retain their original owners, charges and evidence. Then execute complete clean and lost-receipt two-MVM retirement runs.

The preceding [current-source paired transfer](../2026-11-13-paired-sdk/native-retirement.md) verified 257 exact bytes and carried 5,792 B forward / 1,192 B return, but failed protected-state retirement and timed out. This component checkpoint does not change that outcome. Full workspace/features/BDD, native/platform/WAN, sustained capacity and FIPS/security acceptance remain open under [#111](https://github.com/R4-Laboratories/SpiralCore/issues/111) and [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
