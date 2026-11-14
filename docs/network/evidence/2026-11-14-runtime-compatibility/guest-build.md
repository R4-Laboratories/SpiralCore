# Compatible unsigned guest build for both architectures

The canonical `mvmctl dev guest-bins` producer completed for **x86-64 and
AArch64** from contributor source
`9394d74f26d13424afeaade1ace9ece0f7a29dd8`. The clean rebuild's own manifest
reports that exact commit and a clean worktree. MVM's canonical
`dev guest-bins --check-release-archive` reader accepted the complete archive,
with **76 members** and matching checksum sidecar.

This is an **unsigned contributor build**, not an accepted MVM release, guest
execution witness or installed transfer result. The existing verifier and
release-authentication gate remain mandatory before the installed paired test.

## Artifact and declarations

The archive has 22,536,289 bytes and SHA-256
`d596b947bf9e91f50ea73a1e366b8b0226b157fed7580cbeaec41df3ceb33759`.
Its canonical reader is pinned separately in [guest-build.json](guest-build.json).
Each architecture's runtime assembly declares:

| Profile field | x86-64 | AArch64 |
| --- | --- | --- |
| Mailbox reader format | 1 | 1 |
| Maximum serialized boot record | 8,192 B | 8,192 B |
| Directed journal v1 | Present | Present |
| Object pair v1 | Present | Present |
| Sender ordinal v1 | Present | Present |
| Host/guest protocol, minimum | 2 / 2 | 2 / 2 |

The producer used MVM-pinned Zig 0.13.0 and cargo-zigbuild 0.23.0 with checked
archive hashes. Rust was 1.97.1. It reused the existing Cargo target under a
six-GiB memory cap, two-core CPU quota and two build jobs. Neither the older
signed runtime nor a previous artifact was patched in place.

The exact build command was:

```bash
cargo run --locked -p mvmctl -- dev guest-bins --out "$SC_UNSIGNED_RUNTIME_OUTPUT"
```

`SC_UNSIGNED_RUNTIME_OUTPUT` was a distinct output directory for each attempt.
The actual receiver-validation command was:

```bash
mvmctl dev guest-bins --check-release-archive "$SC_UNSIGNED_RUNTIME_ARCHIVE"
```

That command validates the archive and returns its manifest. It is not a
signature-verification claim. The supplementary qualification check separately
required clean source identity, both architectures, all three declared semantic
features and the 8,192-byte bound. Raw binaries stay on the Linux test host;
compact source, artifact, member/declaration and tool identities are in the
[structured receipt](guest-build.json).

## Retained provenance failure and correction

The first producer run succeeded and the canonical archive reader accepted its
contents. The supplementary clean-provenance check then refused it because its
manifest truthfully reported a dirty worktree. All tracked content and executable
modes matched the pinned source, but a 163-byte macOS AppleDouble `._.git` file
had traveled with the source export. The source identity observes untracked
files; matching only tracked bytes did not establish a clean checkout.

The metadata file was moved outside the source tree and retained with its hash.
A fresh producer invocation wrote a new archive in a separate output directory.
It reported clean source and passed the qualification check. All **76 member
hashes**, both runtime assemblies and both build-source fingerprints match the
first artifact. The original dirty artifact and log remain retained. The
[history receipt](guest-build-history.json) records both identities and the
metadata move; no archive manifest or provenance field was edited.

## Build costs and remaining execution

| Build command | Wall time | User + system CPU | GNU-time maximum RSS |
| --- | ---: | ---: | ---: |
| First build | 1,985.82 s | 3,616.07 + 160.56 = 3,776.63 s | 3,026,900 KiB |
| Clean rebuild with compiled cache | 24.75 s | 25.19 + 1.64 = 26.83 s | 1,735,360 KiB |

These are build costs, including child compilation. GNU-time maximum RSS is
not simultaneous aggregate host memory. They are not transfer CPU, deployed
runtime latency, throughput or energy measurements.

Next acceptance still requires an authorized signed runtime, feature-specific
live guest behavior, paired exact transfer/ACK/readback and original-owner
retirement. Host/guest runtime changes for this checkpoint do not establish
WAN, platform, fleet-capacity or FIPS qualification. Track those gates in
[#268](https://github.com/R4-Laboratories/SpiralCore/issues/268) and its parents.
