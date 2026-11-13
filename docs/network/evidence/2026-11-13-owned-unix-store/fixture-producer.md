# Installed FlowMux fixture producer

The project owner approved the bounded Rust example in [issue 261](https://github.com/R4-Laboratories/SpiralCore/issues/261). It prepares the signed one-CPU, 256-MiB workload consumed by the existing installed controller. It reuses MVM's authenticated boot assets, image materializer, signer and archive verifier. It adds no transport or admission authority. [Reproduction instructions](../../../../tools/installed-flowmux-witness/README.md#prepare-the-authenticated-inputs).

## Test-first baseline

The [hosted Linux baseline](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43496597009/job/132131195782) tested head `2c9c2633e6cf4244947cd2eba819257aa3f9978d` through PR merge `782d04a3666ae6a4b94bf965bffc36acf04033ce`. The original exact-record golden passed. The actual producer test executable compiled and then failed all five intended assertions: noncanonical IPv4, occupied destination, unsafe parent, wrong staged ELF digest and mismatched baked argv. Every refusal still returned the scaffold's generic error, which did not satisfy the required specific reason. Exit 101 therefore records the expected missing behavior, rather than a compiler or infrastructure failure.

The executable SHA-256 was `051336369d014e99be91e712c8fc3782f4fc5121e95bc45011be80be4bc0a8f6`, unchanged before and after execution. The 3,725-byte artifact ZIP has SHA-256 `4daf3482e5bf0cb501a8df0b966cc16380f38cc469f93052545c0dd752b8eee8`. All eight member hashes were checked. [Source, executable and artifact identities](fixture-producer-baseline.json).

## Implementation and remaining acceptance

The successor implements staging, canonical role construction, authenticated boot acquisition, signing, independent archive reopening and no-replace packet publication. It requires an existing producer identity and current applied revocations. Its operator output contains the package and public verification inputs; private signing material stays in the separate producer home.

Compilation, the five refusal tests and strict example Clippy must qualify this successor. A genuine signed export and the original installed one-guest transfer, signed audit and retirement remain separate unexecuted gates. The tests do not establish those results. There are no transfer-speed or byte-saving claims from this producer check. Wick is operator-paused; hosted checks do not use that machine.
