# Frozen33 native source custody (baseline only)

All seven original artifact ZIPs from [run41777249828](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41777249828)
are retained unchanged. Each API SHA256/length matches; all42 inner checksum
entries verify. Each native executable scope is nonzero (9 per row), and every
row actually lists/executes33 tests with0 failed/ignored/filtered. Strict Clippy
and pinned Rust1.97.1 host triples match Linux24 x64/ARM64, Mac26 Intel/ARM64,
Windows2022/2025 x64 and Windows11 ARM64. The matrix job and seven native jobs
all completed successfully. The repeated231 role-test observations are not231
distinct tests or booted VMs.

Artifacts actually identify tested merge`fcf0555e1d60f8ccb068a630615b207aa4f40d94`,
not the PR head. Its API parents are`7e747983…` and reviewed head`57c09d9253f9fb509044b578dd278e777f744a13`;
its full tree`38fc292eab1dcfc5df2138f8b48f23eb219e0dc2` exactly equals that
head's tree. Source custody therefore does not infer equality from branch names
or from a green badge. Current-main changes through e66caf5a do not alter the
four-crate component closure/workflow/role manifest, as the retained empty
closure diff records.

The recorded lock hashes intentionally differ by checkout bytes: Unix LF
`2d5344f59c7734046d6a718fb77f7dc1ac66ba88522fcdd99e6c2cae0c9a9405`;
Windows CRLF`46526948b259a70ae36d3094afedfb81a595df42f22e8639bf22aaa390153e3a`.
Both match the exact committed lock under the corresponding line ending.
The artifact records source commit and lock identity, not independent hashes of
every compiled source file. The separate coverage snapshot hashes its159 inputs.

**This old source is not the final release candidate.** Independent review later
found fatal cancellation could release queued/buffered verified bytes. Its
33-test/platform success remains valid historical evidence but cannot qualify
the delivery-fence successor. Fresh34-test source1e69a99d requires its own native
matrix/custody. These ordinary-process TCP/TLS/shared-core tests establish no
native MVM backend, installed aggregate budget, sustained/WAN or whole-goal gate.

`verify-artifacts.sh`, API metadata, original ZIPs and `custody.tsv` retain the
checks. Only duplicate derived extraction was removed after validation; original
archives and failures remain. No workflow was rerun by this collector.
