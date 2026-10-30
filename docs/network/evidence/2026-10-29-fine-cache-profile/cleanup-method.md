# Authorized post-review text-log retirement

The parent authorized retiring only this diagnostic's owned verification text logs after the independent review. The explicit selection was 37 `.log`, `.stdout`, and `.stderr` files under `qualification/`, `receipts/`, and `measured/`, totaling 38,110 bytes. Numeric `.exit` files and per-process host observations remain. No Cargo target directory or earlier experiment was touched.

The Rust method is `independent-review/retire-logs.rs`. It bounds reads, requires canonical regular files from a fixed allowlist, requires the successful independent-review receipt and pinned original qualification, and checks exact selection count/bytes. Its read-only `--check` was run and inspected before `--apply`. Apply verifies that the check receipt still binds the identical prospective records, writes and synchronizes `structured-qualification.json`, synchronizes its parent directory, reopens and compares the complete JSON, and only then removes the selected files. Each removal is separately recorded and synced in `independent-review/log-retirement-journal.jsonl`; the final receipt is `diagnostic-cleanup.json`. All protected input bindings were checked unchanged afterward.

The structured replacement preserves all original file hashes/lengths, individual test names/statuses, exact test totals and original summaries, assertion/error blocks including the initial OS92 unsupported filename failure, warning details, build unit/version lines, recorded binary invocations, checks, stage-completion output, and commands wherever actually retained or confirmed by the author. Some intermediate Cargo argv were not recorded exactly; those are explicitly unavailable rather than reconstructed from assumptions. Source contexts without an individual frozen manifest are labeled accordingly. These records do not replace final source-bound qualification with intermediate results.

The final pass counts remain 15 helper/analyzer and 136 snapshot tests, plus one explicit ignored test. The retained negative evidence remains three corrected-category runtime failures, two analyzer-safety runtime failures, and the earlier OS92 setup limitation. Exact command files, source manifests, qualification JSON, schedules, all scientific JSONL/analysis, template data, executable identities, and independent methods/results remain unchanged.

Original premeasurement qualification/manifests intentionally still name the retired text paths and their historical hashes. They have not been rewritten to make a past check appear current. A direct rerun of the full pre-retirement custody checker or former driver now requires the historical text inputs; it will fail on their absence. All numerical observations, independent recomputed statistics, and structured verification evidence are retained. This retirement ran no endpoint, benchmark, or performance observation.

Commands ran from this experiment root with exit status zero:

```sh
/Users/casey.allard/.cargo/bin/rustc --edition 2021 --target aarch64-apple-darwin independent-review/retire-logs.rs -L dependency=target-diagnostic/aarch64-apple-darwin/release/deps --extern serde_json=target-diagnostic/aarch64-apple-darwin/release/deps/libserde_json-1526d65a4d76f4d2.rlib --extern sha2=target-diagnostic/aarch64-apple-darwin/release/deps/libsha2-edcd0ba55ef603fc.rlib -o independent-review/retire-logs
independent-review/retire-logs --check > independent-review/log-retirement-check.json
independent-review/retire-logs --apply
```

The check output redirection used shell noclobber. The successful final deletion receipt binds the method source, native executable, structured replacement, unchanged original qualification, and all selected original hashes. It records 37 removals and 38,110 removed bytes. The structured qualification SHA256 is `ea85e2ce6cce3ebf1523aab4fa980ad0446404a86950d2a1a57094b33ace8faa`; the unchanged original qualification SHA256 is `7a51e3a886b644c1b71ff829479439873eba06f2c6d52c38034159a5492f2c5e`.
