# Independent successor review: checked scanner quote API visibility

2026-11-05 UTC. Scoped independent Code/Rust/Security review by `pii_512_review`.

**Verdict: ACCEPT the thin API delta for Session integration, pending final coherent consumer/graph checks. No blocking defect found in the visibility/reexport/docs change.** This is not a successful overall Cargo-test or all-target-graph verdict. The inherited signer-bin failure remains explicitly OPEN in this isolated source context.

Read-only review of README, handoff visibility.patch/pins/execution-context, quote-receipts.tar.gz and archive-inventory.json. No builds, execution, source edits, new agents or broad audit. Logical roster `5e31d33c344d39b0` and Code/Rust/Security roles independently rechecked active. Prior split implementation review remains unchanged; this successor review covers only the public seam and its focused evidence.

## Source judgment

Comparing included whole before/after leaves shows exactly two intended product files changed:

1. `telemetry_collector/owned.rs`: existing `owned_scanner_requested_bytes(&RedactionPolicy) -> anyhow::Result<u64>` changes pub(crate) to pub and gains documentation explicitly withholding quota/admission and retaining actual same-pool reservation before collector construction.
2. `telemetry_collector/mod.rs`: the SAME existing function moves from the crate-private reexport group to the public reexport group. `OwnedCollectorStart` and `start_owned` stay crate-private.

The function body still obtains actual StreamRedaction/Core facts and delegates to the original private checked helper. There is no formula copy, alternate facts input, fallback, mutable-control constructor, new quota/grant, policy/validator change, unsafe code, mutation path, payload logging or authority change. Existing errors propagate through Result. The private helper remains private. No Core source or declaration coefficients change.

This implements the previously approved narrow architecture seam: Root may read the same source-coupled scanner declaration and compose checked disjoint budget terms without copying the model. Numeric data is not an admission receipt, native heap/RSS proof or permission to construct. Caller must propagate errors and retain actual same-pool admission and all remaining positive terms. No MVM wire/envelope/version/capability change is implied.

Whole before/after owned.rs and mod.rs bodies are reference material, not synchronization inputs. Integrate only the exact visibility/reexport/docs hunks into Session's coherent current producer. The reviewed source hashes belong to the isolated packet; they cannot be used as hashes of a later rebased Session source.

## Independently checked evidence

- Archive hash and length verified: SHA `01e110c6edae565b5ea28388c193c6d80e37e0d2e5d7871089786bfbf5b01ad7`,1,150,924 bytes. Archive contains exactly32 regular files matching the inventory's complete name set; all32 byte lengths and SHA-256 values match.
- Actual API RED: build exit101; compiler JSONL has exactly one error E0603, `owned_scanner_requested_bytes` is private, at external test import. No executable artifact is emitted in that RED. All5,238 supplied input hashes match before/after.
- Successor: build JSONL emits the external integration test `owned_scanner_quote` from the actual copied source package, `fresh:false`, executable `/var/tmp/spiralcore-pii-capacity-20261104/target/release/deps/owned_scanner_quote-9a2eb55ad569fdab`. This is actual compilation/artifact evidence despite the overall command's later failure.
- Executing the protected caller is recorded as exit0 with exactly2/2 PASS. Test source imports the public reexport, obtains scanner24,363,008 and actual separate collector metadata summing with checked_add to51,757,056, and rejects an unsupported category. It invokes the actual checked query; no shadow formula or synthetic grant exists.
- Protected executable hash recorded before execution is `1ddb24b05fb881ef796197c9147699aac7017c4047fcf189c51d46ac24e37e10`, matching result.json. The packet intentionally excludes executables; this review checks the compact custody/result records and does not claim independent live binary rereading or rerunning. Postbuild inline context is explicitly a transcription, not an independent /proc capture.
- Successor source-before/source-after maps are identical across5,238 supplied source/config inputs. Comparing RED and successor maps gives EXACTLY the two visibility/docs source changes listed above; external test and Core are unchanged. Actual Core pii.rs remains `8b8e5b1289abc1231e677b0a6dab7952d32d9263411480d9e84698360ec01547`, the independently reviewed split source. Hostd helper/model body changes are absent.
- Scoped `cargo clippy ... -p mvm-hostd --lib ... -- -D clippy::all` exits0. Its compiler JSONL contains no compiler-message diagnostics and stderr shows no warnings. This qualifies the library scope only.
- Structured RED/successor command receipts share the actual source cwd, absolute config, release/offline/locked arguments, jobs1/incremental0 and LTO=false. The retained runner derives command.json from the actual argument/environment values supplied to subprocess. The separate strict command and execution-context transcription clearly delimit library scope and settings.

