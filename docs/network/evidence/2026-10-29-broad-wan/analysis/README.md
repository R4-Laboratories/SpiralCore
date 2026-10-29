# Broad WAN numerical analysis

Frozen Rust analysis source and locked dependencies. Run with Rust1.97.1 and one completed case directory containing shard-0 through shard-4. Each shard must retain remote/rows.jsonl. This tool does not transfer data.

All thirty paired observations are retained; it computes per-row combined CPU and lifecycle costs before midpoint medians and nearest-rank p95. The warm threshold uses the ratio of median elapsed times; paired geometric mean remains separately descriptive. The pre-measurement correction and observed RED-to-GREEN regression are preserved. HTTP-body thresholds are distinct from whole-path byte acceptance and exact reopened verification.

Five tests, Clippy, formatting and release build passed before measurements. Executable SHA256: `5b312a269a716b2a540de998f0bc17aea4f23c0a99f622510e488a99944eeee4`. Independent review is preserved in the localized case packet. Original sender/receiver measurements are never rerun by this analysis.
