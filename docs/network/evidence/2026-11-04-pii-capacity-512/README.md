# Owned PII scanner: qualified 512 KiB NFA control

2026-11-04 component checkpoint for #43/#49. The selected owned NFA control is **524,288 bytes**, enforced by the existing private Core builder and accepted by Hostd's exact control predicate. DFA cache remains262,144 bytes, nesting64, one search at a time on the original receive thread. Canonical rules/order/Unicode/validators, legacy constructors and all other model multipliers/fixed terms are unchanged. The experiment changed no primary checkout or SDK publication. Independent source/evidence review has since passed; the separate producer composition is consuming this delta in an explicitly intermediate context. Final integrated graph and native qualification remain required.

This tightens an enforced engine control and its conservative declared workspace; it does NOT demonstrate lower actual allocation/RSS, faster networking, two-VM fit or FIPS qualification. The regex controls are approximate engine construction limits, not whole-process allocator limits.

## Preregistered candidates and actual execution

The fixed candidate plan is [experiment-plan.md](experiment-plan.md) (original retained plan SHA83697204aeacc934878007aa8b441e55ebe22cf0cb1fa9c5763ba339651f0c3a). One actual Core private-builder test ran each candidate in a protected release object:

| NFA cap | Actual result |
|---|---|
|512KiB|325 ordered selections construct; exact legacy masked bytes AND rule-name tuple equality;54 representative first/repeat searches;5 default/mode cases pass.|
|256KiB|REJECTED at selection5/order[0,1,2,3,4], after4 successful constructor/equality cases.|
|128KiB|REJECTED at selection3/order[0,1,2], after2 successful constructor/equality cases.|

Only512KiB is selected. This is not975 passing constructors or a claim that256/128 are supported. The test uses the existing actual private build_owned method and actual RegexSetBuilder/RegexBuilder controls, not a public mutable-controls API or copied regex implementation. One protected unit-test entry passes; its583 supplied inputs and binary match before/after. Its source still had the original1MiB public default, with actual candidate controls supplied only inside the private test.

Before product edits, the genuine actual AuxiliaryLedger test failed with WouldBlock:80MiB=83,886,080B of prepaid metadata cannot admit the old96,845,824B canonical collector request. After the minimal source-coupled512KiB change, the SAME test passes at67,485,696B, then still refuses a byte past the actual remainder and a duplicate Collector claim. Both arms use the same release/LTO configuration;5,235 source inputs and protected binaries verify. This is actual component subdivision, not a new quota, VM admission or two-guest witness.

The final protected Hostd binary passes8 AuxiliaryLedger test entries (including that new case) and11 owned-collector entries. The standalone new-case invocation overlaps the8, and iteration counts are not added as unique tests. Three thin public-Core API regressions against the NEW actual Core rlib pass: exact selected controls/unchanged other fields; default/disabled/detect/redact/refuse byte/name/match behavior; unknown/duplicate/oversize policy refusal with static errors. These use public original APIs and introduce no quota or controls constructor. Scoped release Clippy -Dclippy::all for Core(host-state library) and Hostd library both exit0 with zero emitted compiler warnings. This is not an all-targets or full native/platform qualification.

## One selected public requested-allocation witness

The existing reviewed Rust/System allocator witness was reused with only its Core source path changed. Original generator, all325 nonempty ordered policies,32KiB dense body, allocator counters and two-policy/three-input first+8-repeat sample are unchanged. Exactly ONE selected public325-constructor run and ONE sample run completed. Every constructor's postdrop live delta equals0; both sample policies return99,980->99,980 requested live bytes after all observed outputs/scanners are dropped. Runtime queries report NFA524288/DFA262144/concurrency1. New protected probe SHAf739f861f71730ecbbbde710e3bdce00f56c5840752408f088c9d7a84e4c5b19;597 source inputs verify.

| Maximum scanner term | Historical1MiB | Selected512KiB |
|---|---:|---:|
|Declared scanner workspace, six engines|69,451,776B|40,091,648B|
|Measured constructor requested increment|2,254,054B|2,254,054B|
|Measured conservative realloc overlap|2,391,826B|2,391,826B|
|Measured retained compiled allocation|1,938,125B|1,938,125B|
|Measured sample first-search peak increment|637,807B|637,807B|
|Measured sample repeated-search peak increment|65,623B|65,623B|

