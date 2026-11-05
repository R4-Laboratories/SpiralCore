# Guarded startup workspace and installed Firecracker launch attempts

The original tenant grant now separates the transient signed-image verification workspace from endpoint funding. Runtime acquires the one-use startup workspace before full verification, completes it after retaining the verified snapshot, and prepares the endpoint afterward. Disk custody stays with the original grant. The selected fixture retains the 256 MiB tenant limit. Its managed quotes do not establish process-wide peak memory.

The test-first compile produced four expected missing workspace API diagnostics. The guarded protected binary then passed 30 focused checks, with three native or fixture-specific tests ignored in that command. All 5,409 pinned inputs matched their before and after readbacks. The four native helpers linked against that exact graph. The source patches and all phase receipts are in the archive.

Both subsequent installed runs used the same protected binary and helper identities with fresh copies of the signed synthetic fixture. The first reached the state directory privacy check and refused the directory created under umask 0022. The second explicitly verified umask 0077 through the actual launcher identity, started the endpoint and telemetry provisioning, and reached the original Firecracker guardian. The guardian control pipe closed without a ready response, and original cleanup remained blocked. No running guest was observed in either run. No successful transfer or retirement was recorded.

The qualification feature is scoped to the development candidate. These records provide no WAN, scale, or FIPS result.
