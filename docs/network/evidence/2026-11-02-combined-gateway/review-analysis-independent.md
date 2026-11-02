# Independent campaign arithmetic review

Reviewer: `mvm_tests`. Date: 2026-11-02. Verdict: **PASS; no arithmetic correction required.**

Reviewed run `41394672925`, measured source
`7e698aefe88f3729bc1b6e4f7ca72d9f036c110d`. Source, binaries and archive custody
were the root reviewer's separate responsibility. This review used raw
`client.json`, `server.json`, and `capture.json` from the 24 session directories
under `/tmp/spiralcore-combined-41394672925/combined/campaign`.

Independent jq extraction and JavaScript arithmetic were used, rather than
rerunning the production analyzer. Process CPU was added within each matched
trial before taking the median. Comparison ratios were computed within each
trial before taking their median; independent medians were not divided.

Reconciliation found **zero mismatches** against `analysis.json`:

- All 24 complete-session IP-byte, logical-byte, process-CPU and client-wall rows.
- All 336 operations, joined to receiver batches by session and transfer ID:
  channel deltas, sender CPU, summed receiver handler CPU, endpoint-stage CPU,
  client wall, batch count and request-frame count. No orphan receiver batches.
- All 336 paid-group cumulative rows and 336 connection-prefix rows, including
  group preparation, both Combined auxiliary publications, the namespace setup,
  and preceding groups where applicable. Setup was not omitted or counted twice.
- 288 deliveries, 2,289,396,384 requested bytes; operation counts of 276 sends,
  12 Warm target seeds, 24 schema publications and 24 checkpoint publications.

## Complete mixed-session results

Independent medians of three trials; CPU and elapsed are seconds.

| Link | Arm | Captured IP bytes | Summed process CPU | Client elapsed |
|---|---|---:|---:|---:|
| Unshaped | Whole-zstd | 40,266,059 | 1.505146 | 1.126738 |
| Unshaped | Warm | 33,395,072 | 1.309050 | 0.990653 |
| Unshaped | Runtime | 33,395,197 | 1.459759 | 1.142403 |
| Unshaped | Combined | 16,719,671 | 1.197458 | 1.040818 |
| 80 ms / 10 Mbps | Whole-zstd | 41,295,031 | 3.133881 | 35.520383 |
| 80 ms / 10 Mbps | Warm | 34,235,354 | 2.662683 | 29.867256 |
| 80 ms / 10 Mbps | Runtime | 34,229,935 | 2.792965 | 30.221147 |
| 80 ms / 10 Mbps | Combined | 17,237,193 | 1.911030 | 16.821350 |

Combined's median paired reductions:

| Link | Control | IP bytes | Process CPU | Elapsed |
|---|---|---:|---:|---:|
| Shaped | Whole-zstd | 58.2588% | 38.0828% | 52.6526% |
| Shaped | Warm | 49.6513% | 27.8432% | 43.6796% |
| Shaped | Runtime | 49.6429% | 31.5770% | 44.3087% |
| Unshaped | Whole-zstd | 58.4776% | 20.4424% | 7.8721% |
| Unshaped | Warm | 49.9336% | 8.3117% | **−4.7834%** |
| Unshaped | Runtime | 49.9338% | 17.9688% | 8.8922% |

## Setup and losses retained

For the shaped small tier, Combined setup is 4,416 logical bytes and 169.297 ms;
setup plus first delivery is 5,187 bytes and 344.102 ms. For the large tier,
these are 4,140 bytes / 197.768 ms and 4,911 bytes / 519.660 ms. Both checkpoint
and schema operations contribute to these totals; the later group's actual
already-present auxiliary handling is charged. Through-six totals also match
the report: 1,058,383 bytes / 2.108987 s and 14,892,081 bytes / 14.270994 s.

Confirmed losses and tradeoffs using paired ratios:

- Small setup-plus-first uses **33.8367% more endpoint-stage CPU** than Whole,
  despite **18.6334% lower elapsed**. Large setup-plus-first saves 13.6262% stage
  CPU and 82.6187% elapsed.
- Large generated Combined and Runtime both transfer 771 logical bytes, but
  Combined uses **4.28456× Runtime sender CPU**. Their median sender times are
  100.379 ms and 23.574 ms. The comparison does not isolate a particular encoder
  cost. Median endpoint-stage CPU is 157.926 ms versus 80.280 ms.
- Large unrelated objects select Raw in every arm and transfer the same
  14,883,962 logical bytes. Combined has **13.7899% higher sender CPU** and
  **0.3935% higher elapsed** than Whole. Small unrelated sender CPU and elapsed
  increase 8.9464% and 16.7512%, respectively.
- Warm is faster than Combined on the unshaped mixed sequence. Combined's
  complete-session win does not imply an operation-by-operation CPU or time win.

The numerical tables and percentages in
`docs/network/combined-gateway-results-2026-11-02.md` agree with this raw
recomputation, including separate sender/receiver medians. Review identities:

- `analysis.json` SHA-256:
  `edaf981cce1391abbd2e570d0bdb64ae34c36fa225f2c67e5e2ad1f67bf36e17`.
- Results report SHA-256:
  `770b6f1d920061b7bda4a17f9a771fb331ef1310e64a44956b466aaf0dc22b87`.

Limits remain explicit: three descriptive paired trials, local calibrated network
impairment rather than Internet WAN, captured IP bytes only at session scope,
and nested operation/handler CPU distinct from complete process CPU. CPU must
not be added to elapsed or interpreted as physical energy. No benchmark rerun,
new test, production edit, or broader artifact audit was performed for this review.
