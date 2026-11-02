# Results delivery verification

Source implementation PR #29 merged as `e5b59c4bcd0791610634e0c707ad86965465850a`
after both required executions passed on measured source
`7e698aefe88f3729bc1b6e4f7ca72d9f036c110d`.

- Combined run `41394672925`: success; 24 sessions, 336 operations, 288 exact
  reopened deliveries. Linux core15/gateway38 tests, strict Clippy and release
  build passed before the registered traffic. No retry or failed campaign.
- TUN compatibility run `41394672864`: success; 12 sessions and48 exact flows.
- Combined original ZIP matches the GitHub API digest; 1,029 manifest entries,
  162 source entries and3 executable identities verified. Executed workflow
  blob `efdb9fba8e6efb520ca2ca0e6ac80f749e7280ee` matches the source workflow.
- TUN original ZIP matches the API digest; 862 artifact entries,43 source entries
  and2 binaries verified. Workflow blob
  `b7cbe295fb84ec8bc1b6d75a15884097c060dcd8` matches its measured source.
- Both originals were archived to the existing private dataset, then freshly
  force-downloaded at immutable revisions and compared by size, SHA-256 and bytes.
  Archive receipts distinguish independent custody checks from root manifest checks.
- The frozen Rust analyzer exited0 on the full original campaign. A second run
  on the compact Git-bound campaign produced byte-identical analysis JSON:
  `edaf981cce1391abbd2e570d0bdb64ae34c36fa225f2c67e5e2ad1f67bf36e17`.
  An independent reviewer recomputed raw session, operation and cumulative joins.

The results change only documentation/evidence, not measured source or fixtures.
Original raw outputs retain their whitespace and hashes. Full PCAPs and executable
files remain in the archive and are excluded from compact Git retention. No user
saved sessions, research work or other labs' files were changed or removed.
