# Current-MVM preparation record

[Receipt](receipt.json) records the mechanical port of the existing fourteen-file
mailbox primitive patch onto upstream commit
`6ae461a54d85a1b70477c1efb97ea4a33ef3b331`.

The [new reproducible bundle](../../../../../crates/spiralcore-mvm-live/mvm-service-patch/README.md)
is separate from the older measured source. All legacy hunks applied with zero
fuzz and no conflict; one supervisor hunk shifts by 46 lines. The current
ResourceLedger and Base64 changes remain intact. Local preparation and remote
patched-file checks agree, and the minimal locked core/net typecheck passed.

No guest was launched and no source was substituted into the old measured pilot.
This is source preparation, not service completion or native-performance evidence.

The subsequent [Reply authorization regression](reply-operation-red-green.json)
records a tests-first canonical correction in `0002`: proper Reply-only grants
are admitted, Send-only grants cannot authorize Reply, and Call/Cast keep Send.
The new three-test subset failed twice before the correction; all fifteen
transport tests passed on the final formatted source afterward. The final bundle
applies both patches and verifies its fourteen resulting file hashes. This fixes
the scoped authority defect while leaving the wider delivery gates open.

[Preparation portability evidence](portability.json) records real archive/patch/
hash integration tests on macOS and Linux under controlled hash-tool availability,
global CRLF settings and spaced paths. It also checks native Cargo-path generation
using a simulated `cygpath`. Actual Windows CI remains an independent required
gate; these checks do not constitute native Windows or MVM-backend evidence.

The [prepared-header accessor record](prepared-header-red-green.json) adds the
third patch and records its RED missing-method failure, sixteen passing upstream
transport tests, strict lint, and final three-patch package checks on macOS and
Linux. The immutable getter exposes metadata only. It does not prove configured
producer keys or permit omission of canonical verification.

The [dual-URL preparation record](dual-url-preparation.json) covers the subsequent
Cargo mapping change: upstream and contribution-fork URL blocks map eighteen
entries onto the same nine physical crate paths. Real macOS/Linux package tests
check both blocks and native-path conversion; this changes no MVM source or wire
semantics. The prepared Git repository also inherits the original shallow
boundaries so its publisher can traverse the borrowed source history safely.
