# Original-config parent binding source review

HOLD before integration qualification. Narrow independent Code/Rust/Security
source review,2026-11-04; no build/test or product edit performed.

Reviewed installed_parent.rs SHA-256
`a8a90b84eaacf1b7a0dee58c4579a8ea9c5220649bbaef3bae1d29c1927c3a7b`
and config_tests.rs
`af62734651484c37f4a0ebf85a8c783c77e9588a921550c26b619e906ca71a0f`.
Compared the actual untracked source with published0bd directly: these product
files are intentionally absent from the main documentation branch's index, so
`git diff0bd` alone represents deletion and is not the implementation delta.

One lifecycle blocker: bind_original_admission commits admitted_plan and
admitted_session before fallible bind_original_registration. That latter method
may reject unsupported proxies/services/capabilities, identity/path bounds or a
32KiB metadata reservation. A known-preflight refusal therefore leaves
admitted_plan=Some and registration=None. A corrected bind is then rejected as
already bound, and seal_unused refuses completion even though no helper or VM
was invoked. This can strand the original reservation under a capacity/policy
failure. The three authored tests cover earlier identity/body/tenant refusals,
so they do not exercise this ordering gap. Finding sent to Root immediately.

Minimal shape: prepare and validate the bounded registration candidate and
reservation outside the lifecycle mutex; install admission/session/registration
together in one short commit after the fresh original-owner checks. Losing
candidate/credit destruction must occur outside the mutex. Add a focused
unsupported-proxy or capacity-refusal case proving unchanged Root lifecycle and
usage, followed by successful corrected binding or unused cancellation.

The reviewed authority direction is otherwise sound for this bounded seam:
the legacy binding-only API retains workload==VM; generated-name binding requires
the config's exact original owner Arc, tenant/name/backend/shape, and repeats
the complete opaque admitted-body continuity check before handing back the SAME
grant. A public config/plan label or equal projection is insufficient. The new
source tests use actual synthesis/admission and actual foreign owners; they do
not claim native execution. Product GREEN, exact composed dependency closure and
the identified ordering correction remain pending.

## Separate three-SDK-seam source check

PASS for these additive API seams only; the Root ordering HOLD above remains.
Read the exact isolated overlay and compared all three with published MVMf708:

- Core host_lifetime.rs `2f6539a9a86c9fdfd130ff05b36f80ae1a6e42dec87d2340beac5d40654c6e8c`:
  only the eight-line erased-owner accessor is added. The typed downcast and
  Arc pointer comparison export no owner, FD, reservation or new permission;
  equal projected values from another actual owner do not suffice.
- Hostd host_lifetime_parent.rs `2b8d3a4e2fc7928123d057dd495b64d2f52b932e7cb4854d8cf12c2df3cc0f3a`:
  default bind_config freshly verifies the original config/anchor and delegates
  to the prior bind contract. Canonical bind_current still calls bind_admitted
  on the actual opaque plan first and bind_parent_permit last. The None legacy
  path remains unchanged. Root's override can add stricter original config
  checks without substituting a DTO for opaque admission.
- Client launch/mod.rs `0ef00e4de33be59ea631d1785cf5f31cc55e9616b7f06bfb0c26d4ff5efbb4ca`:
  nonconsuming install_owned_host_agent delegates to the SAME original backend
  and retains its underlying install-once/no-active-launch checks. The old
  consuming builder delegates through it with the same error mapping; no
  backend reconstruction, registrar attachment or new authority constructor
  appears.

Unrelated Core native-stack/peer hunks were excluded from this comparison.
These source checks do not represent execution or combined-graph qualification;
the exact composed build/tests and Root's refusal-atomicity regression remain
separate gates. No additional blocker in the three SDK deltas.

## Refusal-atomicity successor source recheck

PASS for the minimal correction, pending exact execution qualification. Root
reports a separately retained actual3PASS/1FAIL regression at
binding-preflight-red-corrected; this review does not relabel its build
prerequisites or substitute source reasoning for the pending GREEN.

Reviewed installed_parent.rs
`eb2dc51c8f1e3f927868d5d4b6237a8a2303cb8730027917a0b27dc8f4a7c308`,
host_agent.rs
`6f331e17370e6594b0c96cb66fe1ed6aa26bf4822070b53196e190ce8b08041d`,
config_tests.rs
`6b374dfe7f68f2052d67886b707657d5db0a724b2adcafdedf229ff6788d0b5c`.
prepare_original_registration now returns a fully validated, reserved32KiB
candidate without touching lifecycle state. Admission/session/candidate are
installed together after the short lock verifies an open and still-unbound
original grant. The candidate local is declared before the mutex guard, so a
losing/failed commit drops the guard before destroying candidate credit; no
filesystem, callback or candidate cleanup is introduced under that mutex.
Legacy binding-only behavior and all previously reviewed authority checks remain.

The focused source regression supplies an unsupported proxy to the SAME actual
opaque plan and original config, checks admission/session/registration and
resource usage remain unchanged, then removes that proxy and binds the SAME
opaque plan successfully. It directly exercises the previously identified gap.
No remaining source blocker in this bounded correction; exact GREEN, source
readbacks and combined-graph qualification remain execution/delivery gates.
