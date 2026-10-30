# Independent broad16 evidence review

The latest checker also includes the schema repair in `qualification-capture-projection-correction.json`. Production capture rows intentionally emit `task: {id}`; the checker now joins that exact projection and phase to one already validated canonical receipt, checks status/timestamps/body totals, and obtains trial and arm from the full receipt. The failed checker and empty localized v1 output are preserved under `pre-capture-projection-correction/`. Ten tests pass. The first localized case was independently reviewed successfully into a new `independent-review-v2.json`; no measured transfer or data was rerun or altered.

The checker includes the **pre-measurement median-gate correction** recorded in `qualification-median-correction.json`. Warm acceptance uses raw median completion time divided by sparse median completion time, with a threshold of at least 2.0 and the unchanged 90% median HTTP-body reduction gate. `paired_wall_ratio` remains the descriptive geometric mean of paired ratios. No campaign data existed when this consistency correction was made. The prior source, executables, and qualification are preserved byte-for-byte under `pre-median-correction/`; the original `qualification.json` describes that predecessor. Nine tests passed at that correction, including a minority-outlier regression and the inclusive 2.0 boundary.


This Rust-only local tool is prepared before the broad WAN campaign. It performs no transfer, benchmark, networking, cache mutation, or subprocess execution. It links `serde_json 1.0.154`, `sha2 0.10.9`, and `zstd 0.13.3`, not the production benchmark or the author's analyzer. The pinned zstd library is used only to reproduce the registered compressed fixture and ordinary zstd payload sizes during offline review. The locked zstd-safe/sys versions match the transfer crate (7.3.0 / 2.1.2+zstd.1.5.7).

Builds and tests must finish before measurement; post-run evidence analysis must run after timing has stopped. The qualification contains no performance result.

For each completed registered case, first run the independent packet-total replay, then the receipt/statistics review. Use new output paths and preserve failures:

```sh
review_root=/Users/casey.allard/spiralcore/local/broad-wan-20261029/independent-review
campaign_root=/Users/casey.allard/spiralcore/local/broad-wan-20261029
case_name=localized_32
"$review_root/target/release/pcap-review" \
  "$campaign_root/campaign/$case_name" > "$campaign_root/campaign/$case_name/independent-pcap.json"
"$review_root/target/release/spiralcore-broad-independent-review" \
  "$campaign_root" "$case_name" \
  "$campaign_root/campaign/$case_name/aggregate.json" \
  "$campaign_root/campaign/$case_name/independent-pcap.json" \
  > "$campaign_root/campaign/$case_name/independent-review.json"
```

The checker independently reconstructs all 30 trial seeds, 24-permutation order cycle, per-arm namespace scopes, task identities, actual fixture lengths, and whole-object SHA256 identities. It requires all five shards and all four arms, with 240 warm or 120 cold receipts per case. It verifies selected mode, unchanged warming policy, absence of unexpected published-cache shortcuts, raw/zstd payload and frame/ACK/batch arithmetic, source/executable identity, and exact reopen receipts. It computes sender plus receiver handler CPU and seed plus transfer costs per row before midpoint medians, nearest-rank p95, geometric mean paired ratios, win counts, and registered warm/cold gates. All vectors must agree with the separately authored analysis.

The root source-qualified producer record is the trusted custody input. Each shard's frozen qualification and manifest, GitHub producer proof, validated archive digest/size, build-reuse receipt, expected build inputs, current features, and exact executable hashes must agree with those pins. This is an offline cross-check of retained authenticated workflow evidence, not a second live GitHub authentication or hardware attestation.

Capture review checks every row's application-body coverage, full partition sums, raw final tcpdump counter text, offload and pinned address evidence, bounded lifecycle ordering/quiet/final counters and hash bindings. Independent streaming PCAP replay validates every selected packet's IP header total length, direction and TCP443 ports, and campaign totals. The two results must bind to the same capture report and PCAP hashes. A complete-case 30-pair IP statistic is emitted only if all five captures are complete. Incomplete captures retain their reasons and never produce an automatic reduced subset.

The independent PCAP replay intentionally does not duplicate the production parser's TCP SYN ownership implementation. Per-row ownership continues to depend on that separately reviewed parser. PCAP captured header length is never counted as full packet length. Both tools reject malformed/truncated input. JSON is bounded at 8 MiB and PCAP at 256 MiB; inputs must be regular files. This local evidence checker expects exclusive custody of its inputs.

Reopened bytes were checked by the frozen receiver executable before its owned cache was deleted. This review independently reconstructs expected object identities and validates those retained counts/receipts and publication-hash verification records; it cannot reopen a deleted cache again. It does not independently reconstruct sparse or zstd-prefix encoding choices, but validates the reported mode and complete protocol-body arithmetic. Both are legitimate raw fallbacks before Begin. No raw body or CPU failure is excluded because packet capture is incomplete.

Eight unit tests pass, including retained real Linux normal/interrupted/ongoing lifecycle records, five altered lifecycle negatives, six receipt mutations, missing paired seed cells, altered numerical vectors, overflow, exact fixture hash/length binding, and canonical scope/order uniqueness. The historical real localized fixture golden IDs test compatibility only; no historical measurements enter the v4 statistics. Sixty plans generated by the frozen benchmark (all twelve cases and five shards) agree with the independently reconstructed tasks and scopes. A separately executed 32-byte truncated PCAP is rejected with `UnexpectedEof` and exit 101, without a successful output.

The retained lifecycle intermediate-text validator accepts the inline USR1 format observed on the pinned Linux runner. Production also supports a three-line form. This is an explicit compatibility restriction; an unrecognized format fails review rather than relaxing capture qualification.

Build qualification used native Rust 1.97.1 on aarch64-apple-darwin, default features, no timing. Commands were `cargo test --offline --locked --all-targets`, `cargo clippy --offline --locked --all-targets -- -D warnings`, `cargo fmt`, and `cargo build --offline --locked --release --bins`, all with this directory's manifest. Exact compiler and dependency tree are retained under `qualification/` and source/executable hashes in `qualification.json`.
