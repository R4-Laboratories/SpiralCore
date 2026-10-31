# SC-MVM-001 evidence

One registered host canonical-envelope campaign, source
`3553af21ea8a2bd2cde70ab7a6c1317e0dbf0dad`, MVM
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b`. Outcome: 78 operations, 78 exact
stopped-store readbacks, twelve persistent sessions and twelve verified owned
stores removed. `run.exit` is zero. No retries, discarded warmups or WAN pooling.

- [measurement-contract.md](measurement-contract.md): dose and accounting registered before implementation/timing.
- [frozen.json](frozen.json), [frozen.sha256](frozen.sha256): source, hardware, lockfile and pre-run executable/source identities; all hashes checked again after execution.
- [transfers.jsonl](transfers.jsonl): every setup, operation, exact readback and cleanup/completion observation. SHA-256 `72f8c2778e368c2da47387b2e8ade945be9c07660b2a443fcbd7496ed5332444`.
- [summary.json](summary.json), [analyze.jq](analyze.jq): medians and paired ratios. Reproduce with `jq -s -f analyze.jq transfers.jsonl > regenerated-summary.json` in this directory. All groups have three trials; arithmetic uses matching size/trial/workload pairs.
- [qualification](qualification/): observed RED/GREEN, final fifteen-test success, Clippy attempts/final success, default check, release build, scoped coverage and independent review receipts. The source coverage predates only the example evidence-sync fix; production source hashes are unchanged. Raw command output is intentionally preserved, including its original whitespace, to meet the user's reviewable-evidence requirement.
- `mvm-issue-4629.json`, `mvm-issue-4633.json`: upstream prerequisite snapshots.
- `disk-before.txt`, `disk-after.txt`: storage receipts; shared Cargo targets were reused and incremental compilation disabled. No measured plaintext stores remain.
- `SHA256SUMS`: retained evidence index, excluding itself.

Actual counts are encoded canonical envelopes in both directions, not IP packets.
CPU/elapsed are sequential combined-host measurements, not endpoint-specific or
WAN time. Whole is a prepared zstd encoder with Raw fallback; these pseudorandom
fixtures use Raw. The fixed conformance clock and process-local replay do not
qualify a live mailbox service. The 600-byte historical HTTP model is recorded
separately from actual MVM envelopes. See the [result and limits](../../mvm-readiness-results-2026-10-31.md).
