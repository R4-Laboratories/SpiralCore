# Execute opt-in CPU-conscious selection

Approved contract: `docs/specs/2026-11-02-cpu-conscious-design.md`. User approval
covers implementation, measurement and GitHub publication. No further design
approval is required. Keep the scope narrow and retain test/measurement evidence.

1. Transfer owner, medium risk: `schema.rs`, `schema_zstd.rs`,
   `schema_selection.rs`, `session.rs` and focused tests. Add useful-emission
   counts without changing bytes; explicit policy and honest skip receipts;
   preserve default and state/error behavior. Record RED then GREEN and frozen
   parity. Depends on committed contract.
2. Root, medium risk: gateway-core `src/object.rs` and gateway
   `src/object_service.rs`, their `tests/object_combined.rs`. Add explicit
   constructor/configuration mode and receipt serialization. Test configuration
   RED before implementation and both authenticated persistent paths after core.
3. Measurement owner, medium risk: `examples/object_fixtures.rs`,
   `tools/gateway-network/runtime-object-netns.sh`, runtime-analysis crate,
   `.github/workflows/spiralcore-candidate-cpu.yml`, new registration under
   `docs/network/evidence/2026-11-02-cpu-conscious/`. Add a separately identified
   same-source comparison profile; preserve historical outputs. Report actual
   byte differences rather than imposing false policy parity. Freeze exact dose.
4. Independent reviewer, low risk: focused code/security/measurement review.
   Run appropriate crate tests and strict Clippy once changes settle. One local
   Cargo/timing owner at a time. Fix concrete failures, no broad audit expansion.
5. Root, low risk: commit/push frozen implementation, execute the bounded hosted
   campaign, verify source/binary/artifact identities and independently reconcile
   metrics. Record report, ledger, raw receipts and HF fresh-download custody.
   Merge reviewed results, recheck origin and sync checkouts without user-file
   changes. Production default remains Exhaustive regardless of experiment result.
