# Canonical history and committed-policy checkpoints

These development candidates continue the original-instance lifecycle work published in
[PR #331](https://github.com/R4-Laboratories/SpiralCore/pull/331). The installed consumer selection
remains MVM `f2ddbcbf` and shared core `8994951b`.

## Verified history and source identity

[MVM `ceae7dd9`](https://github.com/Casey-allard/mvm/commit/ceae7dd9594d1ab61a2f67f2191ca25b3882c3ce),
tree `799854e091c88e639ce36ec00965ec5234e81051`, integrates canonical
[#5664](https://github.com/tinylabscom/mvm/pull/5664) and
[#5655](https://github.com/tinylabscom/mvm/pull/5655). Seventeen of the eighteen changed paths have
canonical bytes. The client module exports compose the verified-history module with the retained
host-lifetime, direct-policy and retirement exports. Independent review verified the composition.

Verified readback authenticates the supplied audit chain and exact selected generation bindings
before returning sealed output. The selected route locates history. Delivery retains its existing
retention and retirement checks. The verified CLI command runs in the early read-only dispatch.

Linux x86-64 passed **120 tests** across five scopes. The counts were 13 verified-output, 96 stream,
one CLI early dispatch, one genuine compiled-checkout identity and nine existing output-binding
tests. The first stream run had 71 passes and 25 refusals because the SSH umask of `0002` made the
legacy fixtures group-writable. The identical ELF and 96-test selection passed under private
fixture umask `077`. The first run also recorded an empty selector that supplied no executed test.
Product checks and assertions were retained. The compiled-checkout case used a genuine Git
checkout with the candidate HEAD and index, verified separately against the source contents.

The tracked-file SHA-256 manifest covers 4,847 inputs and hashes to
`c956c676434cba3e8857006c055ff8802d9dd170287d2e5634a0d51993584fce`.
The successful test processes totaled 2.05 s wall, 1.34 s user CPU and 0.48 s system CPU. Highest
process RSS was 108,412 KiB. The two Cargo test-build commands took 117.36 s and 124.36 s wall.
Strict all-target Clippy passed for `mvm-build`, `mvm-core`, `mvm-client` and `mvm-cli`.

## Committed project policy and original owners

[MVM `6266ee94`](https://github.com/Casey-allard/mvm/commit/6266ee9470205d78fff15450059f853ce0074b14),
tree `1d00a585994f40a7e0adf3d24acaa94a8bc48aef`, integrates canonical
[#5654](https://github.com/tinylabscom/mvm/pull/5654). Existing named and paired ownership
guards remain in place. The canonical shared activation barrier spans persisted generation
validation and dispatch, and the committed generation check retains its Linux/Firecracker
requirement. Delayed native HVF activation holds the barrier before changing preparation state and
through boot. It preserves the earlier refusal for an invalid activation phase. The original
Firecracker path holds the barrier through its concrete runner. Independent review covered
dispatch, lock ordering, original custody and the dependency gate changes.

The final focused Linux run passed **160 tests in nine scopes**. These included activation-barrier
serialization, policy folding, backend quiescence, backend refusal, original installed grant and
stop controls, 61 named-launch tests and guarded rollback. The 4,865-file source manifest hashes to
`34776b5860097d331cfece4d5910f4db2b4ee4bad2778e8489d0fb3b38dc3bf4`.
Three compiler-selected test binaries were fresh. The other two matched the earlier exact hashes.
The test build took 36.85 s as reported by Cargo. Test processes totaled 2.07 s wall, 0.89 s user
CPU and 0.11 s system CPU. Highest process RSS was 63,080 KiB.

Seven measured graphs each added only the first-party `mvm-policy` package. The default/release
counts were 366/386 for Linux x86-64, 365/385 for Linux ARM64 and 362/388 for macOS ARM64.
The all-features count was 590. The existing ceilings moved by exactly that package where the
graphs were already gated. The guest ceilings were retained. Both closure gates passed, including
guest counts of 88, 84 and 85 at their existing limits. Strict all-target Clippy passed for
`mvm-core`, `mvm-build`, `mvm-policy`, `mvm-hostd`, `mvm-runtime`, `mvm-client` and `mvm-cli`.

## Remaining installed qualification

Both checkpoints record local component costs. They did not measure network traffic, per-endpoint
transfer CPU, or verified transfer lifecycle time. No live Firecracker or HVF boot, native macOS or
Windows execution, full workspace, WAN or capacity gate ran for either candidate. The source receipts,
exact executables, original failed run and validated archives remain on the HDD.

| Executable archive | Bytes | SHA-256 |
|---|---:|---|
| History and stream controls | 96,418,532 | `60536194fd8f2f2d76ec12c49c3fc5e57d0edeabfdf29e48fbbedd969c3770d8` |
| Checkout identity and output binding | 19,074,295 | `e6dfebd4f0bcdfb151e0a895c52f44dd9977dfc4cc8d96b268dc34fc6f08c7d9` |
| Final committed-policy controls | 110,801,408 | `c6c4ce1205c87cfd9b519b98a0712dc7e5f5042ad0bed3cbd3c313df71246b4f` |

Canonical [MVM #5668](https://github.com/tinylabscom/mvm/pull/5668) and later signed intent, retention
and epoch enforcement work remain outside the two candidates. Follow the detached coordinator and
the canonical epoch gate for later reconciliation. The request for the installed audit-health and
original-deadline interface remains unanswered. The compatible signed guest runtime and original
installed readiness are still open. [Ari’s coordinator update](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-7056911902).
[Audit interface request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842).

The joined resource envelope and installed storage witnesses in
[#258](https://github.com/R4-Laboratories/SpiralCore/issues/258), private MVMD, and the complete
delivery requirements in [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42) remain open.
