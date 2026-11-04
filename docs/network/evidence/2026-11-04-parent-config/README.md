# Original VM config and atomic registration admission

2026-11-04. Implemented component of [mailbox issue #43](https://github.com/R4-Laboratories/SpiralCore/issues/43), not completion of the installed service or [full MVM delivery #42](https://github.com/R4-Laboratories/SpiralCore/issues/42).

The installed parent now binds the actual opaque admitted plan to the original config/launch anchor and SAME private VM grant. It permits a generated VM name to differ from the signed workload label only on that verified config path. The older binding-only API retains its workload/name restriction. Canonical helper registration continues to identify the actual config VM; neither a digest nor equal projected fields supplies authority.

The registration candidate and its actual 32 KiB metadata reservation prepare before the short atomic admission/session/registration commit. An unsupported preflight cannot consume the binding. Losing candidates release after the lifecycle mutex drops. Registration retains a Weak launch anchor, avoiding a grant/anchor cycle.

## Executed source

- SpiralCore code candidate: [`8ff9e5597b95e0057ce394026960fce3feb90979`](https://github.com/R4-Laboratories/SpiralCore/commit/8ff9e5597b95e0057ce394026960fce3feb90979), parent `0bd0f4d0`, tree `9932826e3bfd8e86afab01edbd092f1ffab11704`.
- MVM SDK: [`bef4146c0de18b22ac392118acfd898a369cc64c`](https://github.com/Casey-allard/mvm/commit/bef4146c0de18b22ac392118acfd898a369cc64c), parent `f708539f`. Only three additive API files change: Core original-owner identity, Hostd config-aware parent binding, Client nonconsuming registrar installation.
- Adapter binding: [`7f638428522e70b36a82d68700a6374267ea7e40`](https://github.com/R4-Laboratories/SpiralCore/commit/7f638428522e70b36a82d68700a6374267ea7e40). Its two manifest/lock changes preserve all product/test bodies and all 196 package name/version pairs. The older lock had stale source identities; the [review](adapter-pin-review.md) details the repair rather than calling it a literal SHA-only replacement.
- Shared transfer/core remains `ca5d9ca1`. Final metadata has exactly one MVM Core, one Contract and one shared transfer core, with no path overrides. Every package name/version pair remains unchanged from the previous live lock.

All **131 supplied source inputs** match the candidate's Git blobs; `candidate-source-verified.json` records the check. The fetched SDK has no tracked difference from its pinned commit. Source hashes before and after execution/Clippy agree. Protected test-binary hashes agree before and after execution. The compact packet retains commands and explicit status/hash JSON; binaries and full source trees stay on Wick.

## Actual failure and correction

The first Rust metadata probe against the unchanged published f708 Hostd library fails E0599: `bind_config` is absent. Its selected library hash is unchanged before/after.

The first larger extraction fails before tests because eight exact sibling transfer inputs required by `include_bytes!` were missing. Restoring those immutable 0bd files changes no product algorithm. The corrected release build executes four new cases: **three pass and one fails** with `registration preflight consumed admission`. That failure reproduces the independent review's initial HOLD.

Preparing the real registration candidate before lifecycle publication fixes it. Both the isolated diagnostic path-binding run and the final public Git-dependency run execute **19 PASS / 0 FAIL / 1 ignored**. These are overlapping runs, not 38 distinct tests. Four entries are new:

1. Actual admitted workload label maps to its original generated VM and registration exactly once.
2. Another actual owner with equal projected installation/binding fields cannot bind the original grant.
3. Changed signed plan body and config tenant refuse before parent binding, with unchanged managed usage.
4. Unsupported registration preflight leaves admission/session/registration and managed usage unchanged; correcting the same config permits the original binding.

The other 15 passing entries are existing parent, endpoint, resident and retirement-policy regressions. Several use explicitly private policy fixtures; none supplies a fabricated native exit receipt. Two resident tests intentionally catch controlled panics, whose stderr remains in the packet. The normally ignored real-helper test was **not executed by this checkpoint**; its earlier real-process result retains its prior source identity.

A mixed Git graph initially selected both bef and f708 Core/Contract identities and was refused. The manifest/lock-only adapter successor fixes that graph. Final source qualification uses its public immutable dependencies, not the earlier diagnostic path override.

## Final commands and limits

From the full source checkout, the release parent scope is reproducible with:

```sh
cargo +1.97.1 test --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib installed_parent:: \
  -- --test-threads=1
cargo +1.97.1 clippy --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib --tests -- -D clippy::all
```

The actual test invocation uses a copied, protected release executable after `cargo test --no-run`; commands are in the archive. Both release build and test process exit zero. Strict Clippy exits zero with **13 acknowledged Rust warnings** for unconnected component paths, not a warning-free build. Its first attempt fails target resolution because the extraction omitted the aliased sibling adapter test/common files; restoring only their exact 0bd bodies fixes that prerequisite without removing `--tests`.

These tests boot no guest, register no VM, transfer no object and establish no new CPU/byte/WAN advantage or capacity bound. The shipping resource profile remains inactive. Canonical resident/helper integration, real per-VM registration, retained telemetry ownership, positive endpoint retirement, sustained recovery, platform/WAN acceptance and FIPS qualification remain open.

`receipts.tar.gz` has SHA-256 `5dfe6cb0755cb6e773fc6a12bd98e594411b89f68c23a89d5fad68afa67fa671`, 854,025 bytes, 83 hashed members plus the checksum index. Every regular member verifies. No payload store, VM disk or executable is copied locally. Original failed records stay preserved; only compact public records and source are retained here.
