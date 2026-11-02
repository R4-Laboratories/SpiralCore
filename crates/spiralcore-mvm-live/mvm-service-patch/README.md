# Current-MVM mailbox service primitives

This is a separate preparation bundle for MVM commit
`6ae461a54d85a1b70477c1efb97ea4a33ef3b331`. It ports the retained fourteen-file
experimental mailbox primitive patch onto that current upstream source. The
older bundle in `../mvm-patch/` and all measurements pinned to it are unchanged.

The port applies with zero fuzz and no conflicting/rejected hunks. The hostd
supervisor insertion moves by 46 lines to line 981; its current surrounding
lifecycle logic is preserved. Current MVM's transport `ResourceLedger` and
private Base64 imports remain upstream-owned and unchanged. The first patch introduces
no new semantic changes beyond the retained patch. A second focused patch fixes
canonical outbound Reply authorization: Reply messages require the Reply
operation, while Call/Cast retain Send. The old Send-only gate rejected proper
Reply-only grants and could admit a Reply under a Send-only grant/context. Three
regression tests cover both boundaries and unchanged Call/Cast behavior; the
existing error variant is retained for compatibility.

The bundle includes canonical verified-open/transport accessors and the existing
experimental dedicated mailbox mapping on port 5257. It is preparation for the
production service work, not completion of issue #4629, an upstream port
reservation, accepted contribution or a new native-performance qualification.
The production service implementation and tests are developed in SpiralCore
before an upstream contribution is proposed.

The upstream Apache-2.0 notice remains in `LICENSE-MVM`. `provenance.json`
identifies the current base blobs, original patch and all patched hashes.
`bundle.sha256` checks this preparation package. No full MVM vendoring,
credentials, build products, Cargo manifest changes or lockfile edits are bundled.

Supply an existing Git repository containing the exact current commit and a new
output path whose parent exists:

```sh
bash crates/spiralcore-mvm-live/mvm-service-patch/prepare.sh \
  /absolute/path/to/mvm /absolute/path/to/new-prepared-mvm
```

Preparation archives that pinned commit, independently of the input repository's
checked-out branch or local edits. It never switches, modifies or copies private
working files from the input. The resulting isolated `source/` tree receives all four patches in order,
all twenty-one final patched hashes are checked, and a nine-crate `patch.toml` is emitted.
It reserves 4 GiB disk plus extraction allowance, downloads nothing and invokes
no Cargo command. Failure retains its owned output for diagnosis.

Preparation is separate from builds. Use the emitted config only with a consumer
explicitly pinned to the resulting published contributor-fork revision
`ae12e9deb41cc9a3264196ce22ebc72d4042883d` (or a separately reviewed equivalent)
and its matching dependency resolution. Do not substitute this bundle into the old measured pilot or rewrite
its lockfiles and evidence. Native tests require an approved Linux/KVM execution
boundary; no guest has been launched by this preparation bundle.

## Preparation portability

Preparation chooses `sha256sum` when available and otherwise uses `shasum -a 256`.
It refuses a missing SHA-256 implementation before creating output. In Git Bash,
`cygpath -m` converts each dependency path to native mixed form such as
`C:/work/mvm/source/crates/mvm-core`; POSIX platforms retain canonical POSIX paths.
Both forms are TOML-escaped and support spaces.

Archive creation explicitly disables Git CRLF conversion, and the fresh output
repository pins `core.autocrlf=false` and `core.eol=lf`. The bundle's scoped
`.gitattributes` also preserves LF during checkout, so hashes and patch context
remain canonical on Windows. No input repository or global Git setting changes.

Run the preparation integration checks with a repository containing the pin:

```sh
bash crates/spiralcore-mvm-live/mvm-service-patch/test-prepare.sh \
  /absolute/path/to/mvm
```

These checks actually archive the committed source, apply all four patches and
verify all twenty-one hashes and the complete published `ae12e9d` Git tree under SHA-256-only and fallback tool availability,
missing-tool refusal, a global CRLF preference, paths containing spaces and a
simulated `cygpath` conversion. Simulation checks configuration generation;
executed native Windows builds/tests remain a separate platform gate.

The third patch adds only `PreparedMailboxEnvelope::header()`, an immutable
metadata borrow. It does not prove that the configured sender signing/encryption
keys created the envelope; canonical verification remains required. No key-proof,
authority, wire-format or performance shortcut is introduced.

The generated Cargo config contains two source URL blocks, for
`https://github.com/tinylabscom/mvm` and the outside-contributor fork
`https://github.com/Casey-allard/mvm`. Both map the same nine crates to the same
physical prepared tree. This supports the pinned contribution dependency while
preserving one canonical type/protocol implementation; it creates no mode split.

## Fourth patch: explicit Firecracker state authority

The first three patches produce contributor-fork revision
`5766754fbaae59dbee548f968716a514d3a796c1`. The reviewed fourth patch produces
published contributor-fork revision `ae12e9deb41cc9a3264196ce22ebc72d4042883d`,
tree `f3e3e8c8558a8410cc23890467ea21d309da7079`, on
[Casey-allard/mvm](https://github.com/Casey-allard/mvm/tree/ae12e9deb41cc9a3264196ce22ebc72d4042883d).
This is publication in an outside-contributor fork, not an upstream PR or
acceptance claim.

A native two-guest failure exposed a directory mix-up: canonical Firecracker
sockets may relocate to a short namespace, while the process marker remains in
the original VM state directory. The old writer verifier inferred state from the
socket parent. `FirecrackerIO::new` now requires the owned VM state directory
explicitly, and verification, restore loading/startup and teardown retain that
authority independently of socket placement. All constructor callers are
migrated. Existing configured-drive checks, PID identity, actual-process file
descriptor custody, strict stop and scope enforcement remain in place. There is
no marker fabrication, path-shortening workaround, trusted-builder bypass or
weakened descriptor check.

`0004-provenance.json` records the original semantic RED, independent source
review, published source/tree and seven changed-file hashes. The relocated
socket unit filter passes four tests, the complete `fc::io` suite passes sixteen,
and all-target checks and strict Clippy pass for `mvm-backends`, `mvm-client` and
`mvm-runtime`. Root independently repeated the sixteen-test suite after
publication. These tests use a bounded owned process/HTTP fixture, not a VM or
KVM. Full native requalification remains separate.

The final `patched-files.sha256` is the twenty-one-file union relative to upstream
`6ae461a`: the retained fourteen files plus these seven. Its hashes are generated
from the published `ae12e9d` objects. `provenance.json` retains upstream base blob
identities for every final file. The fourth patch and its detailed provenance
are included in `bundle.sha256`. Unified-diff blank context lines are retained;
the scoped patch whitespace attribute preserves their syntax while preparation
still uses `git apply --whitespace=error-all` to reject invalid source additions.

Registration was exercised on Wick using the existing read-only Git cache that
contains the exact upstream base. All four preparation variants passed. A fresh
prepared extraction also reconstructed Git tree
`f3e3e8c8558a8410cc23890467ea21d309da7079`, exactly equal to published `ae12e9d`;
its owned disposable source was removed after verification. The integration
script now asserts that complete tree identity for each successful variant.
This is source-preparation integrity evidence, not a guest launch or protocol
performance result.
