# Execute candidate CPU optimization

Contract: `docs/specs/2026-11-02-candidate-cpu-design.md`. Reuse existing isolated
worktree and shared target directory; preserve user files and historical evidence.

1. Core owner adds a small Rust candidate-stage profile (gateway example using
   canonical fixture generation or equivalent identity-checked inputs), tests its
   bounded dose, and records the baseline before production edits. If evidence
   contradicts the proposed bottleneck, narrow the change accordingly.
2. Core owner implements focused RED/GREEN changes in transfer schema selection,
   schema-zstd/probe helpers and only needed prepared-prediction glue. Exact
   canonical-output comparison, ties, compressible controls, probe fallback,
   malformed input and existing session negative tests guard the change. Medium
   risk; independent specialist reviews proof and allocation boundaries.
3. Measurement owner extends the existing fixture/harness/analyzer with an
   explicit three-arm candidate-CPU policy and a bounded workflow that builds
   pinned before/current binaries. Preserve historical policy defaults and
   byte-identical historical analysis. New evidence stays under
   `docs/network/evidence/2026-11-02-candidate-cpu`.
4. Root cross-reviews infrastructure; independent reviewer checks production.
   One Cargo window at a time, targeted transfer regressions and final gateway
   checks/Clippy. Commit frozen source and run one registered matched campaign.
   No builds overlap timed work. Diagnose actual failures and retain receipts.
5. Root reproduces results, obtains bounded independent arithmetic review,
   archives verified original ZIPs, writes report/ledger, and merges source and
   results through PRs. Sync both checkouts to origin/main. No hidden losing case,
   invented send-only time, broad cleanup, or unrelated implementation work.
