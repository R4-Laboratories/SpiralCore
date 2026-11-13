# Original-owner MVM store integration

The successor [Linux registered-endpoint result](linux-installed-retirement.md) completes exact readback and original-owner retirement on published MVM `31f8a562`. It repairs the confined namespace-publication syscall and separates orderly admission closure from hard failure. The measurements below retain their original `c07d912e` source and scope; they are not relabeled as successor runs.

The MVM host adapter now keeps the originally admitted parent and child directory descriptors throughout SmallV1 receive, reader shutdown and retirement. Its two original macOS relay-custody regressions pass. This is native host-process component evidence; it does not qualify an installed guest, two-host WAN transfer or a production capacity envelope.

The [MVM contributor checkpoint](https://github.com/Casey-allard/mvm/commit/c07d912e79cb1623bd26516fb6a48eddb640e01b) is published. The adapter consumes SpiralCore revision `4ea98b9e27116a163c9767f5a351bf177eff3b6b`. The implementation preserves the wire format and object identity. The original MVM partition guard must approve store funding before the selected endpoint creates effects. An implementation that has not supplied that guard refuses activation.

## Recorded execution

| Command or selector | Observed result | Scope |
| --- | --- | --- |
| `cargo test --locked --offline -p mvm-hostd --lib`, first full run | 2,471 passed, one failed, 15 ignored; 37.13 s test execution | Stale mailbox-socket retirement fixture failed in the parallel process. |
| Same full command after isolating that fixture | 2,472 passed, zero failed, 15 ignored; 36.43 s test execution | Native macOS hostd library tests. Existing ignored tests remain unqualified. |
| `cargo clippy --locked --offline --workspace -- -D warnings` | Passed | Contributor MVM workspace on macOS; Linux-only source is outside this result. |
| Publication hook: `cargo +stable clippy --workspace --all-targets -- -D warnings` | Passed | Published MVM source; mechanical test-lint repairs retain the recorded earlier failures. |
| Original partition resource-guard tests | Two passed | Default refusal and original freshness checks. |
| Installed VMM lifecycle tests | Eight passed | Original child/holder, shared wake aliases and first kernel-error controls. |
| Registered endpoint fixture funding | One passed | Funded original plus five refusal cases; synthetic fixture, not aggregate host qualification. |
| Joined original-parent suite against public MVM SDK | 25 passed, zero failed or ignored; 1.89 s test execution | All parent/configuration tests, including eight endpoint tests and portable-quote compatibility controls. |
| Complete joined canonical-host library | 170 passed, zero failed or ignored; 10.49 s test execution | Includes all 25 parent/configuration tests plus worker, collector and resource controls. |
| Joined socket-backed mailbox adapter | 11 passed, zero failed or ignored; 1.02 s test execution | Exact local socket exchange and refusal controls; no guest or WAN claim. |
| Joined SpiralCore SDK Clippy with `canonical-host --all-targets -- -D warnings` | Passed | Native macOS public dependency closure, after preserving the prior lint failures. |
| Existing supervisor/resource harness | 73 passed, zero failed or ignored | Real production files, retained reservations, legacy compatibility and cleanup controls. |
| Original close-between-creation-and-attach selector | Passed; 113.973 ms test-process wall time | Prepared-reader hook reached, original reader collected and store retired. |
| Original poison-between-creation-and-attach selector | Passed; 83.322 ms test-process wall time | Prepared-reader hook reached; uncertainty retained the original reader and debit. |
| Replacement of an installed store after endpoint drop | Passed; 35.234 ms test-process wall time | Original owner remained locked; replacement directory sentinel survived. |

The three separately timed selectors used executable SHA-256 `4e3117032bafc5bad1097cfeb184e7f4918413b85f63045d22730b2b505a6bf4`. Those times are test-process measurements, not transfer latency or CPU measurements. The first full-run log SHA-256 is `d20b2a21cceef2554634f604f51ad180b49f2d68cd54be9ad0b196cef492533a`. The [compact receipt](mvm-integration-results.json) records the successor log hashes and discovered test summaries.

## Failure retained and repair

The stale-socket test assumed that dropping its local listener closed the final listening descriptor. A separate native macOS kernel control showed that a forked child can retain the descriptor before exec despite `CLOEXEC`: the socket still accepted a connection after the parent closed its copy, then returned `ECONNREFUSED` after the child exited. The original full-suite error does not distinguish a successful connect from other uncertain kernel outcomes, so that precise interleaving is unproven. The test now creates its listener inside the existing isolated test process. Its live-listener refusal, one-second deadlines and final absence assertions remain intact. Production socket retirement was unchanged.

An earlier carrier-funded fixture also lacked the newly required store workspace. It now funds the actual SmallV1 operation charge plus workspace; the unfunded refusal, one-byte TLS witness and refund assertions remain intact. The passing full hostd result includes that control.

## Original resource owner

One serial SmallV1 receiver requires one shared parent descriptor, two persistent store descriptors and up to four transient descriptors: seven total. Its iterator and traversal workspace quote is 1 MiB in addition to the existing codec operation charge. These are capacity requirements, not transferable authorization.

The shared installation ledger now admits descriptor charges at both host and tenant scope. A retained reservation clone keeps its exact original slot and debit. Insufficient descriptors or codec bytes refuse before store effects. Legacy noncanonical provisioning retains its existing identity and grants zero new store descriptors; a fresh canonical profile binds the new capacity. The parent adapter checks the retained reservation's serial and actual charge, then rechecks admission.

The contributor fork's joined footprint measurement counted all 2,956 tracked Rust files: 1,039,258 SLOC and 1,310,100 physical lines, including the added original-guard fixture controls. Its explicit ceilings changed from 1,038,600 / 1,309,500 to 1,039,300 / 1,310,150. All categories, regression tests and the downward ratchet remain included. This is a visible contributor-fork adjustment, not upstream acceptance of a larger source budget.

## Acceptance still open

The contributor fork also refused manual CI dispatch with HTTP 422 despite reporting Actions enabled and its workflow active; this is not a hosted CI result. The local Linux gated check was attempted and refused because `cargo-zigbuild` is unavailable; native Linux/hosted compilation is still required. All 25 original-parent host-process tests pass against the public dependency, and all-targets strict Clippy passes for that joined adapter. The SDK pin changes no registry package versions; its Rustix 1.1.6 matches the resource model. Linux x64, Linux arm64 and macOS arm64 CI now select the eight original-parent endpoint tests, with hosted execution still pending. The installed native Linux selected endpoint, guest lifecycle and two-host WAN must pass on their exact published dependency closure. macOS managed activation still refuses where enforced process-address-space qualification is unavailable. The separate Windows Server 2025 sustained-stream job on SpiralCore PR 259 reached 3,533 of 4,096 blocks in each direction before its existing deadline; it remains a failed gate. Neither its deadline nor acceptance count was relaxed. No new WAN speed, compute reduction, energy saving or FIPS claim follows from these component tests.

The first hosted checkpoint refused four jobs before tests because the active platform manifest still expected SDK `7b224540`. That expected source now matches published SDK `c07d912e`. The locked local metadata check confirms all 19 resolved MVM packages in the joined SDK and all three in the standalone adapter use that same source. Historical run revisions, source validation and test counts were preserved; hosted acceptance still requires the new exact-head results.
