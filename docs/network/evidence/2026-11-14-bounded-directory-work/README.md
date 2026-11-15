# Bounded owned-directory work qualification

This packet qualifies the fixed caller-buffer directory slice in SpiralCore PR #289 for issue #258. It preserves the full five-part MVM delivery program in #42. Installed consumer adoption follows publication of the reviewed core. Native process fixtures do not establish booted MVM, backend, WAN, fleet, FIPS/CUI or host-blind acceptance. Ari's compatible signed runtime and private MVMD remain external gates; no final upstream MVM contribution is submitted.

**Issue #258 remains open.** Consumer adoption and destruction-before-credit accounting are still required, alongside broader workspace/funding acceptance. All five #42 outcomes remain active; this packet is one component checkpoint and does not close installed MVM, platform/backend, WAN, fleet or security acceptance.

## Source and boundary

The reviewed candidate is `44e895503072984a05fa61c00e832aeedcc70389`, tree `66ba73a347d82acd45a472831df9f4a8dbb07e84`. Native CI uses merge `f5eadb87352d3fc7e010c09857f4aeff86216c2f`, whose tree matches the candidate. PR #289 merged at `29393e9b476daeef41f51485d2fb8829c4bbefde`; its tree also matches the reviewed and tested source. Independent final source/security/artifact review passed before merge.

Each original store owns two fallibly allocated 16 KiB directory buffers. The accepted retained vector capacities total exactly 32,768 requested bytes and remain attached through successful retirement, quarantine and final destruction. The quote excludes allocator overhead/fragmentation, fixed bookkeeping, native stacks and kernel memory; it is not an RSS bound or an original-parent authorization. MVM's existing 1 MiB workspace floor remains required and will be checked against the core quote during consumer adoption.

A complete nested walk uses a nonblocking mutex borrow before opening iterator FDs. Contention/reentry returns the original Limit; poison returns Storage. Linux uses rustix RawDir over the caller buffer. The macOS helper consumes a freshly opened/verified offset and uses getattrlistbulk with a fixed aligned initialized buffer, checked attribute/error/record/name parsing, and distinct callback stop. No growing buffer, legacy DIR or ambient-path fallback is admitted. Borrowed names grant no authority. Existing fresh private/type/link/mount/identity, namespace, preflight, publication and original writer-lock checks remain. Core Rust keeps forbid(unsafe_code), and the canonical codec/protocol and existing Windows/Path behavior remain shared.

## Qualification

The final Wick run checks all 496 exported input hashes before and after execution. The helper and allocation witness executable hashes remain unchanged. Rust is 1.97.1; the owned user unit is configured for 6 GiB RAM, zero swap and 200% CPU, with two Cargo jobs and the existing shared cache. Its terminal receipt records MainPID0 and successful exit. No new VM, cloud host, Nix build or local Cargo target was created.

Wick passes eight parser tests, 23 owned primitive/work controls, seven owned integration/allocation/resource tests, 87 release FlowMux tests, and strict helper/transfer Clippy. New controls cover actual retained capacity, forced refills and fresh offsets, contention/reentry, poison without cleanup, original callback errors and unchanged buffer allocation identity through successful retirement. The existing deterministic two-store scan fixture holds both original buffer pairs and eight transient FDs, releases both workers and joins before assertions. Native macOS additionally executes the actual API with more than three buffers of names, independent offsets, early stop and refusal; Linux parser tests alone cannot qualify that call.

The populated fixture verifies 4,194,304 exact application bytes and records 4,250,368 request plus 55,808 reply bytes at the in-process encoded boundary. Wick's final requested Rust heap peak is 2,301,533 bytes, instrumented fixture wall is 1.671413189 s and process CPU is 128.073500 ms. Setup, sender-outside-dispatch, receiver dispatch, readback, recovery, refusal and retirement CPU/time fields remain separate in the raw row. These are observed whole-fixture Rust allocation requests, not isolated filesystem workspace or hard memory evidence. The modeled two-component quote is 18,681,720 bytes; it is not a permit. Direct libc allocation, allocator overhead, stacks/RSS and physical energy remain outside that measurement.

Native FlowMux rows retain 64 MiB exact application readbacks in each direction, actual TLS bytes, setup/process CPU and lifecycle time under the existing 2 s and 240 s guards. All 37 executed source-PR checks pass; four optional/manual jobs are skipped and remain NOT_RUN. Seven native FlowMux ZIPs pass 198 member hashes and 191 embedded checksums. Both macOS Intel/ARM rows pass ten helper tests, while Linux x64/ARM pass eight parser tests; all seven native rows pass 87 FlowMux tests. Component tests do not complete supported MVM/WAN acceptance.

Ten existing lockfiles change only by adding the local macOS helper package and transfer dependency edge. No prior package/version/source changes. Eight target-resolved graph pairs cover Linux x64/ARM and macOS Intel/ARM normal/build and feature closures: Linux is unchanged after source-root normalization; macOS adds the helper and its existing libc edge only. These Cargo graph observations run on Linux and do not substitute for native execution.

## Review and original failures

Focused independent design and source/security review preserved custody, lock ordering, borrowed-name lifetimes, the shared entry budget and preflight-before-unlink. Two review findings were corrected: the lifetime test initially created a Weak before exclusive retirement, and the helper CI build initially retained compiler JSON only in memory. Weak creation now follows successful retirement; the production Arc::get_mut guard is unchanged. Compiler JSON is saved before testing the original exit status and rendered errors remain visible.

The first Wick core run passed 22 controls but failed the new Weak lifetime fixture; its raw log is retained. The corrected intermediate run passed 22 controls, and the final expanded suite passes 23. Strict helper Clippy initially rejected a manual integer divisibility expression; it was changed to is_multiple_of. That initial raw Clippy log was overwritten during exploration, so this packet does not claim complete raw retention for that one failure. Source and final compiler outcomes remain distinct.

An attempted all-manifest offline metadata pass stopped at an uncached historical gateway ahash dependency before tests. Validation then used locked relevant metadata plus an exact ten-lock comparison without changing registry versions. Two incomplete source exports omitted historical compile-time JSON/artifact fixtures; raw compiler logs are retained and all five required fixtures were supplied before final source hashing and strict checks. No test assertion, deadline, authority or security check was relaxed.

## Retained artifacts

The raw Wick evidence archive has SHA256 `1c87e0d85a8bd9e1a9b990152e422d63dd7421dff694adca75156b343b770cf7`. Raw native ZIPs, member hashes and embedded checksum validation are retained. The packet keeps compact summaries/identities rather than payload stores or build trees. Cleanup records will name only owned disposable exports after their evidence is verified and published; shared Cargo, user files and prior historical evidence remain intact.

## Native populated allocation rows

OS | Architecture | Requested Rust peak bytes | Fixture wall ns | Process CPU ns
--- | --- | ---: | ---: | ---:
macOS | ARM64 | 2298168 | 734888792 | 995819000
macOS | X64 | 2298168 | 1301976674 | 1587380000
Linux | ARM64 | 2273769 | 291128207 | 390197520
Linux | X64 | 2273769 | 338163081 | 429036917

CPU spans may overlap across the fixture's two workers; they are not send-only time or physical energy.
