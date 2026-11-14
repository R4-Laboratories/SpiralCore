# Current-source paired guest role inputs

The two static x86-64 Linux musl role executables now build against contributor MVM `468661f47e01d0f59c6f1da4e35a968ceccb3ad2`. The role source and SpiralCore path dependencies were staged from `0941abea5e1122bd594477931288eda9d7c556c7`; the host driver retains its separately qualified source identity from the [paired SDK report](README.md).

| Role | Bytes | SHA-256 |
| --- | ---: | --- |
| Source | 1,952,144 | `c9a68bbb571bb54b9155a187fe0e496157cfe719b563b83ffce421a7611c59d9` |
| Destination | 1,952,144 | `c936f851db871551ffb86bce8f54d9f53405d3cdaca51c0ce80875a646474773` |

Both passed MVM's existing `validate_static_guest_elf` for x86-64. Independent `readelf` checks found no interpreter or dynamic dependency. These are application role executables, not a complete runtime, sealed workload image or native delivery result.

## Dependency and toolchain preparation

Normal Cargo resolution fetched the existing exact canonical Git pin `48fa5b488c4543f0f7988df722633554663a9764`. The paired workspace patch selected local contributor `mvm-agentd`, `mvm-contract` and `mvm-core`; the original upstream source pin was preserved. Five local MVM package versions moved from rc.1 to rc.3, with **all 192 existing registry package/version/checksum rows unchanged**. The resulting guest lock SHA-256 is `48b0baf3ed095b12bfdd5d04e71f801b87df3e92299dc4637fa0153f3c60e4a4`.

The first offline attempt refused the missing Git cache. Plain rust-lld compilation then reached zstd-sys and ring's need for a musl C compiler. Existing MVM-managed Zig 0.13.0 and cargo-zigbuild 0.23.0 supplied the target-correct tools, with a private wrapper cache and existing shared Zig cache. No system install or glibc substitution was used. A subsequent missing shared-fixture error was corrected by restoring the original layout: the guest and `fixtures` directories are siblings, and the source, destination and host use the same immutable fixture bytes. All three failures remain retained separately from the successful build.

The successful command used Cargo 1.97.1 `zigbuild --release --locked --offline`, target `x86_64-unknown-linux-musl`, selecting `spiralcore-durable-source` and `spiralcore-durable-destination`, with two build jobs and a 6 GiB enclosing memory cap. The existing shared Cargo target was reused. The managed-tool hashes are observed identities, not a new download-provenance claim: Zig `7f9e3a661e909d5188d1b8b14f082b98a19c323a30d43bfdd1b2893ed37273e0`, cargo-zigbuild `84987c8fd31f12cda90d38172040dc83e083e818b44c2ae2f7668b695450409b`.

Original build, lock-resolution and static-validation records remain in private Linux directory `paired-0941-468661.ra3f9xwc`. Runtime preparation through the existing source-runtime builder is the next separate step. A contributor-built runtime is distinct from the signed official rc.3 candidate used by the earlier FlowMux witness. No current-source paired guest, recovery, WAN, sustained-session, fleet or FIPS acceptance is claimed here.