## Failure and consumer limits retained

The successor overall `cargo test ... --test owned_scanner_quote --no-run` exit is101, not0. Its compiler JSONL contains E0061 at `crates/mvm-hostd/src/bin/mvm-signer-helper.rs:95`: a four-argument function receives three arguments. That sibling bin is not one of the changed product files and its input hash is unchanged across RED/successor. The successful external executable and scoped library Clippy do not erase or qualify that bin failure.

The parent task reports Session's coherent candidate already uses the proper original RegisteredListener plus RegistrationAdmission call path, with signer source SHA `9fa321a196b4350646c128d8fad80b872220e35e5df8d4a46e1ccd2e98506217`. That Session source was not included/reviewed here; do not present it as independently verified by this report. Do not repair the isolated older signer with a guessed argument or legacy listener workaround merely to manufacture a green packet. Validate Session's actual final graph and exact source separately.

The external test success supports API availability and checked-query behavior in this bounded intermediate context. It does not constitute complete all-consumer rebuild, current SDK acceptance, installed native admission/capacity, transfer/WAN, recovery, scale or FIPS/CUI qualification. Those gates remain OPEN. A pure visibility/docs seam did not require another325 allocator run because the source maps and diff show the measured Core/model implementation is unchanged; existing split evidence keeps its original source/configuration limits.

## Exact identity

All values below are SHA-256.

| Artifact | Hash |
|---|---|
| quote-receipts.tar.gz | `01e110c6edae565b5ea28388c193c6d80e37e0d2e5d7871089786bfbf5b01ad7` |
| handoff/visibility.patch | `c94473e7ba484d970681579aad64c2b1470d2a4c71c13b8f857cb20ee7b601cc` |
| handoff/pins.json | `6744817086922e3431f61e68a8e80cb98e5a86c4d15618650b3a3a104b19c73a` |
| handoff/execution-context.json | `945c10484a044d6c172736f07ccd124b0b955b5e0bcb3f9097842ebe765cdc19` |
| owned.rs before | `60242a6563491a16a295d92fa4c7a594c3b0445fb4cda3bfc5331f9bada2d9e1` |
| owned.rs after | `b932a0f4b9471c62df926731d66bf9684a7350974c785f15635c629bd7c7b685` |
| mod.rs before | `203e3bb15fcabfe4e511206db86633d73acd27b91ea0a2c5250f967a6440da14` |
| mod.rs after | `0f86c770b2fccb224bf1c7b44140cf0b5ead831ada10b04a284b7aa2ab0161b0` |
| external test source | `9eac7c6202e05fa198c42d40da08d079e4064bb6f983a5d32139ea4978076260` |
| protected external test executable (recorded) | `1ddb24b05fb881ef796197c9147699aac7017c4047fcf189c51d46ac24e37e10` |
| successor build.jsonl | `95b81a73bb64e38de9e4e7b9ff9c0a8a2ae8def7ed88aa450a8adafd0e2fb33d` |
| successor test.stdout | `ed0720f94d3e62eac8721841b9efb140340826805c7fc78de666896c33561504` |
| successor result.json | `5b3a75bd2ae5e125293f21d70f30cdbd5b35e560365f61fe107323fda9ce58df` |
| scoped strict.jsonl | `5987d3404ae11edacf20f9292de5eb611ed1157019e208c210f625b90d11d4b5` |

The thin API change is acceptable for narrow Session integration. Final source-pinned consumer/graph checks remain required before broader delivery claims.