Thus the declaration reduces28MiB per maximum scanner while the observed actual peaks remain unchanged. It is not28MiB of measured heap reclaimed. Existing objects are not repriced/refunded; the original same-pool permit remains held throughout its actual lifetime.

The historical/new325 constructor median instrumented CPU is1.667529/1.639907ms and wall1.667628/1.640054ms. Median destructor CPU increases32.802->34.545us. The two-policy sample medians and every raw row remain in comparison.json; historical/shared-host timings are diagnostic, not randomized paired speed/energy results. No transfer or WAN ran here; no send-only time, wire-byte or network acceleration claim follows.

## Source and evidence boundaries

The initial copied workspace lock was stale against already-pinned unbuilt pure workspace dependencies; the first --locked build stopped before compilation. The offline-resolution diagnosis changed no registry versions or Core/Contract/regex versions. The Hostd RED/GREEN/strict context restores the original canonical pure1025 override and lock. This is an intermediate one-Core dependency context, NOT acceptance transferred to SDK9a or the newer upstream integration base.

The public witness's successful build was followed by one wrapper copy-path failure BEFORE any measurement; its exit1 is retained. Copying the already-built actual binary from its correct owned target and continuing postbuild produced the one measured run. No failed measurement was silently rerun. Old baseline evidence remains in the original owned-PII requested-allocation packet; no historical file is retagged512KiB.

Current production delta is two Core literals (query plus its existing assertion), a cfg(test) private candidate module, and ONE Hostd exact-NFA guard literal. The entire retained owned-reference-only.rs body is custody reference only: DO NOT overwrite another agent's newer collector/producer changes with it. Apply only the guard hunk. The new AuxiliaryLedger test is a focused appended test; no ledger implementation changes are made.

## Capacity limits and next milestone

Keep the tenant ceiling256MiB. The unchanged source-model arithmetic at512KiB gives collector metadata67,485,696B, helper52MiB plus32MiB usable stacks, each collector6MiB usable stacks, each controller8MiB main, and8,349,628B per supplied Small local pair.

| Partial known subtotal | One endpoint + supplied local pair | Two endpoints + two pairs |
|---|---:|---:|
|Known terms above|178,595,772B|269,111,160B|
|Remainder under268,435,456B|89,839,684B|−675,704B|

These are PARTIAL disjoint source declarations. Native signal/guard supplements, TLS/carrier, codec readers/relays, Root bookkeeping, bootstrap and persistent-session terms still need explicit source-coupled qualification; no missing term is zero. One installed guest+host peer may be the first milestone only after its complete worksheet and actual native execution. It cannot replace the final two-concurrent-endpoint, sustained/fairness, platform/WAN or upstream gates. Further helper/control optimization needs a separate measured plan; no other ceiling is raised or multiplier relaxed here.

All original source snapshots, normal/instrumented binaries and Cargo products remain on Wick. The compact archive excludes executables and payload/build stores but includes their hashes, raw failures/results, exact incremental sources, commands and source maps. No local raw VM/cache/dataset was created.

## Publication and licenses

[Independent Code/Rust/Security/privacy review](independent-review.md) verifies the source/evidence delta and preserves every broader release exclusion. The source snapshots and incremental patch here derive from MVM and preserve its Apache-2.0 license; see [LICENSE.mvm](LICENSE.mvm). SpiralCore report text remains under the repository documentation license. The archived Hostd full body is a custody reference only: apply the exact guard hunk, never replace newer producer changes.

The archive is 1,872,670 bytes, SHA-256 `3492f095fe1432ebaa28d8e894f6fd4479327f262c633553201903847a0ccf39`. It contains 124 members, including 123 checksummed records. Extract only into an empty inspection directory; extraction does not execute stored code or commands. It retains the intermediate source/lock context and all actual failures. This publication does not assert an independently buildable final MVM SDK or transfer acceptance to another source graph.
