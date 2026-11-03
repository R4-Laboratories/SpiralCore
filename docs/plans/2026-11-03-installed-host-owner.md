# Implement installed owner and full B6a resource leases

Base7c220cc8. Source/contract: [installed owner API](../api/installed-host-owner-v1.md), [architecture](../specs/2026-11-03-installed-host-owner-design.md), parent B6a/B6b plan. User authorized autonomous implementation; lead approves the two-lock/epoch design. Full goal remains host-wide budgets, long-lived native sessions, pipelining and canonical WAN/upstream delivery.

## Steps

1. **Focused RED harness** — `crates/spiralcore-mvm-live/tests/installed-owner-harness/{Cargo.toml,Cargo.lock,tests/lifecycle.rs}` compiles the absent exact product `../../src/installed_owner.rs` path. Prove missing source/API before implementation. libc-only, pinnedRust1.97.1, no VM/network. Riskmedium; contract precedes tests.
2. **Stable storage primitives** — `src/installed_owner.rs`: typed errors/key/epoch; private directory and file open/identity checks, in helpers of about50 lines or less. Reject unsafe objects without replacing locks. Riskhigh; step1 RED.
3. **Bounded durable state** — same module: fixed canonical record parser/encoder, checked generation, private temp/file sync/rename/directory sync, fencing. Keep at most current+one temp. Riskhigh; malformed/replacement tests.
4. **Owner EX and holder SH API** — same module: initialize/open/snapshot, begin/finish/reconcile, non-Clone leases and original epoch checks. Prove live-holder refusal, positive retirement/readmission and stale-launch refusal through real subprocesses. Riskhigh; storage helpers.
5. **Main crate integration** — `src/lib.rs`, `src/host_budget.rs`, `src/orchestrator.rs`, `src/main.rs`: explicit installed owner borrows guard before activation; budget clones retain installation ownership; no independent constructor emits host scope. Existing witness path stays explicitly orchestrator-scoped. Riskhigh; componentGREEN+independent security review.
6. **Actual lifetime custody** — `src/host.rs`, `src/linux_host.rs`, `src/orchestrator.rs`; additive concrete pinned-MVM backend lease API/launcher under reviewed separate source contract. Diskless sender/verifier and sudo pre-exec acquire SH and validate epoch; actual VMM retains it through exit. No fake disk, arbitrary privileged callback, idle-pool reuse or sudoers change. Riskhigh; typed ABI contract+fork-source provenance first.
7. **Full B6a recovery** — native Linux/Firecracker and macOSARM/HVF within total2guests/2vCPU/2GiB, owner+worker death with backend still alive blocks admission; positive backend exit→holdersEX→durable retirement→one new epoch. Default/verification paths covered. Keep scope unqualified until executed. Riskhigh; actual custody.
8. **Measured publication and continuation** — compact records, full affected gates/coverage/independent reviews, source PR then results PR. Record bytes/CPU/memory/full lifecycle against matched baseline where applicable; no performance claim from component tests. Update #42/#43/#49; fullgoal and B6b/C9/C10 stay open. Riskmedium; exact source qualified.

Focused helpers stay small; review/test each behavior before the next. No Mac full dependency build or payload store for component tests: a disposable bounded libc-only target may be used and removed after executable/test receipts are verified. Prefer Wick for native builds; its SSH availability must be rechecked, never infer a running process stopped from timeout. A temporary remote outage does not narrow the objective or authorize paid hosts.

## Acceptance

- [ ] Component source/API RED then real-process GREEN, exact durable records and typed refusal.
- [ ] Main installed entry closes every activation path; aggregate budget cannot multiply across owners.
- [ ] Actual worker AND actual Linux VMM/macOS HVF lifetime coverage, including sudo and diskless roles.
- [ ] Owner crash blocked while resources live, positive reconciliation/readmission after exit, late epoch refusal.
- [ ] Native bounded evidence and exact-source independent/security/CI gates; then B6b/C9/C10 and WAN continue.

A module or durable crash fence alone is partial B6a, not the requested final state.
