# Successful producer 40950252067: independent runtime and custody review

Pinned source: `9dce4c57d1f41cef7365b297cd690849afce34e7`, repository `R4-Laboratories/SpiralCore`, workflow `.github/workflows/spiralcore-wan.yml`, first attempt. This successor is separate from the failed `linux-producer` directory, whose artifacts were not changed. No campaign gate files were created by this reviewer.

The successful GitHub run and single-job steps were fetched through the authenticated GitHub CLI into `run-api.json` and `jobs-api.json`. The artifact listing and exact sender/evidence API objects are retained. The evidence artifact `spiralcore-wan-40950252067-1` was downloaded exclusively to `remote/`. The actual whole-job log is retained locally as `producer-diagnostic.txt`, with its SHA256 bound in the final receipt. It contains no measurement credential: this qualification-only run had no WAN URL, run ID or token.

The native capture reviewer independently replayed the Ethernet/IPv4/TCP records from both ordinary and instrumented synthetic qualifications, including SYN ownership for the one-row fixture and the separate ongoing connection. Each qualification had 137 normal, 134 interrupted and 382 ongoing packets, totaling 1,306 independently checked records. Normal was complete; interrupted and ongoing remained incomplete as required. It checked IP lengths rather than capture snapshot length, body coverage, final captured/received equality and zero drops, PCAP/lifecycle hashes, all expected outcomes, server exit/reap, offloads and owned topology cleanup. Full stderr prefixes are not retained by the controller, so full-stream stderr digests cannot be independently reconstructed.

The runtime/custody reviewer binds success to the exact API source, repository, first attempt and successful source/capture/coverage/sealing/upload steps. It checks the actual coverage log uses optimized release artifacts and that the 88-operation ignored broad qualification finished. The instrumented suite contains exactly **167 passed, zero failed, zero ignored across 20 test binaries**; the ordinary suite contains **165 passed, zero failed and two ignored across 20**. Separate provenance-helper test counts are excluded. It also checks all 33 transfer-source hashes from the native qualification manifest against the frozen checkout.

Coverage reports 7,673/8,476 lines (90.52619159981124%), 600/648 functions (92.5925925925926%) and 13,856/15,873 regions (87.29288729288729%). Branch and MC/DC coverage were not collected. These are optimized instrumented source-coverage observations, not performance or energy measurements and not a minimum coverage acceptance gate. Ordinary release checksum verification succeeded both before and after coverage. The instrumented controller's receipt hash is distinct from the ordinary controller; instrumented executable bytes are not part of the uploaded sender artifact.

The sender artifact is **12931876271**, exactly **6,982,485 bytes**, authenticated API digest **`sha256:bc0d2c339fd4060321ec584f0f1cbc28973c285a863346c781c121ea9f016e51`**. `gh api repos/R4-Laboratories/SpiralCore/actions/artifacts/12931876271/zip` downloaded `sender.zip` under a 100 MiB file limit. The native checked-out validator at `/Users/casey.allard/spiralcore/target/debug/spiralcore-build-manifest` verified the API proofs, actual ZIP digest/length and bounded flat archive before the four fixed entries were extracted. Each output used a 32 MiB file limit and restrictive umask. Extracted files remain mode 0600. No Linux binary was executed on macOS.

Executed validator commands, from this evidence directory:

```sh
/Users/casey.allard/spiralcore/target/debug/spiralcore-build-manifest proofs expected-reuse.json producer-run.json producer-artifact.json producer-proof-validation.json
/Users/casey.allard/spiralcore/target/debug/spiralcore-build-manifest archive expected-reuse.json producer-run.json producer-artifact.json sender.zip sender-archive-validation.json
/Users/casey.allard/spiralcore/target/debug/spiralcore-build-manifest verify expected-reuse.json producer-run.json producer-artifact.json sender build-reuse-validation.json
```

All three validation receipts report success. The independent final reviewer also checks the three Linux ELF headers, actual binary hashes/lengths, nonexecutable local permissions, byte-identical remote/downloaded manifest copies, ordinary capture qualification hash, producer checksum receipt and authenticated artifact digest. The retained manifest is `sender/manifest.json`, SHA256 `ddf224f86ec70c52ea01b0988847e15276464008c8627926c52489445f955f36`.

Both independent reviewers are native Rust. From `/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore`, each was compiled with:

```sh
/Users/casey.allard/.cargo/bin/rustc --edition=2021 -C opt-level=2 REVIEW_SOURCE \
  -L dependency=crates/spiralcore-transfer/target/debug/deps \
  --extern serde_json=crates/spiralcore-transfer/target/debug/deps/libserde_json-f11651c4b5834b4e.rlib \
  --extern sha2=crates/spiralcore-transfer/target/debug/deps/libsha2-11f7c61dcc994809.rlib \
  -o REVIEW_EXECUTABLE
```

Compiler: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, host `aarch64-apple-darwin`, LLVM 22.1.6. The serde_json rlib SHA256 is `d1233394af34e44bbb105c560c78302ffa07d22cc5f35fd30b6413efde000eed`; sha2 is `f330242bbe618610a480be8d6467841cc2c6acc06ff71d93c7d278cf702c0f02`. The supplied Mac provenance-validator executable is SHA256 `9aba4e17d76c699dfef0ded8eb0bb8d32fb9dbd9474e44ee22ed869f962883fe`; its checked-out implementation hashes are bound in the final receipt.

Reviewer commands, all observed exit zero:

```sh
reviewer/capture-review remote dual-capture-review.json
reviewer/custody-review prepare . /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
# Native validator proofs, archive and extracted-file verification above.
reviewer/custody-review final . /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
```

The reviewers use exclusive output creation. To reproduce without modifying retained results, copy the required inputs into a fresh directory, retaining the same structure, then run the commands there. Relative paths in the final JSON are relative to this evidence directory. `independent-runtime-custody-review.json` SHA256 is `e5231d8eaead8d3804f7b1aebe85a7ed0945f9a80b77655e2ba614448bd92133`; `dual-capture-review.json` is `4a0a76ceefd3a3810dfb1d44bd02253301ae5771aaa0b85da0f5f15ccf9f74f7`. Source and reviewer executable hashes are included in those receipts.

The source review found no blocking custody boundary issue. Its requested exact-test-count improvement was applied before running the final reviewer. No failed reviewer run or acceptance bypass occurred. The actual sender archive remains available locally even after GitHub's one-day retention expires. Future consumers still require their own same-run capture qualification. This packet qualifies a producer and a local synthetic capture lifecycle; it does not report a WAN transfer or reinterpret historical incomplete captures.
