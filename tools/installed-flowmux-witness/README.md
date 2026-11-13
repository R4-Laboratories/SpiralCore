# Installed FlowMux qualification

This opt-in Linux package installs a signed synthetic workload and runs a cooperating peer against the original installed SDK controller. It exercises guest application bytes → authenticated FlowMux/vsock → admitted host endpoint → pinned TLS/Object-v1 peer, followed by original-owner shutdown. It does not create a guest NIC, replace MVM's mailbox, or authorize an SSH service.

The workload requests one denied destination and one permitted literal IPv4 destination. The permitted stream contains 200 identical 29-byte JSON lines followed by `00 ff 00 00`: exactly **5,804 bytes**. The peer requires every byte and the original EOF, echoes the same bytes and half-closes. An extra byte, truncated body, wrong peer identity, different destination or changed signed role refuses the attempt.

## Qualification boundary

The first installed native result is recorded in [verified-echo.md](../../docs/network/evidence/2026-11-12-installed-flowmux-startup/verified-echo.md). It used its own recorded source and executable identities. This package is a successor with explicit operator inputs; those earlier results do not qualify a new package build.

The installer and peer use the same pinned MVM revision. Their direct SpiralCore dependencies use the exact Git revision used by `mvm-hostd`, so the concrete scope, sender and receiver types share one Cargo identity. The controller is a separate executable. Inspect the manifests and resolved lockfiles when changing a pin; do not substitute local source paths or treat a type-check against existing artifacts as a locked build.

## Prepare the authenticated inputs

Use two separate, fresh, operator-owned role directories beneath a private parent. Directories must be `0700`; input files must be regular, single-link, owner-only `0600` files. The reader rejects unsafe files instead of repairing their permissions. Private keys and issued operator packets must never enter Git, CI artifacts or a public result archive.

Both role directories contain the same bounded `profile.json` with these fields:

| Field | Required value |
| --- | --- |
| `issuer_id` | The explicit plan-issuer ID, at most 128 bytes |
| `allowed_ip` | The literal IPv4 compiled into the sealed guest |
| `port` | `38443`, matching the sealed role |
| `server_name` | The selected cooperating peer TLS name |
| `archive.sha256` | Lowercase SHA-256 of the exact `.mvmpkg` bytes |
| `archive.decoded_bytes` | The exact file length of that archive; this historical field name does not mean guest application bytes |

Unknown JSON fields refuse. The installer additionally requires:

| File | Meaning |
| --- | --- |
| `install.json` | `publisher_key_id` and `expected_revocation_publication`, selected independently by the operator |
| `workload.mvmpkg` | The genuinely signed package with the exact guest, rootfs, kernel and descriptor |
| `environment.json` | The operator's expected signed environment descriptor |
| `publisher.pub` | Raw 32-byte Ed25519 publisher public key |
| `revocations.json` and `revocations.json.bundle` | Current authenticated image-set revocations and their signature bundle |
| `issuer.der` and `issuer.spki` | The matching Ed25519 plan issuer private PKCS#8 DER and public SPKI DER |
| `peer.spki` | The peer's public SPKI DER, used to derive its TLS pin |

The peer role requires only `profile.json`, `issuer.spki` and `peer.der`, its own Ed25519 private PKCS#8 DER. **The installer does not need the peer private key, and the peer does not need the issuer private key.** Input directories are explicit provisioning boundaries, not a claim of isolation from a trusted host administrator.

Build the guest from [spiralcore-flowmux-proxy-guest.rs](../ari-own-mailbox-witness/paired-guest/src/bin/spiralcore-flowmux-proxy-guest.rs), with `SPIRALCORE_FLOWMUX_FIXTURE_IPV4` set at compile time. The authenticated environment must select `x86_64-linux`, one CPU, 256 MiB, only the egress capability, no volumes, secrets or services, and this exact argv:

1. `/usr/local/bin/spiralcore-flowmux-proxy-guest`
2. `denied.flowmux.test:38443`
3. The selected canonical IPv4 followed by `:38443`

Its allow-list contains only the third argument. Changing the address requires rebuilding and genuinely resealing the role. The package's environment must equal `environment.json`; changing an unsigned runtime argument is not sufficient. The existing [native role preparation instructions](../ari-own-mailbox-witness/paired-native.md#literal-ipv4-role) describe the measured role and its sealing boundary. Use the canonical MVM signed-release producer and current revocation authority; a locally chosen publisher in this disposable Dev installation gains no official publisher authority.

## Build and select the real controller

