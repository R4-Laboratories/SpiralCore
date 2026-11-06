# Installed managed resource reservations v1

This is the parent accounting prerequisite for #43/#44/#45/#49. It grants no
boot, route, owner, mailbox or object permission. Canonical MVM admission and
its actual native lifetime remain separate mandatory authorities.

The existing InstalledSupervisor creates exactly one ManagedResourceBudget
under its original InstalledAuthority. There is no public standalone pool
constructor. Cloning a budget/lease shares that same ledger and exclusion owner.
Budget scopes are fixed32-byte tenant accounting tags, never authorization.
The resident host limit is512MiB, guest reservation is at most two1GiB guests,
cache1GiB, staging512MiB and16 owned workers. Per-tenant host resident is256MiB,
cache512MiB and staging256MiB, with the same global two-guest ceiling. Host
resident equals plaintext+codec workspace+carrier+metadata+explicit worker
stack declarations. Guest RAM envelopes are a separate domain and include the
guest's own allocations; guest codec bytes are not also charged as host heap.
These are managed resource declarations, not a bound on all OS/kernel/runtime
RSS. Actual process/guest memory is independently measured for capacity release.

Before allocation/spawn, reserve the entire claim atomically at host/tenant
scopes. Refusal changes no usage or active reservation. All arithmetic is
checked; no transient independent allocation is granted by a concurrency cap.
Use fixed bounded slot tables (128 active reservations/32 stable tenant tags).
No new tenant per message/nonce. Metadata/snapshot storage is bounded by those
tables and remains separate controller bookkeeping, not a fabricated payload
measurement. A read-only snapshot never reconstructs a reservation.

A private serial binds each reservation to its exact slot, claim and original
pool. Clones retain one charge until its last owner drops. ManagedValue<T>
retains the reservation until the contained value has dropped, so an owned
reply cannot release its allocation before serialization/write/cleanup. The
installed parent bridge must retain the same actual lease in native anchors
and endpoint/helper owners. Passing a byte count, bool, profile DTO or an
arbitrary Send+Sync object does not satisfy that bridge.

Reserve checks the original active installation epoch before ledger mutation,
then rechecks admission under the ledger. Close/fence blocks new reservations;
existing charges remain through teardown. Supervisor finish requires zero
trustworthy managed charges as well as the original transport/child/kernel
holder checks. Underflow, poisoned accounting or stale slot identity closes
admission, retains one bounded owner quarantine and blocks successful finish;
there is no counter reset or Drop-based success. No filesystem/process I/O
occurs while the accounting mutex is held.

The API is ManagedClaim, ManagedResourceBudget::reserve(tenant,claim), limits(),
snapshot(), ManagedReservation::check_current(), and attach(value). Only the
controller issues the original budget. Tests prove sibling/tenant/domain
contention, exact rollback, shared clone ownership, close/refund and retirement
exclusion. They do not establish booted fleet capacity or integrated endpoints.
The existing owned stdio/nonce/holder handoff binds projected child limits to
these actual parent leases before provider readiness; that integration remains
required, rather than treating this accounting component as the final service.
