# Reproduce the experimental MVM source patch

This directory contains the exact 14-file patch against MVM
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b`. It adds the canonical verified-header
and transport-byte accessors plus the dedicated experimental mailbox service on
5257, including a host-selected bounded bandwidth profile. `standard` retains
4 MiB/s; `experimental64` selects 64 MiB/s. Both retain the 8 MiB burst and the
existing stream/idle limits; other service policies are unchanged. This is a local
integration patch, not an upstream port reservation or a
claim that MVM's mailbox delivery issues have been completed.

The upstream Apache-2.0 license is retained as `LICENSE-MVM`; preparation also
preserves the original license and notices in the exact committed source archive.
The patch records the local experimental changes separately from that base.

`provenance.json` records the original Git blob identities and patched SHA-256
hashes. `bundle.sha256` checks the patch, provenance, file-hash list, preparation
script and retained license. The patch includes only the listed source/test changes: no instruction
files, private data, keys, lockfile edits or generated builds are bundled.

Supply an existing MVM checkout at that exact revision and a **new** output
directory whose parent already exists:

```sh
bash crates/spiralcore-mvm-live/mvm-patch/prepare.sh \
  /absolute/path/to/pinned-mvm /absolute/path/to/new-prepared-mvm
```

The script archives the committed base into the new isolated `source/` directory,
checks and applies the retained patch, verifies all 14 resulting source hashes,
then prints the absolute path to its generated `patch.toml`. Local changes and
untracked files in the input checkout are neither copied nor modified. It checks
for a 4 GiB free-space reserve plus extraction allowance, downloads nothing and
runs no Cargo commands. Failed output is retained for diagnosis; use a new output
path for another attempt. Neither input nor SpiralCore lockfiles are edited.

Use the emitted config with the dedicated live crate and its retained lockfile:

```sh
cargo +1.97.1 test --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --config /absolute/path/to/new-prepared-mvm/patch.toml
```

The six patched direct dependencies lead transitively to the same isolated MVM
workspace, avoiding mixed patched/unpatched contract types. Do not apply this
config to change the compatibility adapter's retained upstream lockfile. The
unpatched adapter remains independently buildable with its default features.

Preparation verifies portability and source identity. Actual build, guest/vsock
execution and security qualification are separate recorded checks. If upstream
source changes are revised, regenerate the patch and provenance together and
repeat preparation checks before claiming reproducibility.
