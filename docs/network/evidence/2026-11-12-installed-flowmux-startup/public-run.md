# Public installed FlowMux runner: native Linux result

On 2026-11-13, the public installer, cooperating peer and installed controller completed one Linux x86-64/Firecracker invocation. The guest received the expected refusal, then sent and received **5,804 exact bytes**, including binary and trailing-zero bytes, through its admitted FlowMux endpoint. The original SDK collected the guest and workers. The SDK, peer and mount namespace exited zero. [Machine-readable measurements and identities](public-run.json) · [Public reproduction instructions](../../../../tools/installed-flowmux-witness/README.md)

The measured SpiralCore source is [`7583bd8c`](https://github.com/R4-Laboratories/SpiralCore/commit/7583bd8c97b7471a8f831b331906aac2ed2d2222). The normal locked Cargo manifests resolve the host SDK, installer, peer and three helpers to public MVM [`7b224540`](https://github.com/Casey-allard/mvm/commit/7b22454077530e96146d68204b711de53e3d05a8). The sealed guest contains the separately published `v0.28.0-rc.2` runtime, whose producer records source `849382ad417a28f3d68b051411b5ae3bab591b13`, and image-set `0.2.4` kernel inputs. It was not rebuilt from the host integration revision. This records interoperability of these exact components; it does not establish a single-revision guest/host build or qualification of newer upstream changes.

## Traffic, compute and elapsed time

| Operation | Application sent / received | CONNECT sent / received | Guest elapsed | Guest process CPU |
| --- | ---: | ---: | ---: | ---: |
| Refused destination | 0 / 0 B | 79 / 64 B | 5.851159 ms | 1.205944 ms |
| Permitted exact echo | 5,804 / 5,804 B | 69 / 39 B | 185.678166 ms | 7.262799 ms |

The peer independently checked the entire body and EOF before echoing it. Its socket counted **3,661 incoming and 3,234 outgoing TLS bytes**. Accept through half-close took **175.218304 ms**. These are cooperating-endpoint TLS counters on the same host; they exclude TCP/IP headers and are not physical-NIC or WAN measurements. This run has no raw or zstd control, and the echo includes receiver-to-sender traffic rather than two independent compression samples.

| Process accounting from GNU time | Wall | User CPU | System CPU | Maximum RSS |
| --- | ---: | ---: | ---: | ---: |
| Installer | 7.43 s | 6.09 s | 0.53 s | 11,172 KiB |
| Controller | 12.53 s | 9.26 s | 2.10 s | 267,936 KiB |
| Peer, including waiting for the guest | 12.33 s | 0.02 s | 0.03 s | 8,496 KiB |

GNU time reports its observed command accounting, including waited descendants where applicable; these CPU rows do not isolate codec work. The controller separately recorded preparation at 1.945564770 s, launch at 11.936584265 s, dispatch at 213.899903 ms and completion through retirement at 12.329045527 s. Those phase boundaries overlap and must not be summed. Peer waiting and controller execution also overlap.

## Build settings and retained failures

The standalone consumers now carry MVM's existing crypto development settings: `mvm-core` at optimization level 2 and `sha2`, `ring`, `zeroize`, `curve25519-dalek` and `ed25519-dalek` at level 3. Cargo does not inherit those profiles from a dependency workspace. No crypto algorithm, provider, wire meaning or signed deadline changed. The scripted ACK test also reuses its prepared peer session.

The earlier installer-only preflight used the same signed archive, a fresh 512 MiB filesystem and the same 512 MiB address-space cap. It recorded 21.88 s wall and 21.11 s summed user/system CPU; this run recorded 7.43 s and 6.62 s. These are **one observation per configuration**, with different page-fault counts (85 versus 4 major faults) and uncontrolled filesystem/cache warmth. They establish the observed reduction, not an isolated causal speedup, latency distribution or energy saving. The preflight transferred no guest application stream. [Earlier native trial 9](verified-echo.md) remains a separate source-scoped result.

Ordinary locked Linux Cargo checks passed for the public package: six tests, all-target strict Clippy and both executable builds. The controller passed strict all-target Clippy and all **160 library tests under default parallel execution**, with no ignored or filtered cases. Its two output-validation tests passed individually, and this native invocation ran the exact named installed test once. Eleven shell preflight/structural controls passed separately. The complete MVM workspace is not claimed green by these scoped consumer checks.

Earlier default-parallel attempts passed 150/160 and then 158/160. Independent tests could fork while another test held an original lock descriptor. The final change uses MVM's existing re-execution helper for 51 tests that require independent holder lifetimes; their assertions and internal concurrency remain. A failed-write socket fixture explicitly shuts down its peer. Production locks and refusals, the signed 900 ms expiry and the original test count remain unchanged. Failed attempts and their hashes are retained; a separate serial pass is not substituted for the final parallel pass.

The premerge pipeline audit passed on the measured source after independently matching the build, executable, native and signed-audit receipts. Separate code/Rust and security reviews found no blocker in their declared scopes; the author of the fixture-isolation changes did not independently review those changes. Compact receipt hashes are recorded in the JSON. These reviews do not complete the broader MVM acceptance program.

A final formatting-only successor corrected three feature-gate layouts and test-harness module ordering. All 52 changed Rust files passed the scoped formatting check; strict Clippy, the same 160 default-parallel library tests and the controller build passed again. The rebuilt controller is **byte-for-byte identical** to the executable used in the native trial, SHA-256 `1368832f0adb4ad5d86193b5463374450cf80fc50294ca9e3872875074f782ff`. The JSON records both source revisions and the new build receipts. No additional guest run is claimed. A recursive formatting check also reported unchanged baseline files; those unrelated files were preserved.

## Ownership, audit and remaining acceptance

The finite ext4 image was 536,870,912 bytes. Installation used 97,509,376 filesystem bytes; final usage was 138,104,832 bytes. The runner imposed address-space caps of 512 MiB for the installer, 192 MiB for the peer and 768 MiB for the controller. The guest used one vCPU and 256 MiB with no NIC. A one-shot, operator-approved test process inherited KVM access; login group membership was unchanged.

The original owner reported zero charged guests, guest RAM and workers, and verified all four retained reservation identities. Retained balances were **806,371,328 host metadata bytes, 9,494,528 cache bytes and 536,875,008 staging bytes**. `all_resources_released` was deliberately false: verified retained storage remains charged. No matching original helper or Firecracker process, original mount or loop attachment remained. This is bounded shutdown evidence, not full storage reclamation or crash-recovery acceptance.

A separate macOS readback checked the exact 2,336-byte audit copy with MVM's canonical signature/chain verifier from the previously built verification tool. It verified two entries: one allowed and one refused destination under the original signer, tenant, workload and session. The original owner separately checked descriptor/reservation custody. **The host audit fields still contain a zero plan UUID and an unbound image name.** The signed session/workload decisions are not a cryptographic assertion of plan or image binding; that remains an integration gap.

The operator retains the original private run records and signed drains under trial ID `native.EvrfGPu9`. After independent verification, the owned backing image was removed with its original mount, loop and open-use checks clear; observed free space increased by 319,086,592 bytes. That cleanup does not change the trial's measured retained ledger disposition. Only compact measurements, verified scope outcomes and hashes enter Git; the original host address, session, audit public key and plan ID remain in the private hashed receipts. Keys and issued operator packets are private. This result does not complete two-guest installed FlowMux operation, sustained capacity, Windows/macOS installed endpoints, owner access, canonical cross-host WAN, FIPS/CUI qualification or the full MVM delivery tracked in [issue #42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
