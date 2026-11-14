# Authenticated rc.3 guest runtime candidate

The Linux preflight verified two Sigstore signatures: one over the guest archive and one over its checksum file. The guest producer was upstream tag `v0.28.0-rc.3` at `ffd90f895d7080629eceb16095d1ca81c0a49b95`. The verifier ran from contributor checkout `eb450b97f92668847317239335b1dd340d1c4b28`, the intended host source. The upstream candidate-signing job succeeded, but the overall release workflow failed and skipped release publication. This packet carries the **signed prepublication candidate**. [Signing job](https://github.com/tinylabscom/mvm/actions/runs/43486823143/job/132133312570) · [Release workflow](https://github.com/tinylabscom/mvm/actions/runs/43486823143)

The download matched Actions artifact `14072662215`, attempt 1: 21,557,023 bytes, ZIP SHA-256 `333b05e4e71c2c10a5d4c00cf749244333ce1559e2f45402589c898412db34e7`. The ZIP contained the four files below plus the 890-byte `runtime-upload-context.json`. The download receipt recorded transport identity. A subsequent verification checked the upload context and signatures. [Original artifact](https://github.com/tinylabscom/mvm/actions/runs/43486823143/artifacts/14072662215)

| Member | Bytes | SHA-256 |
| --- | ---: | --- |
| `mvm-guest-bins-v0.28.0-rc.3.tar.gz` | 21,549,008 | `baedf180bad50a5ee34b438510a8431d02e7fce66ff36c896c2ceb4d2841b784` |
| `mvm-guest-bins-v0.28.0-rc.3.tar.gz.sha256` | 101 | `413fcab2575a9c5ad3b4cff9b06c3e0a13d29a24d2b7ccbca0e065552d821627` |
| `mvm-guest-bins-v0.28.0-rc.3.tar.gz.bundle` | 9,339 | `16fc7a9fb20a964cd85b235c30d89026be464e3ed4fe1044320f821967592f41` |
| `mvm-guest-bins-v0.28.0-rc.3.tar.gz.sha256.bundle` | 9,535 | `cb8a18eddb7d25d64e7abb6b3e44d6d97fdd0972ff1338adeee104e66585b186` |

The existing MVM `verify_directory(..., "signed")` and `verify_quartet` functions consumed the original run, tag, repository and commit identities directly. The verifier required the upstream `release.yml` certificate identity at `refs/tags/v0.28.0-rc.3`, GitHub Actions’ OIDC issuer, the tag-push event and the exact source commit. It needed no emulated Actions environment. The Linux amd64 Cosign v3.1.4 binary matched its official release asset SHA-256, `424e2c97d0b45fd7a6b608a87f090f35633f4c23ece5f092d48bba11aaf4bbfa`. Cosign reported `Verified OK` twice and the verifier exited 0. Verification JSON SHA-256: `afe3247f7842486bc947b379855066ba4dccd8af55abe169f8f359282a52e79f`. [Upstream verifier](https://github.com/tinylabscom/mvm/blob/ffd90f895d7080629eceb16095d1ca81c0a49b95/scripts/runtime_release_proof.py) · [Cosign v3.1.4 release](https://github.com/sigstore/cosign/releases/tag/v3.1.4)

The tag’s guest source includes the public mailbox reader boot binding and the root-only ACK key. Its `flowmux_drive.rs` (SHA-256 `c7a780ba5fd7a67ac948ad4e97ffad52740445922502f164d341c77c0a0f371f`) requests `0444` at file creation. The contributor’s later file-descriptor permission correction is absent from these signed guest bytes. The effective umask and binding permissions inside this candidate guest still require a native observation. [Tagged guest source](https://github.com/tinylabscom/mvm/blob/ffd90f895d7080629eceb16095d1ca81c0a49b95/crates/mvm-agentd/src/flowmux_drive.rs#L403) · [Reader permission controls](reader-boot-permissions.md)

**SDK admission and guest boot were not run in this preflight.** The next check is the rc.3 SDK’s portable-asset acquisition with the signed quartet, kernel and current revocations before export or boot. The previously measured rc.2 guest remains a separate result.

## Subsequent SDK and installed qualification

Contributor SDK `468661f4` acquired and verified this candidate with signed revocation publication 2 and verified kernel SHA-256 `1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6` (19,995,452 bytes). Image-set manifest SHA-256: `5033520a59514f5d206d76d13db8c9b7e5841f068c83dd8585ef141879a1bb24`. The probe exited 0 in 6.42 s wall / 3.70 s user / 0.17 s system CPU, maximum RSS 61,076 KiB. Both original temporary stages were absent after drop. Acquisition-log SHA-256: `1c43db345417f1eebdbe59e60bf2ff618633c1c396ee85ab730d532d26817b04`.

The same authenticated candidate subsequently completed signed workload export and the installed one-guest transaction. [Current native measurements and limitations](native-rc3-fixture.md). The signature preflight above and this pre-boot acquisition are separate measurements; effective guest file permissions were not independently inspected.
