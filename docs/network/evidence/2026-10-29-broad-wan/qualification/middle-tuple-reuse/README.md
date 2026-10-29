# Middle-insertion capture: observed four-tuple reuse

Frozen source: `9dce4c57d1f41cef7365b297cd690849afce34e7`. Case `middle_4096`, shard 1. The recorded capture remains **incomplete**. No performance trial was replaced, no raw evidence was edited, and no production parser change was made.

The native diagnostic independently parsed all 447,603 PCAP records. Captured and received counters both equal 447,603, kernel drops are zero, and lifecycle qualification passed. There are 48 outbound SYNs across 47 distinct four-tuples; every captured tuple has an observed outbound SYN. One tuple has two different initial sequence numbers:

`10.1.0.84:46852 → 104.16.230.132:443/TCP`

| Observation | First use | Second use |
|---|---|---|
| Task | `16773120-middle_4096-06-raw_full` | `16773120-middle_4096-11-sparse` |
| Phase | transfer | seed |
| Outbound SYN record, one-based | 12,496 | 435,097 |
| SYN timestamp, Unix nanoseconds | 1793314608097330677 | 1793314677289138639 |
| Initial sequence number | 845141602 | 1926263598 |

Both SYN/SYN-ACK pairs are present. The first use ends with FIN, peer FIN/ACK and final ACK at records 24,936–24,938. Its last observed packet precedes the second SYN by **66.849409911 seconds**. These are directly observed headers, not an inference about kernel timeout settings.

The frozen parser deliberately marks a four-tuple ambiguous when a later outbound SYN has a different initial sequence number (`spiralcore-capture.rs:565–568`). Its second pass sends **every packet on that tuple** to the unattributed bucket (`:619–620`). Independent replay exactly reproduces that bucket: **24,928 packets, 34,974,525 IP bytes**, including 34,898,699 upload and 75,826 download bytes. This explains exactly two rows with zero assigned packets and application-body undercoverage. It does not mean those two transfers had no captured packets.

Continue the frozen campaign with its existing incomplete-capture policy: retain all application-byte/CPU/wall observations, retain the incomplete report, and suppress the complete 30-pair IP claim for this case. Equal counters do not prove absence of all possible capture loss. An epoch-aware parser would require a separately specified state machine, delayed-packet and sequence-wrap ambiguity handling, regression tests and new qualification; this diagnostic does not approve retroactive row reassignment.

`diagnosis.json` preserves the exact headers, small handshake/teardown excerpts, diagnostic SYN-time segments and raw-input/source/executable hashes. Segments are investigative groupings, not replacement production IP accounting. The first reader attempt incorrectly required monotonically ordered packet timestamps and stopped before producing a diagnosis. Its source and failure record are retained in `attempt-1/`. The corrected diagnostic counted **134 adjacent timestamp inversions**, permitted them as the production parser does, and sorted only its explanatory excerpts by timestamp. No raw record was reordered or changed.

Full local diagnostic directory: `/Users/casey.allard/spiralcore/local/broad-wan-20261029/middle-capture-investigation`. Raw evidence remains under `campaign/middle_4096/shard-1/remote`. Binaries and debug stderr are omitted here. Retained executable hashes:

- Completed diagnostic: `6b71f93507da526513a010349624ea4bbc8ff838cebd597e8618b0da8bd5f210`.
- Failed initial diagnostic: `1512a6ed7577b953fef2ca04e71ba6dcfab4b612cad2d0530ac293c09102cb14`.

The reviewer was built with native `rustc 1.97.1`, edition 2021, `-C opt-level=2`, using existing serde_json/sha2 rlibs from the source worktree's `crates/spiralcore-transfer/target/debug/deps`. Invocation: `review RAW_REMOTE_DIRECTORY NEW_DIAGNOSIS_JSON`; output creation is exclusive. The completed run returned zero. It was performed while transfers were paused, and all PCAP scans/builds stopped before the next timed case.
