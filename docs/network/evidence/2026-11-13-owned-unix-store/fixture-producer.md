# Installed FlowMux fixture producer

The project owner approved the bounded Rust example in [issue 261](https://github.com/R4-Laboratories/SpiralCore/issues/261). It prepares the signed one-CPU, 256-MiB workload consumed by the existing installed controller. It reuses MVM's authenticated boot assets, image materializer, signer and archive verifier. It adds no transport or admission authority. [Reproduction instructions](../../../../tools/installed-flowmux-witness/README.md#prepare-the-authenticated-inputs).

## Test-first baseline

The [hosted Linux baseline](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43496597009/job/132131195782) tested head `2c9c2633e6cf4244947cd2eba819257aa3f9978d` through PR merge `782d04a3666ae6a4b94bf965bffc36acf04033ce`. The original exact-record golden passed. The actual producer test executable compiled and then failed all five intended assertions: noncanonical IPv4, occupied destination, unsafe parent, wrong staged ELF digest and mismatched baked argv. Every refusal still returned the scaffold's generic error, which did not satisfy the required specific reason. Exit 101 therefore records the expected missing behavior, rather than a compiler or infrastructure failure.

The executable SHA-256 was `051336369d014e99be91e712c8fc3782f4fc5121e95bc45011be80be4bc0a8f6`, unchanged before and after execution. The 3,725-byte artifact ZIP has SHA-256 `4daf3482e5bf0cb501a8df0b966cc16380f38cc469f93052545c0dd752b8eee8`. All eight member hashes were checked. [Source, executable and artifact identities](fixture-producer-baseline.json).

## Implementation and remaining acceptance

The successor implements staging, canonical role construction, authenticated boot acquisition, signing, independent archive reopening and no-replace packet publication. It requires an existing producer identity and current applied revocations. Its operator output contains the package and public verification inputs; private signing material stays in the separate producer home.

The [successor hosted job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43500008767/job/132142992197) compiled the real example, passed the same five refusal tests in 0.25 seconds, and passed strict example Clippy with `-D warnings`. Its head is `a5b24c8a47470a3deeebfa07d62cb1bcb1ad4811`; the tested merge `e5b01c4b6cd2e453502d983d82995da66e2ef966` has the identical tree `f07027c087985c4b0bbefc91ecf3b8f56af1d052`. The 42,889,512-byte test executable has SHA-256 `6a902c7770fb1de6e65a58bc396a5885100d852b63ce01ed5a1c77756c431537`, unchanged before and after exit 0. The artifact ZIP digest was checked against GitHub's digest; all nine member hashes are recorded. [Exact stdout, compiler, source and artifact identities](fixture-producer-candidate.json).

| Check | Original scaffold | Implemented producer |
| --- | --- | --- |
| Noncanonical IPv4 | Wrong refusal reason | Pass |
| Occupied output and original-file preservation | Wrong refusal reason | Pass |
| Unsafe output parent | Wrong refusal reason | Pass |
| Wrong staged ELF digest and absent output | Wrong refusal reason | Pass |
| Baked argv disagreement | Wrong refusal reason | Pass |
| Legacy publication record golden | Pass | Pass |
| Strict producer Clippy | Not run | Pass |

The workflow builds with `cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml --features canonical-host --example installed_flowmux_fixture --no-run --message-format=json`, selects the unique actual test executable, and runs it with `--nocapture --color never`. It requires all five tests and an unchanged executable. Clippy uses the same feature, lockfile and pinned compiler. The record golden independently checks the unchanged 49- and 81-byte headers, body bytes, lengths and metadata refusal paths.

The separate [Linux host and contract job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43500008767/job/132142992576) also completed successfully on the same head. It ran the live crate's Linux-host targets, shared transfer and adapter conformance suites, strict Linux-host Clippy, the default guest-entry check and the two synthetic measurement-analysis controls. This is SpiralCore workflow coverage, not a full test run of the pinned MVM dependency workspace. Existing ignored tests and actual guest boots remain outside that result.

A genuine signed export and the original installed one-guest transfer, signed audit and retirement remain separate unexecuted gates. The tests do not establish those results. The 0.25-second unit-test runtime is not a producer, transfer or CPU benchmark. There are no transfer-speed or byte-saving claims from this producer check. Wick is operator-paused; hosted checks do not use that machine.
