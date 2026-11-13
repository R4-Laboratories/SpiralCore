# Reproducing the installed FlowMux fixture

**Proposed for project-owner approval, 2026-11-13.** The Linux witness consumes a signed synthetic workload and separate operator inputs. This proposal makes the workload reproducible after its private staging files have been retired. It uses the existing FlowMux, Object-v1 and MVM authority paths.

## Choice and invocation

The manual route seals a staged ELF with MVM's image builder and runs the SDK's signed-release export fixture against the resulting rootfs. It requires an operator to reproduce the argv, environment, resource declaration and archive pins across separate steps. We recommend a small Rust example in `crates/spiralcore-mvm-live/examples/installed_flowmux_fixture.rs` that derives those values once and verifies them at publication. It uses the crate's existing `canonical-host` dependencies. A non-Linux invocation returns an unsupported-platform error before filesystem effects.

```sh
cargo run --locked --features canonical-host --example installed_flowmux_fixture -- \
  "$GUEST_ELF" "$TRUSTED_ELF_SHA256" 192.168.103.59 "$NEW_OUTPUT_DIR"
```

Run from the live crate on x86-64 Linux with an explicit, operator-owned private producer `MVM_HOME`. Supply the guest ELF built with `SPIRALCORE_FLOWMUX_FIXTURE_IPV4` set to the same address and its independently recorded SHA-256. Require lowercase 64-hex SHA-256 and a literal IPv4 whose parsed value renders identically. Require a fresh output leaf under a private operator-owned parent. Reject aliases, existing destinations and unsafe permissions before signing. Stage privately, verify the actual staged ELF against the supplied digest, and publish a complete packet without overwriting an existing destination.

## Producer and operator boundaries

1. Capture the current signed revocation proof through `ImageSetRevocationStore::in_mvm_home().capture_applied_proof`. Acquire `PreparedPortableBootAssets::acquire(GuestArch::X86_64)`. MVM authenticates the pinned image-set kernel and CLI runtime quartet: archive, checksum, signature bundle and checksum signature bundle. The observed availability of the rc.2 release files establishes download availability only. A missing, stale, mismatched or unverifiable input stops publication.
2. Stage the program at `/usr/local/bin/spiralcore-flowmux-proxy-guest`. Set `ImageRuntimeConfig` argv to that path, `denied.flowmux.test:38443`, and `<canonical IPv4>:38443`. Use `InjectAndMaterializeRequest` with the acquired `guest_binaries()`, `InProcessOnly` and `sealed(true)`, then call `inject_and_materialize`. Reopen the guest sidecar and compare its argv with the descriptor. Use the original rootfs, verity tree and root hash.
3. Construct the validated environment for `spiralcore/spiralcore-flowmux-proxy-guest@0.1.0`, target `x86_64-linux`, one CPU and 256 MiB. Its sole capability is egress and its sole allowed destination is the literal IPv4 on port 38443. Volumes, secret slots and services are empty. The existing installer issues the host-vsock-proxy plan; the guest has no NIC.
4. Use `mvm_client::bundle::{BundleExportInputs, PostureInputs, HostBundleSigner}` with the original acquired kernel and initramfs, verity inputs, matching resources, environment and sealed egress posture. Require the operator's existing producer signer. MVM's `ed25519_keypair::load_existing` and `HostBundleSigner::from(HostSigner)` allow loading that identity without first-use key creation. Use the standard signer exclusively in memory. Stage, verify and publish through the original boot closure. Reopen the archive with the expected publisher public key and check signed descriptor equality, archive length and digest before handing it to the installer.

Output consists of `workload.mvmpkg`, `environment.json`, the raw 32-byte `publisher.pub`, the exact verified `revocations.json` and its `.bundle`, and public facts for the existing installer inputs: publisher key ID, revocation publication, allowed IPv4, port, archive SHA-256 and byte length. The operator independently selects the trusted publisher and current revocation publication, plus the plan issuer ID and peer TLS name. The installer and peer receive only their respective private keys as prescribed in the [witness instructions](../../tools/installed-flowmux-witness/README.md). Keys and issued plans stay out of logs and public evidence.

## Acceptance

First demonstrate refusal for a wrong ELF digest, noncanonical IPv4, existing or unsafe destination, and sidecar/descriptor argv disagreement. Each leaves no published packet and preserves existing files. On Linux, reopen a genuinely signed fixture and verify its kernel, guest runtime, rooted workload and operator inputs. Run the existing one-guest witness with the verified Firecracker 1.17.0 executable. Require the denied request, exact 5,804-byte binary echo and EOF, original signed audit and clean retirement; record actual TLS bytes, timing and CPU. Record the MVM host revision and the official rc.2 guest runtime identity separately. WAN, fleet and FIPS qualification retain their existing gates.
