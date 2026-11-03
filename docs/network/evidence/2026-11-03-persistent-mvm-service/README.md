# Persistent-service checkpoint evidence

Read the [results and limitations](../../persistent-mvm-service-results-2026-11-03.md)
first. Product source is `7531985`, merged through PR #54 at `47324dc`.
The continuing acceptance plan and issues #43/#49 remain open.

- `before/` and `after/`: 18 original Rust host-process object-campaign receipts,
  three trials per raw/zstd/Combined arm, including setup and cleanup fields.
- `registration.json`: exploratory baseline timing and separately registered
  five native cells, resource limits and explicit unexecuted gates.
- `native/`: five original public Firecracker receipt sets; no VM disk, private
  bootstrap, cache payload or launch directory. `native-receipts.tgz` and its
  56-member `native-members.sha256` retain transfer custody.
- `native-cleanup.txt`: verified successful disposable stores removed on Wick;
  apparent byte lengths do not claim freed filesystem blocks.
- `validation/`: actual locked-release build/test/Clippy output. Zero-test
  example targets are retained and are not counted as tests.
- `qualification/`: successful seven-native-client and Linux CI identity,
  original archives, inner checksum verification and explicit role limits.
- `coverage/`: isolated instrumented-test results, executable/source identities,
  exact changed diff, line hits, derivation and uncovered lines. Coverage is
  separate from every normal-binary performance measurement.
- `reviews.md`: independent source/security/infra/evidence approvals and hashes.
- `source-sha256.txt`: all 272 tracked crate files exported to Wick.

The root `SHA256SUMS` binds every other file in this packet, including original
inner manifests. Verify from this directory with `shasum -a 256 -c SHA256SUMS`.
Windows original inner manifests use CRLF; stream-only normalization for their
verification is recorded in `qualification/README.md`. Never rewrite original
receipts to manufacture a passing gate. This is local component/native evidence;
canonical WAN, full persistent lifetime, machine-wide owner exclusion and guest
plaintext/cache aggregation are not completed by this packet.
