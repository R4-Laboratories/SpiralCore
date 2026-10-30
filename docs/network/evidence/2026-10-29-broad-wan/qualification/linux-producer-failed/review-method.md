# Failed producer 40947946641: independent review

This directory preserves the failed first final producer at source `16fe9f44e858053a202b396fb7fca628dfcfd97c`. It is not a reusable producer and must not supply campaign gate files. No WAN measurement ran. No Linux executable was downloaded or executed locally because sender sealing and upload were skipped after coverage timed out.

The authenticated GitHub CLI fetched the run, one-job step list, artifact list and evidence artifact metadata into the four `*-api.json` files. `gh run download 40947946641 --repo R4-Laboratories/SpiralCore --name spiralcore-wan-40947946641-1 --dir .../remote` retained the failure evidence. `gh api repos/R4-Laboratories/SpiralCore/actions/artifacts/12929774775/zip` retained `evidence.zip` under a 1 MiB file limit. The native failure reviewer independently checks that its 52,488 bytes and SHA256 match the authenticated API digest. `gh run view ... --log-failed` supplied the exact failed-step diagnostic; its digest is recorded in the final JSON.

Only native local Rust reviewers were executed. From the source worktree, both reviewers were compiled with:

```sh
/Users/casey.allard/.cargo/bin/rustc --edition=2021 -C opt-level=2 REVIEW_SOURCE \
  -L dependency=crates/spiralcore-transfer/target/debug/deps \
  --extern serde_json=crates/spiralcore-transfer/target/debug/deps/libserde_json-f11651c4b5834b4e.rlib \
  --extern sha2=crates/spiralcore-transfer/target/debug/deps/libsha2-11f7c61dcc994809.rlib \
  -o REVIEW_EXECUTABLE
```

Compiler: native macOS `rustc 1.97.1 (8bab26f4f 2026-07-14)`, host `aarch64-apple-darwin`, LLVM 22.1.6. The linked serde_json rlib SHA256 is `d1233394af34e44bbb105c560c78302ffa07d22cc5f35fd30b6413efde000eed`; the sha2 rlib SHA256 is `f330242bbe618610a480be8d6467841cc2c6acc06ff71d93c7d278cf702c0f02`. Source worktree: `/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore`.

Run the retained commands with a new output filename, because the reviewers use exclusive creation:

```sh
reviewer/ordinary-capture-review remote NEW_ORDINARY_REVIEW.json
reviewer/failure-review . NEW_FAILURE_REVIEW.json
```

Both commands returned zero. `ordinary-capture-review.json` independently reconstructs all 653 Ethernet/IPv4/TCP packet records and fixed-endpoint SYN ownership, verifies original IP lengths rather than snaplen, final counter equality, body coverage, PCAP/lifecycle hashes, exact case copies, server exit/reap, offload states and owned topology cleanup. Expected normal/interrupted/ongoing classifications are preserved. The instrumented capture did not run. The initially prepared `reviewer/capture-review.rs` and executable are an **unrun dual-capture reviewer**, retained only as preparation history; they produced no result and do not qualify missing instrumented evidence.

`independent-failure-review.json` binds the exact source, API identity, evidence archive, coverage status, complete retained file inventory, reviewer sources/executables and ordinary capture review. It verifies that ordinary build and capture succeeded, coverage failed with exit 124, and sender/WAN steps were skipped. The ordinary release checksums passed both before and after the isolated coverage attempt. Instrumented compilation completed in 33.45 seconds; 14 tests visibly completed (4 library, 1 CLI, 9 benchmark), while the remaining ignored 88-operation broad benchmark test was still running when the 300-second command timeout expired. No assertion failure was printed. This demonstrates a bounded timeout, not proof that the unfinished test is correct or that a longer execution would pass.

All historical artifacts are unchanged. The full stderr prefix is not retained by the capture controller, so the independent capture check cannot reconstruct that full-stream stderr digest. Ordinary executable identity is bound to its producer checksum receipt; executable bytes were not uploaded after the coverage failure. This synthetic capture evidence is not a WAN, TLS, throughput or energy result. A successor producer requires a new evidence directory, actual successful runtime qualification and artifact custody review.
