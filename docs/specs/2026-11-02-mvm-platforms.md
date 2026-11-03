# MVM endpoint portability and qualification contract

Date: 2026-11-02. Tracker: [#46](https://github.com/R4-Laboratories/SpiralCore/issues/46),
under [delivery #42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
This defines release qualification work; the candidate rows below are not claims
that every platform already works. Final WAN acceptance is tracked in
[#47](https://github.com/R4-Laboratories/SpiralCore/issues/47).

## Roles and upstream boundary

MVM revision `6ae461a54d85a1b70477c1efb97ea4a33ef3b331` documents Linux
x86-64/AArch64 with `/dev/kvm`, Apple Silicon macOS 26+ with HVF, and the bundled
Apple Silicon macOS 13–25 libkrun preview. Its README explicitly retains an open
live-hardware witness for that preview. It does not offer a native Windows MVM
host or a Windows guest kernel. These are the upstream host boundaries, not an
excuse to omit the requested native Windows remote endpoint.
[Pinned MVM README](https://github.com/tinylabscom/mvm/blob/6ae461a54d85a1b70477c1efb97ea4a33ef3b331/README.md).

| Role | Responsibility | Qualification boundary |
|---|---|---|
| Native MVM host | Boots the Linux guest and owns canonical service/vsock binding and per-VM enforcement | Linux/KVM and supported Apple Silicon macOS backends |
| Linux MVM guest | Canonical mailbox endpoint and admitted FlowMux/owner-access behavior, no NIC | Actual guest architecture, kernel, boot and backend identity |
| Remote mailbox client | Authenticates, sends/receives canonical envelopes and receipts, observes call/cast lifecycle | Native Linux, macOS and Windows process; Windows uses network I/O rather than an invented Windows vsock host |
| Cooperating object gateway | Encodes/decodes object-v1 and owns authorized cache/storage and transport | Native OS, storage guarantees, carrier and exact-byte behavior |
| Packet/NIC attachment | OS-specific interface attachment outside the MVM guest | Linux TUN remains its own mapping; a native Windows/macOS NIC driver is not inferred from a working object client |

The same canonical MVM types/cryptography and SpiralCore codec implementation
serve these roles. Backend-specific socket and storage operations must not fork
the envelope, receipt, object identity, versioning or authorization rules.

## Confirmed starting facts

Current `live.rs` already guards its UnixStream implementation with `cfg(unix)`
and also implements TCP `SocketIo`. That location is not an unconditional Windows
compile blocker. The current MVM default dependency closure also contains
deliberate non-Unix fallbacks and target-gated Unix dependencies; do not replace
it based only on an `os::unix` text search.

A bounded cross-check ran on Wick against the isolated current-MVM prepared
tree, including the recorded mailbox primitive patch:

```sh
rustup target add x86_64-pc-windows-msvc --toolchain 1.97.1
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR=/var/tmp/spiralcore-mvm-service-20261102/platform-probe/target \
  cargo check --locked -p mvm-core --no-default-features \
  --target x86_64-pc-windows-msvc
```

Result: **exit 0**, 36.39 seconds; both `mvm-contract` and `mvm-core` checked.
Rust was 1.97.1 (`8bab26f4f68e0e26f0bb7960be334d5b520ea452`), LLVM 22.1.6,
running on `x86_64-unknown-linux-gnu`. The workspace lockfile before/after was
identical: `c9b6414ac28a1aaf93fb2aa74b77b3fcbe39889eaaddd2b681e7252358d1699e`.
Prepared mailbox source hashes were
`dd21a283ddb117ed42f6d0a07a9fa6d322345f4f23b1c5f6abe519f07b94cbff`
(`mvm-core/src/mailbox.rs`) and
`e03a04b8cddf623e9c76e6c7085b8d18a1e38a4b42e9ed074559744d2a3efc0a`
(`mvm-contract/src/protocol/mailbox.rs`).

This is compilation of the existing canonical default closure, **not** Windows
linking, Windows execution, new service qualification or WAN evidence. It gives
no basis for an immediate new `mvm-mailbox-core` crate. The chosen first approach
is to retain `mvm-core::mailbox` and fix observed endpoint portability gaps.
If subsequent native tests reveal a necessary boundary change, record that
evidence and preserve existing public types through aliases/reexports.

Definite remaining gaps from the current SpiralCore source are:

- `spiralcore-transfer::endpoint::{private_file,private_dir}` deliberately return
  `Unsupported` outside Unix. Windows durable object reception is not implemented.
  Publication also uses directory-file sync calls which need a qualified native
  Windows storage implementation.
- `spiralcore-gateway::measurement` calls Unix `clock_gettime`; portable process
  CPU measurement needs an explicit native implementation, such as the already
  used cross-platform `cpu-time` API, with the same documented accounting unit.
- `spiralcore-gateway::object_service` uses Unix file metadata/open options for
  input-file identity. Windows equivalents must preserve the checked input and
  race behavior rather than delete those checks.
- The opt-in `hardware-sha256` feature enables `sha2/asm`, which this repository
  already documents as unsupported on Windows. The portable Windows feature
  profile must use the shared non-assembly implementation and identical vectors.
- Existing Unix socket tests do not constitute Windows TCP execution evidence.
  Portable positive/negative counterparts must actually run on Windows.

## Candidate release matrix

First materialize a machine-readable support manifest containing role, OS build,
architecture, ABI, backend where applicable, toolchain/features, storage mode,
carrier profile, execution status and evidence links. A runner label is not an
immutable OS identity. Record the runner image version and actual OS/kernel/build
for every execution. Each row may be promoted only for the roles it proves.

| Candidate environment | Remote client / object gateway | Native MVM witness | Present evidence / remaining gate |
|---|---|---|---|
| Ubuntu 22.04, 24.04, 26.04 x86-64/glibc | Native matrix jobs for each release | KVM where actually available and admitted | Existing Wick Ubuntu 24.04 native witness; other release cells remain unqualified |
| Ubuntu 22.04, 24.04, 26.04 AArch64/glibc | Native ARM jobs for each release | AArch64 KVM hardware witness | No executed ARM host witness from this portability task |
| Linux distribution/ABI beyond the pinned manifest | No blanket claim from Ubuntu or a container test | Requires the documented host prerequisites and actual witness | Add explicit cells before advertising another flavor; musl is a separate ABI |
| macOS 26 and later Apple Silicon/HVF | Native client and gateway tests for each claimed version | Actual HVF guest/host and two-guest tests | Local machine currently reports macOS **27.0**, arm64; this is not a macOS 26 execution |
| macOS 13–25 Apple Silicon/libkrun preview | Native endpoint tests on claimed versions | Actual libkrun hardware witness | Upstream preview hardware gate remains open; unavailable versions stay blockers |
| macOS Intel client/gateway | Native supported runner/version tests | No MVM host claim from these tests | Client/gateway qualification only, unless upstream adds a native backend |
| Windows Server 2022 and 2025 x86-64/MSVC | Native executable tests, secure storage and WAN | No native Windows MVM host claim | Compile-only MVM default closure succeeded; native endpoint and storage tests remain required |
| Windows 11 AArch64/MSVC | Native executable tests, secure storage and WAN | No native Windows MVM host claim | Available runner is a test opportunity, not an executed result |
| Windows desktop x86-64 or another Windows release | Explicit native version cell before support claim | No native Windows MVM host claim | Server execution or ARM execution does not silently certify this cell |

GitHub currently documents standard public-repository runners for Ubuntu
22.04/24.04/26.04 on x64/ARM64, Windows Server 2022/2025 on x64, Windows 11 on
ARM64, and several Intel/Apple Silicon macOS releases. Use explicit labels and
check availability before dispatch; do not use a moving `-latest` label as a
release pin. Standard public-repository runners are a free test option, but their
availability does not prove KVM/HVF/libkrun execution privileges or every native
backend prerequisite. [GitHub hosted-runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
[runner images](https://github.com/actions/runner-images/blob/main/README.md).

## Storage, transport and test obligations

A Windows client may first be qualified for canonical ephemeral mailbox
operations without claiming a durable object receiver. A Windows object gateway
cannot be promoted by disabling private-file checks or returning Published before
equivalent durability. Add narrow platform storage operations with native private
access control, reparse/symlink protection, exclusive writer ownership, exact
readback, atomic publication and crash recovery. Prove the Windows flush/rename
semantics used before acknowledging durable commit. Until that succeeds, retain
the existing fail-closed `Unsupported` behavior and keep the receiver cell open.

The portable carrier uses authenticated TCP/network I/O and canonical encrypted
mailbox values. AF_VSOCK and Unix sockets remain platform-specific native guest
bindings. No UDP/QUIC socket may be opened under only a TCP grant, and no Windows
fallback may weaken peer identity, capability, epoch, crypto or relay policy.

Every declared platform row must execute canonical bytes/crypto/receipt vectors,
call/cast/reply and retry/duplicate/late/cancel/full/shutdown negatives, exact
object sizes/boundaries/edits and storage tests for any receiver role. Capture
actual test counts: a skipped Unix-only test target running zero tests is not
portable coverage. Cross-compilation, same-runner loopback and network shaping
must be labelled separately from native execution and real WAN.

Final WAN registration lists the actual ordered endpoint pairs, including both
directions, topology, direct/explicit-relay classification, source/binary/image
identities and workload/durability profile. Every advertised pair needs an
executed result; do not infer unrun Windows/macOS or architecture/version pairs
from a Linux anchor. Identical wire vectors are compatibility evidence, not a
substitute for that transport execution. All unrun cells remain visible in #46
and #47 and block the final MVM delivery claim.
