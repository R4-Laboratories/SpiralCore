# Retained failures and scope limits

- Original Root run5: 16 publications precede a closed guest barrier and an
  independent peer error. Preserve the original 64-block assertion. Raw Root
  GoAway text was NOT_CAPTURED; the isolated credit RED supplies that diagnosis.
- Test-only credit RED `54cf0ae1`: one exact regression fails at round 16 with
  canonical zero-credit GoAway. Exit 101, source/executable checks and raw output
  remain. Repair `41169543` passes the same exact selector; the complete scoped
  119-control run and separate full baseline are retained.
- Initial credit SDK metadata: rc.5 labels refuse the rc.6 source under locked
  resolution. Corrected metadata2 retains locked/offline mode and records only
  the intended owned-package labels. Initial paired metadata also lacked its
  SC source symlink; metadata2 fixes setup without changing product assertions.
- First native reader CI run refuses the stale paired guest lock under
  `--locked`. Final head bb188 changes only five MVM package labels. The original
  job identity and failure are retained in reader-lock-failure.json.
- An attempted footprint build lacked Git metadata. An old compiled xtask
  invoked from a new directory measured its old compile-time source root; that
  successful command is invalid evidence for the new source. Explicit-root
  unchanged counter/lexer records and freshly built final B2 xtask results
  retain the valid measurements separately.
- First admission merge `a0ee61bb` stops in strict Clippy on a large private
  enum before tests. B2 uses an opaque optional carrier without heap boxing or
  lint suppression. The earlier 4116 full run does not qualify B2; B2 has its
  own full test/86-gate/executable/source records.
- First positive Root execution matches the 490-input predecessor 1455939c,
  not final ee02. A subsequent strict Clippy run finds three redundant Result
  wrappers and a non-Drop borrowed-view drop. Final ee02 corrects those in three
  files; qualified2 reruns strict Clippy and the whole fixture with its own
  490-input guards and measurements. Do not transfer qualified1 timings to ee02.
- Cargo rebuilds three targets during the final MVM test command; their hashes
  remain identical. Of the selected 284 binaries, 260 were recompiled and 24
  are verified same-root reuse, not 284 fresh builds.
- Genuine signed export is skipped in both source PRs. Native artifacts contain
  runner-recorded executable hashes, not downloaded executable bytes. Sampled
  process resources are not deterministic transient peaks. No VM exit receipt
  is fabricated, no guest boots, and no WAN is exercised by the Root fixture.
- Official rc.5 archive existence and checksum do not prove authenticated
  contribution compatibility or live boot. The earlier compact inspection is
  linked from #268; no new signature/boot run is inferred here.
- Historical #300 original failing executable digest remains NOT_CAPTURED;
  current passing gates do not reconstruct that missing historical evidence.
