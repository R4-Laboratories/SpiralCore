# Unused original host-lifetime preparation

Narrow architect decision, 2026-11-04, for the existing resident launch
transaction. Supplements [parent retirement](installed-parent-retirement-v1.md).
This is positive evidence about one **unconverted preparation**, not native VM
retirement, a permission grant, or general proof that an attempted launch had
no effects.

## Canonical additive API

Add `LocalBackend::try_launch_with_host_lifetime`, with the same request/lifetime
inputs as the existing method, returning `Result<LaunchOutcome,
HostLifetimeLaunchFailure>`. The private-field failure owns the original
`MvmError` and `Option<UnusedHostLifetimePreparation>`.
Its consuming `into_parts` returns those two values; consuming `into_error`
preserves the original error for the legacy wrapper. The existing
`launch_with_host_lifetime` delegates and maps into_error, retaining its signature,
error meaning and conservative uncertain-owner behavior.

The unused wrapper has private fields and owns the **same unconverted**
PreparedHostLifetime. It exposes only read-only binding,
`retains_parent_owner<T>(&Arc<T>) -> bool`, and consuming `into_prepared` for
retry. It is not Clone, serializable or constructible from a DTO. There is no
FD, raw owner, public downcast or arbitrary success constructor.

Only the four existing pure refusal branches before `into_anchor` may return
Some unused preparation:

1. Request is not explicitly named transient mode.
2. Explicit requested backend differs from this original LocalBackend.
3. The installed parent provider's static registry lookup is absent.
4. This original backend does not support installed native lifetime.

Those branches directly move the intact preparation into the private wrapper.
No generic error mapper, elapsed timeout, cancellation, panic, formatted error,
or inferred absence may create it. Every error after conversion to an anchor
returns None, including later shape errors that happen before native allocation.
Dropping the async operation without receiving a certificate remains uncertain
for the armed resident owner. Future work added before conversion must not be
silently declared pure by proximity to this boundary.

## Internal identity without ownership escape

Core's private erased parent owner may become `Box<dyn Any + Send + Sync>`;
the existing bind input already requires 'static. The internal comparison
downcasts only to `Arc<T>` and uses `Arc::ptr_eq`. It returns a Boolean and no
reference to the erased value. The same private erasure may move into the anchor;
it creates no public owner getter or new wire field. This check is concrete
allocation identity, not a new admission authority or a test of copied fields.

The Root wrapper passes its private original Arc<VmGrant> type, so a preparation
holding another Arc type/allocation cannot cancel its reservation even when all
binding fields were copied exactly.

## Resident cancellation

Add `ParentLaunchOwner::cancel_unused(&mut self, unused: &UnusedHostLifetimePreparation)`.
It requires full original binding and identical original grant Arc, then the
cleanup-safe retained ledger identity. Under the SAME grant lifecycle mutex,
require no bound admitted plan and endpoint state NeverIssued; only then seal
Retired and disarm uncertain Drop. A previous admitted plan refuses even if this
particular call returned an unused preparation. Preparing/Issued/Uncertain or
CleanupComplete is not NeverIssued and cannot pass this unused path.

Normal admission close permits cancellation; no new launch permission or
filesystem query is needed. Poison, original accounting mismatch/fence and every
identity/state refusal leave the owner armed. No caller Boolean, alternative
pool, marker or error string can reset it.

The borrowed certificate retains its own original grant Arc. Cancellation does
not forcibly refund it: actual charge remains until the certificate, completed
parent owner and all other original owners drop. Calling into_prepared after
cancellation does not restore permission; Retired must continue to deny later
canonical parent admission/native activation. The supported retry path consumes
the certificate **before** cancellation and reuses the same preparation/grant;
it must not allocate another identity or silently replenish resources.

## Focused TDD boundary

Record API-absence RED before product. Qualify the four actual preflight branches
and same/foreign Arc identity; preserve install-once registry semantics by using
separate process cases where needed, never a reset hook. A later post-conversion
shape refusal must return None. Original cancellation/unwind behavior remains
conservative.

For Root cancellation, test exact unadmitted identity, normal close, retained
charge while the certificate lives, final-last-owner refund, different owner
despite copied binding, admitted-plan refusal and non-NeverIssued refusal. No
test-only public receipt constructor is needed. These are preparation/cancellation
component gates, not a guest boot, successful native retirement, or positive
failed-startup cleanup witness.
