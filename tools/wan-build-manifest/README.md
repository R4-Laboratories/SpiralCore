# Source-qualified WAN build artifacts

This small standalone Rust package is compiled from the checked-out workflow source.
It validates downloaded senders before executing them; it is not delivered by, or
trusted from, the artifact it validates. It changes no transfer protocol or benchmark.

The workflow is the authenticated caller: GitHub API responses are obtained over the
GitHub CLI's authenticated connection, then projected to the required typed fields.
The helper does not authenticate arbitrary caller-authored JSON or constitute a signed
attestation. Source checks, repository/workflow/run identity and archive digest checks
must remain in the workflow. Never run `create` as a substitute for those source tests.

The four commands are:

```text
spiralcore-build-manifest create EXPECTED_JSON NEW_ARTIFACT_DIR CAPTURE_QUALIFICATION_JSON
spiralcore-build-manifest proofs EXPECTED_JSON RUN_PROOF_JSON ARTIFACT_PROOF_JSON NEW_RECEIPT_JSON
spiralcore-build-manifest archive EXPECTED_JSON RUN_PROOF_JSON ARTIFACT_PROOF_JSON ZIP NEW_RECEIPT_JSON
spiralcore-build-manifest verify EXPECTED_JSON RUN_PROOF_JSON ARTIFACT_PROOF_JSON ARTIFACT_DIR NEW_RECEIPT_JSON
```

`create` seals exactly three freshly built binaries after successful source tests,
Clippy and Linux capture qualification. It checks the qualification's controller hash.
`proofs` rejects a mismatched or unsuccessful producer before downloading its artifact.
`archive` checks the actual archive SHA256 and length against authenticated GitHub
metadata, then exactly four flat regular members and size bounds before extraction.
`verify` checks strict manifest identities and every extracted binary length and hash.
Every receipt is create-new. No command invokes a downloaded binary.

Text manifests/proofs are bounded to128KiB; the capture receipt to16MiB; binaries to
32MiB each; the downloaded ZIP to100MiB; and its raw central directory to64KiB with
exactly four entries before the ZIP library allocates its member table. Standard and
bounded ZIP64 footers are supported. Stored/deflate metadata is accepted; zip's default
features are disabled and it does not decompress. The workflow uses system unzip with
fixed redirected output paths, file-size limits and mode0600, then verifies extracted
bytes before copying/chmod. Tokens and raw secret configuration are never inputs.

The current consumer runner still performs live capture qualification and records its
own host/offload state. A reused sender's compiler/features/source remain producer
provenance, separate from that current environment. Missing/expired/mismatched artifacts
fail closed without source-build fallback or measured retries.

Qualification uses runtime RED→GREEN corruption/identity/configuration/archive checks,
an actual CLI seal/proof/archive/verify roundtrip, and Rust assertions over the real
workflow guard. The original eighty guard cases remain, with eleven reuse cases.
The Bash3.2 standalone-`[[...]]` errexit limitation discovered here is documented in the
registered plan; assertions and guard failures are explicit instead of relying on it.

```sh
cargo test --locked --manifest-path tools/wan-build-manifest/Cargo.toml --all-targets
cargo clippy --locked --manifest-path tools/wan-build-manifest/Cargo.toml --all-targets -- -D warnings
cargo fmt --check --manifest-path tools/wan-build-manifest/Cargo.toml
```

The parent campaign records exact test/source hashes and independent review. Native
tests do not replace final exact-source Linux producer and artifact qualification.
