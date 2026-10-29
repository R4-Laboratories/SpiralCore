# Post-review diagnostic cleanup

Only the owned SC-OPT-004 diagnostic files were selected: 16 local `.log` files and
4 empty measurement stdout/stderr streams, plus 9 byte-identical packet log copies.
The helper requires exactly 20 local records/29 total selected files and the unchanged
qualification SHA before acting. It captures test cases/counts, original summaries,
commands, source-manifest references and available hashes, RED assertion details,
Clippy failure details and each original diagnostic hash/length. Intermediate setup
executions without an individually frozen source manifest are labeled explicitly;
they do not substitute for the final source-bound qualification.

The check mode reads and validates the entire allowlist without writing or deleting.
Apply mode first creates and synchronizes structured receipts in both locations, then
deletes only the selected owned files and preserves a per-file deletion receipt.
The original qualification hash is checked again after cleanup. No raw scientific
JSONL, analysis, source, template, compiler, dependency, executable identity or review
was removed or normalized. This operation ran in the parent-authorized quiet interval
after WAN shard 0 completed, before shard 1 was dispatched; it ran no performance probe.

Commands, from the owned local experiment directory:

```sh
/Users/casey.allard/.cargo/bin/rustc --edition 2021 cleanup-receipts.rs --extern serde_json=target-diagnostic/aarch64-apple-darwin/release/deps/libserde_json-1526d65a4d76f4d2.rlib --extern sha2=target-diagnostic/aarch64-apple-darwin/release/deps/libsha2-edcd0ba55ef603fc.rlib -L dependency=target-diagnostic/aarch64-apple-darwin/release/deps -o cleanup-receipts
./cleanup-receipts --check
./cleanup-receipts --apply
```

All commands exited 0. The read-only check selected 20 structured records and 29
files. Apply preserved those records, removed the selected files and verified the
original binding unchanged.

| Identity | SHA-256 |
|---|---|
| Rust cleanup source | `590be5a4cf0a7bedd7e151fa93e27644944a7f650657474251bf571539896dbb` |
| Native cleanup executable | `879c4bbde57fc5c2f9ca7a0041db3b41512bf2de57d6b17798550ffc9e2961ee` |
| Structured qualification | `1c7b2139982bb256fb5ca1bd2fc63fcc8bad510d9657c98e5c0bff80970644b0` |
| Cleanup receipt | `e2a8be868a9ddd773cadebee1da265e0e7cdf7802a64dbbdb3f4f840ae21a2ac` |
| Original qualification, unchanged | `f02cbb5722e69553567c10636d3754fea82a564431c0668d321847585e7a3295` |

The original premeasurement manifests intentionally still identify the removed log
paths and hashes. They are historical bindings, not current file-existence assertions.
Rerunning the old qualification/driver commands requires a newly generated, explicitly
identified qualification. Current packet `SHA256SUMS` and `checksums.sha256` are
regenerated after cleanup; the pre-cleanup packet manifest is retained locally under
`receipts/packet-before-cleanup.sha256` for custody history.
