# Adapter SDK source closure repair

PASS for the two manifest/lock files only,2026-11-04. Reviewed candidate
`7f638428522e70b36a82d68700a6374267ea7e40`, parent
`dee6e52a712c7b3dc12ea09be8bc16d3754b08f7`, tree
`a70457227f304c263faa21ab51ec7814a66baa59`. The actual commit changes exactly
crates/spiralcore-mvm-adapter/Cargo.toml and Cargo.lock; no product/test body,
features, workflow or permissions change is present.

Manifest SHA-256
`32791efa07183df5cec4c2d5a29565252228cc8abf48a80d84452f4a04895e67` changes
only both MVM dependency revisions to the SAME immutable SDK
`bef4146c0de18b22ac392118acfd898a369cc64c`. Shared transfer stays atca5d9ca1,
and existing default-feature/protocol feature selections remain unchanged.
The SDK receipt preserves its three source-reviewed API deltas and explicitly
labels its prior execution as an isolated path-overlay graph.

Lock SHA-256
`715092464e6d21850be0d915a7b3162cbc82415f36501eb9a9ea27ed7e4c29fa`
has exactly the same196(name,version) identities. All other package fields,
dependency lists and registry checksums remain unchanged. Six package source
fields change: MVM Core/Contract/macho-sign now resolve tobef4146c, and the
three existing shared SpiralCore packages acquire their manifest-consistentca5
Git source. This is not literally only an MVM-SHA text substitution: the old
dee6 lock still named7d031c7f while its manifest requestedf708, and those three
SpiralCore entries had path-style/no source. The candidate repairs that stale
lock provenance consistently without a package/version refresh.

This bounded source review supports using the candidate to obtain a single
resolved MVM Core/Contract source in the next locked metadata/build. It does
not itself assert that final metadata has passed, compile/test/Clippy success,
native execution, or acceptance of the full MVM contribution. Those remain
separate exact-graph qualification gates. No blocker in the reviewed pin repair.
