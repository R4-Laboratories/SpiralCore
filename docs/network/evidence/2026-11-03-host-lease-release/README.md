# Frozen PR70 source release and hosted custody

**PASS.** [Source PR70](https://github.com/R4-Laboratories/SpiralCore/pull/70)
merged as86cae0a509deb44043265c0dd16128fd43509cb1 at2026-11-03T21:09:49Z.
Reviewed head731b5071 and tested merge556670cd share full tree
95881a0c3f922b208c75823fa67e4b320b19b28b with that actual merge. The two
merge API records identify ordered parents[e71f4080,731b5071]. Root performed
the merge; this collector changed no branch/product and pushed nothing.

- `pipeline-audit.md`: sequential Step1 PROCEDURAL PASS from existing scoped
  source/RED/GREEN/coverage records and final hosted custody.
- `code-infra-quality-gate.md`, `security-quality-gate.md`: Step2 confirmations
  for this frozen component, excluding current supervisor descendants.
- `release-decision.json`: exact source/tree/gate/count identities and limits.
- `owner-*.json`, `linux-*.json`, `reviewed-head.json`, `tested-merge.json`,
  `source-pr.json`, `actual-merge.json`: original hosted/commit/PR metadata.
- `original-archives/`: all four original owner ZIPs,12346B total, with matching
  API SHA-256 digests. No large build artifact was downloaded.
- `rows/`, `archive-members.sha256`, `custody.tsv`: seven original members per
  archive,28 retained hashes,20 compiled-file pins and native row/count checks.
  Expected pins come from **frozen Git blobs**, not current worker/controller
  source files. Linux15×2 plus Darwin16×2 gives62 entries including4helpers.
- `verify-release.sh`, `verification.*`: fixed bounded metadata verification,
  explicit failures, exit0 and empty stderr. Original execution path is the
  owned `/tmp/spiralcore-firecracker-host-lease-20261103/release`; script paths
  preserve that execution identity rather than pretending it ran elsewhere.
- `linux.log`: original full Linux run log; independently recounted115 live
  and138 shared(130adapter+8v1), with overlapping/zero scopes explicit. Strict
  Clippy, default guest check and both synthetic analyzers pass.
- `sealed-packet-check.txt`:80/80 checksum PASS for the separate81-file
  [source evidence packet](../2026-11-03-firecracker-host-lease/README.md),
  which remains sealed and was not modified by this release preparation.

No product build/test/CI was repeated for custody. The separate fork MVM7d
candidate still is not the default dependency. Actual installed controller/
worker activation, VMM lifetime, native positive recovery, full B6a/B6b,
retention/scale/WAN and whole goal remain open. There is no new native MVM,
Windows, performance or energy claim. Current supervisor branch work was never
checked out or pushed into PR70.

This is a new compact release folder outside the sealed source packet. Its
checksums.sha256 covers every other retained file and excludes itself. Root
owns any subsequent report/publication; original source qualification and
actual source merge are already recorded separately from that future action.
