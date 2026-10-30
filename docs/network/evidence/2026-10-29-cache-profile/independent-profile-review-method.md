# Independent SC-OPT-004 receipt review

The review runs only a separate Rust analysis executable after the measured interval. It does not invoke the endpoint, rerun a benchmark, edit the diagnostic snapshot, or alter the author's analysis.

Run from `/Users/casey.allard/spiralcore/local/cache-profile-20261029`:

```sh
/Users/casey.allard/.cargo/bin/rustc --edition 2021 independent-profile-review.rs --extern serde_json=target-diagnostic/aarch64-apple-darwin/release/deps/libserde_json-1526d65a4d76f4d2.rlib -L dependency=target-diagnostic/aarch64-apple-darwin/release/deps -o independent-profile-review
./independent-profile-review /Users/casey.allard/spiralcore/local/cache-profile-20261029 > independent-profile-review.json
shasum -a 256 -c build-diagnostic/executables.sha256
shasum -a 256 -c build-diagnostic/helper-files.sha256
shasum -a 256 -c qualification/qualified-inputs.sha256
shasum -a 256 -c measured/observations.sha256
cmp measured/templates-before.sha256 measured/templates-after.sha256
cmp measured/templates-before.sha256 templates-established.sha256
```

All commands exited zero. The independent program reconstructs the two-block population/arm order, checks source/executable/fixture identity, status, publication and manifest changes, graph retention, frame sequences and responses, modeled application bytes, exact scan API counts, stage sums and containing clocks. It derives all 16 metrics for all 16 cells directly from integer raw receipts, then compares all 256 author metric records: original vectors, midpoint medians, minima and maxima. Floating-point comparisons allow at most `1e-12 * max(abs(expected), 1)` for representation differences; integer invariants are exact.

Reviewed evidence hashes:

| Artifact | SHA256 |
|---|---|
| Independent Rust source | `e6714a6d0254a137e799fdd13b029420dc89877c9adb62b8e1c46498bd2188e1` |
| Independent analysis executable | `1e62df7264080d6aa25de71a2ac96f9184b0c346d0b4bd8a77ce4b6754fc0aba` |
| Independent JSON result | `416536a6be5eb4e5ec54f19eb3028be87324ab832f9056037da23bb2f0ce2523` |
| Author profile-analysis.json | `192273e23ea38379f5a434d0f823e52b86f00894ba41dbb237966bddcd5d41f9` |
| Block 0 raw receipts | `febc5f1e00676f6c37c9f2994950dc591001a59796c7b6bb562f823a5ce9774f` |
| Block 1 raw receipts | `77b150d85b26d18448410d6869452697fab32bcf734568019dcf3537583fd839` |

The preserved execution receipt contains exactly blocks 0 and 1, each exit zero. The before/after template manifests match the established manifest. This verifies the recorded experiment and arithmetic, not an independent replay of its byte comparison or a proof about arbitrary filesystem mutation. Two observations per cell support descriptive attribution only. Instrumentation overhead remains inside the nested scan and handler clocks. API call counts are not operating-system syscall counts; there is no network or energy measurement.
