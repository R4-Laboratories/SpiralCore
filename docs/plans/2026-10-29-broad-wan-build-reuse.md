# Same-source WAN sender build reuse

Status: contract registered before implementation. This changes setup only, not the
registered sixty single-shard measurements, canonical plans, transfer/capture code,
trial ordering, publication readiness or evidence acceptance. No performance reruns.

Qualification-only runs always build/test/Clippy from their exact checkout, build the
three release sender binaries, and pass live Linux capture qualification. Only then
may they upload a one-day immutable sender artifact containing exactly those binaries
and a bounded manifest. The manifest binds source, repository, workflow path, producer
run/attempt, compiler/target/features/lock and each binary length/SHA256. No credentials,
environment dumps, caches, capture payload or arbitrary files belong in that artifact.

Before sealing the producer artifact, a qualification-only diagnostic also installs
pinned cargo-llvm-cov0.9.1 and Rust1.97.1 llvm-tools in an isolated target/environment.
It runs all-target tests including ignored large fixtures (300-second timeout), builds
instrumented binaries separately (300 seconds), and runs the instrumented live Linux
capture qualification (180 seconds). The JSON report records actual coverage without
an assumed percentage gate. Release executable checksums are checked before and after,
including failure; stage/exit receipts and available diagnostics are retained. This
instrumentation never supplies measured sender binaries or replaces ordinary release
capture qualification. Privileged child-process instrumentation may have narrower
coverage than the parent process, so the actual report determines its demonstrated scope.

Measured runs may specify both numeric SPIRALCORE_WAN_BUILD_RUN and
SPIRALCORE_WAN_BUILD_ARTIFACT, or neither. These pins are forbidden in qualification
mode. Reuse requires the originating GitHub run to have the identical full source SHA,
repository/head repository and workflow path, successful completion and attempt one.
Its capture qualification step must have succeeded. The selected non-expired artifact
must belong to that exact run and have the expected name. Missing, corrupt or mismatched
artifacts fail before downloaded code executes; no automatic source-build fallback.

A small independent Rust validator built from the current checkout is the bootstrap
trust boundary. It uses bounded typed JSON with unknown/duplicate field rejection,
fixed file names and streamed bounded hashes, rejects symlinks/nonregular/extra files,
and checks explicit compiler, target, feature and lock expectations. Downloaded binary
permissions are changed and files copied into their execution location only after all
checks succeed. API projections contain only required nonsecret identity fields. The
workflow requires read-only Actions permission and pins third-party actions by commit.

Archive custody is checked before extraction: download only the selected artifact ZIP
with a timeout and file-size limit, compare its actual streamed SHA256 and length to the
authenticated API response, then require exactly four flat regular entries and bounded
expanded sizes. The standalone helper uses pinned zip7.4 with default features disabled
for metadata only. A small raw footer count/range check precedes its allocation and
cross-checks its selected directory; ZIP64 is bounded too. System unzip extracts only
after the archive receipt passes, followed by full extracted-manifest/binary verification.
The manifest also binds the capture qualification's actual controller executable hash.

Native qualification exposed that Bash3.2 does not apply errexit to standalone failing
`[[...]]` tests. Therefore the prior eighty-case shell receipt is not sufficient proof
of negative-path rejection. Workflow guards now fail explicitly, and Rust assertions
exercise the original eighty cases plus reuse cases. Historical profile guard scripts
remain unchanged: their independently checked positive identities remain evidence, but
unqualified rejection-path claims must not be inferred from `set -e` alone.

Every measurement runner still installs capture tools, records its current platform/CPU,
and executes fresh live Linux capture qualification plus per-shard route/offload checks.
Producer build provenance is retained separately from the current runner's environment.
The current single Mac origin/cache/token/tunnel lifecycle and exact reopened validation
remain unchanged. Fresh-source mode retains all existing source tests/builds.

Qualification: observe RED then GREEN for configuration pin pairing/mode restrictions,
strict manifests, wrong source/repository/workflow/run/attempt/compiler/feature/lock,
failed producer/capture qualification, expired/wrong artifact, missing/extra/symlink/
oversized/corrupt files and bounded input/overflow handling. Preserve the existing eighty
workflow guard checks and add reuse cases. An independent architecture/security review
and final exact-source Linux qualification are required before campaign freeze.

The intended setup saving is avoiding repeated compilation and source test execution
for identical verified source; it is not a measured transfer speed or energy claim.
No five-origin bundle, new transport, replay retry, cached capture qualification or
automatic source substitution is included.

Custody: retain the producer artifact through all consumers. Per-shard measurement
artifacts may be removed only after recording their exact ID/digest, downloading and
validating the raw evidence, completing reopened exact-byte verification, and preserving
raw local evidence plus compact Git receipts. Cleanup selects only that newly owned
measurement artifact; it never removes the producer or drops scientific observations.
One-day retention remains an upper bound, not permission to lose undownloaded evidence.
This change authorizes no artifact deletion by the implementation task.
