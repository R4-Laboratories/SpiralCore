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
working files from the input. The resulting isolated `source/` tree receives all three patches in order,
all fourteen final patched hashes are checked, and a nine-crate `patch.toml` is emitted.
It reserves 4 GiB disk plus extraction allowance, downloads nothing and invokes
no Cargo command. Failure retains its owned output for diagnosis.

Preparation is separate from builds. Use the emitted config only with a consumer
explicitly pinned to this current MVM commit and its matching dependency
resolution. Do not substitute this bundle into the old measured pilot or rewrite
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

These checks actually archive the committed source, apply all three patches and
verify all fourteen hashes under SHA-256-only and fallback tool availability,
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
