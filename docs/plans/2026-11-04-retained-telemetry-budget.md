# Retained telemetry implementation

Execute within the user's approved installed-service goal. Source contract:
`docs/specs/2026-11-04-retained-telemetry-budget.md`.

1. **Pre-effect profile check** — `installed_parent/endpoint.rs`:
   require the TOTAL cache allowance to cover the canonical mandatory records
   cap inside `QualifiedEndpointProfile::claim`, before issuance or disk effects.
   Risk low; no profile activation. Regression first in
   `installed_parent/endpoint_tests.rs`, then update synthetic positive fixtures
   to include their telemetry share. Execute locked release parent tests and
   strict Clippy on Wick; retain RED/GREEN source and executable identities.
2. **Original retained owner** — `installed_parent/endpoint.rs`,
   `installed_parent/host_agent.rs`, `installed_disk.rs`, and the installation
   disk-owner modules: reserve only C-R for disposable cache, retain R in the
   original pool, bind actual state-directory authority before collector effects,
   and keep the retained owner independent of the disposable endpoint lifetime.
   Depends on step 1 and the concrete architect-reviewed catalog/lifecycle
   contract. Risk high; actual file and foreign-owner/replacement tests required.
3. **Canonical cleanup protection and recovery** — canonical MVM runtime
   reconcile and installed-state ownership: add a denial-only retention barrier,
   durable bounded catalog, and exact restart adoption. Preserve volumes/secrets
   session ownership and sticky original-child uncertainty. Risk high; real
   filesystem and crash/restart cases, no fabricated receipts.
4. **Native service acceptance** — original resident and canonical backend:
   one guest transfer, stop and verified disposable retirement with retained
   debt still charged; then admitted durable transfer/retirement, same-supervisor
   completion, two guests and recovery. Depends on coherent upstream source,
   actual collector/controller integration and accepted numeric models.

Record each executed result separately. Step 1 is not a retained-storage or
native release claim. Do not infer zero bytes/CPU or new WAN performance from
these policy and lifecycle tests. Preserve original failures and keep the full
MVM delivery issues open.