The package has no default installed-host feature. A build that omits `installed-linux` does not qualify either binary. On the approved Linux build surface:

```bash
cargo +1.97.1 test --locked --manifest-path tools/installed-flowmux-witness/Cargo.toml \
  --features installed-linux --all-targets
cargo +1.97.1 clippy --locked --manifest-path tools/installed-flowmux-witness/Cargo.toml \
  --features installed-linux --all-targets -- -D warnings
cargo +1.97.1 build --locked --manifest-path tools/installed-flowmux-witness/Cargo.toml \
  --features installed-linux --bins
```

Use the existing shared target and bounded build jobs. Build MVM's `mvm-host-agent`, `mvm-network-endpoint`, `mvm-signer-helper` and the guest runtime against the same selected MVM source. Build the SpiralCore `installed_flowmux` integration target with `canonical-host`. Before executing it, require the executable to list `installed_original_flowmux_workload: test`; a successful zero-test command is a refusal condition in the runner. The helper directory must also contain the normally provisioned runtime dependencies required by those helpers.

## Run on an approved Linux/KVM host

This script performs privileged mount-namespace operations. Use an explicitly approved test host with already-authorized `/dev/kvm` access, passwordless sudo for the qualification, GNU time, jq and normal util-linux/e2fsprogs tools. It preserves the operator's original supplementary groups and refuses an occupied original Firecracker/helper lane; it does not stop another installation. The supplied programs must be verified builds selected by the operator. Their hashes are retained in the result directory.

Set these absolute paths to the actual built programs and staged directories. Missing values fail before launch:

```bash
bash tools/installed-flowmux-witness/run-linux.sh \
  "${SC_FLOWMUX_INSTALLER:?set the built flowmux-install executable}" \
  "${SC_FLOWMUX_PEER:?set the built flowmux-peer executable}" \
  "${SC_FLOWMUX_CONTROLLER:?set the installed_flowmux test executable}" \
  "${SC_FLOWMUX_HELPERS:?set the matched MVM helper directory}" \
  "${SC_FLOWMUX_FIRECRACKER:?set the verified Firecracker executable}" \
  "${SC_FLOWMUX_ISSUER_INPUTS:?set the private installer input directory}" \
  "${SC_FLOWMUX_PEER_INPUTS:?set the separate private peer input directory}" \
  "${SC_FLOWMUX_RESULTS:?set an existing private result parent}"
```

The runner creates a fresh 512 MiB ext4 filesystem in a disposable mount namespace, drops back to the original unprivileged operator and starts with an explicit environment. Address-space limits are 512 MiB for installation, 192 MiB for the peer and 768 MiB for the controller. These limits and the one-guest role do not establish a two-guest, fleet or throughput capacity envelope.

The peer's readiness pipe precedes controller dispatch. The controller syncs and publishes the original guest tuple before dispatching the entrypoint. The peer therefore reads that tuple once after accept and refuses a missing or mismatched record. Neither a timer nor a recreated tuple supplies authority.

## Read and retain the result

The private result directory retains executable hashes, installer/peer/controller output, separate GNU time records, namespace status, filesystem measurements, copied signed evidence and original inode observations. A passing sample requires SDK, peer and namespace statuses all zero, exact guest readback, the expected signed allow and deny, and original process/store retirement evidence. A retained cache or journal balance is not an assertion that every reservation is free.

The peer reports both TLS directions and accept-to-half-close time. The controller reports its original dispatch and lifecycle observations. GNU time reports each process's wall, user/system CPU and maximum RSS. Keep startup, guest CPU, controller CPU, TLS bytes and application bytes separate. This local cooperating-peer test has no raw/zstd control and does not establish physical NIC traffic, WAN speedup, FIPS validation or physical energy savings.

The script verifies that its original mount and loop attachment are absent, then retains the bounded backing image for independent audit/signature readback. Remove that owned image only after the signed evidence and original lifecycle have been checked and the compact result has been preserved. Do not remove an attached image or publish the raw operator packet.

Preflight controls can run without KVM:

```bash
bash tools/installed-flowmux-witness/run-linux.test.sh
shellcheck tools/installed-flowmux-witness/run-linux.sh tools/installed-flowmux-witness/run-linux.test.sh tools/installed-flowmux-witness/verify-run.sh
```

Four mocked cases check help, argument count, missing installer and missing named test. Seven structural result cases check a complete report, missing audit artifact, zero-test footer, failed retirement, incorrect peer byte counts, missing named invocation and nonnumeric measurements. They execute no VM, mount or image allocation. Structural checking does not authenticate an audit signature or establish native-host acceptance.
