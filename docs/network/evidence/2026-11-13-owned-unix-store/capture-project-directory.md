# Linux capture ran the explicit command from the caller’s directory

At MVM commit `909ba181c5fdd806c0837acd16c392baa73d924c` (tree `aa8b8ab04070f6311dd0d3395a2e8f3e59989dfe`), the Linux implementation of `capture project --run` traced the command from the caller’s working directory. The original CLI fixture selected the checked-in `rust-hello` project but invoked Cargo in the calling MVM workspace. That nested build replaced the host-services library in the shared target. [MVM issue 5479](https://github.com/tinylabscom/mvm/issues/5479)

## Original reproduction

The source check matched 4,654 files and their modes (64,596,786 bytes, zero mismatches). Compiling the full workspace tests with `--no-run` preserved the library and both Rust compiler probes passed. The executable for the existing `capture_project_emits_report_and_resolve_emits_ir` case had SHA-256 `5220e940914a0621cc5fb1414b8fda172650bc8e4233b112f1b0a3e9f0894876`. That single test passed in 92.61 s. Its enclosing command took 92.759 s wall.

| Observation | Library bytes | Library SHA-256 | SDK probe exit | Client probe exit |
| --- | ---: | --- | ---: | ---: |
| Before the test ran | 564,564 | `db32e376df23ba1a0a8ef87c1e07acb8ed17505796b7f6910c81c77414e6ae0c` | 0 | 0 |
| After the test ran | 492,006 | `6d483c3eaedce89b566a8e94262469261a90e28548d6af65c5d16b4d263bbd74` | 1 | 1 |

The library’s inode changed. Both probes then reported Rust error `E0463` while resolving the compiled dependency. The capture fixture’s own successful exit did not check whether it had replaced a library used by other workspace consumers. These observations support correcting the child process’s working directory while preserving the existing SDK and client exports.

## Focused regression checks

The first control invokes the public collector with an explicit local script that records its actual working directory in a file inside the selected project. The marker recorded the calling crate’s directory, `crates/mvm-capture`. A second control supplied a missing selected directory. Its marker still appeared, proving the original tracer executed the command. Both cases ran with Linux tracing available.

| Original control | Result | Log SHA-256 |
| --- | --- | --- |
| Selected project’s working directory | 0 passed, 1 failed, 5 filtered, 0.49 s | `c93c02aed820de42c047f4b26ff65fcb5e42c25906498373cf2aaecf6a30420d` |
| Missing project must prevent command execution | 0 passed, 1 failed, 6 filtered, 0.32 s | `20b5f1729b68f5c8e3e1484676362e0983b28fa6bd70f378d4423ef5d0e41835` |

The initial correction passed the selected project path to the tracer and set the child command’s working directory before spawn. Both focused cases then passed individually. The capture fixture suite passed 7/7 and the capture library passed 29/29. Review exposed two remaining cases on that source: a relative tracing-tool path and an execution report that claimed a missing-project command had run. Both controls exited 101. Their log SHA-256 values were `4417ed69e15844d09d7be0298df80aa0a3f20601d051a42fdb8ddb1e68883f08` and `244ec28c270b59f246d2ba1253c4a29081330e6046cb618612cc803da8fa0c2e`, respectively. The source changed during the intermediate Clippy invocation, so that invocation cannot qualify a specific revision.

## Final focused verification

The qualified five-file correction is published as [MVM contribution commit `4491a6652`](https://github.com/Casey-allard/mvm/commit/4491a6652). Its committed blobs match the source manifest below. The subsequent upstream join `eb450b97f92668847317239335b1dd340d1c4b28` includes public MVM main `48fa5b488c4543f0f7988df722633554663a9764`; combined workspace qualification is separate from these focused results.

On one frozen successor, three exact controls passed, all seven capture fixture tests passed, and all 30 capture library tests passed. The two original CLI cases each passed separately (0.46 s and 0.51 s). After the CLI cases, the host-services library still had the original `db32e376…ae0c` SHA-256 and the SDK and client probes both exited 0. Strict workspace Clippy with all targets passed in 17.10 s. The final check matched the same five source files used to produce those results.

| Final receipt | SHA-256 |
| --- | --- |
| Five-file source manifest | `38fc4c4c8f45c65603deb39805a1fabc84d277410c615d18fe26c8d1e9d5368f` |
| Test and Clippy log manifest | `0d1a79bf9972f610744812dc407e19973b2d139038e806693a1a25a1cfb78cee` |
| Library identity and both post-CLI compiler probes | `1fe7e95739ea30b09983aea4d8f7df4a26112781e7bf607643736f875279cff2` |

The complete workspace test run remains pending. These results qualify the Linux capture correction and its effect on the compiled dependency.
