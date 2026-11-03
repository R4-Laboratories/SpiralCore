# Persistent host transport budget v1

Status: proposed internal Rust API and local report contract for the first slice
of [persistent canonical service](../specs/2026-11-03-persistent-mvm-service.md).
This is not an MVM envelope, network handshake, signed capability, destination
receipt or interservice IPC protocol. Its version identifies accounting/report
semantics only. Canonical MVM protocol versions and object-v1 remain unchanged.

## Authority and units

The owning orchestrator constructs exactly one owner and clones it into all
admitted local relay links, including links to separate child processes. The
owner uses bounded route and tenant slots provisioned by trusted configuration;
a received ciphertext cannot choose its charged tenant or ask for control-class
exemption. There is no plaintext, credential, receipt key, object ID or cache
membership in this API/report. The owner does not replace endpoint `HostBudget`.

V1 accounts `links`, `frame_slots`, and `serialized_buffer_bytes`. Byte credit
bounds the capacity owned for serialized ciphertext buffers, not plaintext,
guest heap, kernel socket buffers, decoded-envelope copies, allocator overhead,
whole host RSS or cache storage. Every such omitted domain remains separately
bounded or explicitly unqualified. Limits are reservation ceilings, not traffic
measurements. Existing `StreamCounters` remain actual transmitted/received byte
measurements and retain framing/envelope separation.

## Rust contract

Names below specify the semantic seam; private fields and error wrapping may
follow crate conventions. No public constructor can forge a scope, link or frame
permit. Guards are non-clone; owner clones share the same inner accounting.
All fallible counter mutation is checked and atomic under one short mutex.
Socket I/O, allocation of payload buffers, route decode, waiting and worker
cleanup execute outside that mutex.

```rust
pub struct HostTransportBudget { /* cloneable shared owner */ }
pub struct TransportScope { /* trusted bounded tenant/route slot */ }
pub struct LinkReservation { /* non-Clone owner+scope+incarnation */ }
pub struct FrameReservation { /* non-Clone; bound link, reserved capacity */ }

pub struct TransportLimits {
    pub links: u64,
    pub frame_slots: u64,
    pub serialized_buffer_bytes: u64,
}
pub struct TransportBudgetConfig {
    pub host: TransportLimits,
    pub tenant: TransportLimits,
    pub max_tenants: u32,
    pub max_frame_bytes: u64,
}
pub enum TransportBudgetError {
    InvalidConfig, UnknownScope, Full, Overflow, Fenced, State,
}
impl HostTransportBudget {
    pub fn new(config: TransportBudgetConfig)
        -> Result<Self, TransportBudgetError>;
    // Trusted provisioning only; bounded registered slots, never wire labels.
    pub fn scope(&self, slot: u32)
        -> Result<TransportScope, TransportBudgetError>;
    pub fn reserve_link(&self, scope: &TransportScope)
        -> Result<LinkReservation, TransportBudgetError>;
    pub fn snapshot(&self) -> Result<TransportBudgetSnapshot, TransportBudgetError>;
}
impl LinkReservation {
    pub fn reserve_frame(&self, capacity: u64)
        -> Result<FrameReservation, TransportBudgetError>;
}
```

`reserve_link` charges host and tenant link counts together. A frame guard owns
a reference to its link so dropping the outer link handle cannot uncharge a
link with live buffers. `reserve_frame` reserves one slot plus `capacity` bytes
at both host and tenant scopes; `capacity > max_frame_bytes`, zero, overflow,
unknown owner/scope or fenced state is refused before allocation. `Full` is
explicit; failures leave all usage unchanged. The implementation may reserve
one conservative maximum frame capacity before reading the unchanged frame.
If exact-size reading is used, it must read/check only the existing fixed-size
length prefix before acquiring credit and allocating the payload. There is no
new wire length, flag, tenant label, lease token or control marker.

A buffer wrapper should own the frame guard. Allocate at most its reserved
capacity and hold it through decode/forward/retry ownership. Destroy the actual
buffer before releasing its guard; release exactly once on every error path.
Drop underflow or poisoned accounting fences the owner; it must not clamp usage
to zero and continue admission. No expiry releases buffers still held by a live
operation. Fixed child/VM memory grants, if added, use a separate resource and
release only after confirmed exit/cleanup; they cannot be reported as actual
plaintext/cache measurements.

The immutable per-tenant ceiling plus per-link frame cap bounds hot-tenant
occupancy. It does not guarantee fair scheduling. Refuse new tenant slots after
`max_tenants`; deleting empty slots must not affect endpoint FIFO/replay state.
Use host/tenant high-water counts and separate actual network counters to qualify
a capacity envelope. Reserved endpoint receipt capacity remains inside the
canonical service budget; ciphertext relay reservation does not duplicate it.

## Owner lifecycle

The first `session_with_budget` integration covers only the two-guest keyless
relay and rejects Stage One before filesystem creation or activation. Standalone
Stage One remains available with a null transport-budget report and an explicit
unaccounted host-endpoint buffer coverage label. Its plaintext `StreamTransport`
does not silently inherit the relay's serialized-buffer bound.

V1 has one in-process accounting owner and RAII guards. No daemon/client lease,
PID-based authentication, cross-process shared memory or counter import exists.
Separate worker processes consume transport through links owned by that owner.
A process constructing an independent owner has an independent limit: v1 cannot
claim host-wide exclusivity across orchestrators until an OS-held installation
lock/supervisor is implemented and tested. Reports then use `scope=orchestrator`.

The owner remains alive until all link tasks join and buffers drop. Worker
cleanup/exit is separately observed. A future owner restart may not forget
unresolved old resource holders; it must fence and reconcile them before new
allocation. An advisory lock file that is unlinked/recreated, a stale PID or a
timeout is insufficient evidence. Host-wide ownership, if implemented later,
must use one stable private installation scope and OS-enforced exclusion, with
explicit orphan/crash recovery. A remotely acknowledged mailbox acceptance is
never rolled back by releasing a local transport guard.

## Local report schema

The schema below describes a compact owner-only report. It is intentionally
embedded here: no IPC exists and no generated network DTO is implied. All
integer counters are nonnegative exact integers. The Rust serializer must use
`u64` (or checked conversion) and reject nonfinite/fractional inputs if reports
are decoded. Required relationships are semantic validation in addition to JSON
Schema: usage and high-water never exceed configured corresponding limits;
high-water is at least current usage; summed tenant usage equals host usage;
`serialized_buffer_bytes` is reserved buffer capacity; actual traffic is stored
separately. Failure counters saturate visibly rather than wrap. `host` scope is
allowed only with executed exclusivity/recovery evidence referenced separately.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SpiralCore persistent host transport budget v1",
  "type": "object",
  "additionalProperties": false,
  "required": ["version", "scope", "resource_domain", "limits", "usage", "high_water", "registered_tenants", "refusals", "fenced"],
  "properties": {
    "version": {"const": 1},
    "scope": {"enum": ["orchestrator", "host"]},
    "resource_domain": {"const": "serialized-ciphertext-transport"},
    "limits": {"$ref": "#/$defs/counters"},
    "usage": {"$ref": "#/$defs/counters"},
    "high_water": {"$ref": "#/$defs/counters"},
    "registered_tenants": {"type": "integer", "minimum": 0, "maximum": 4294967295},
    "refusals": {"type": "integer", "minimum": 0, "maximum": 18446744073709551615},
    "fenced": {"type": "boolean"}
  },
  "$defs": {
    "counters": {
      "type": "object",
      "additionalProperties": false,
      "required": ["links", "frame_slots", "serialized_buffer_bytes"],
      "properties": {
        "links": {"type": "integer", "minimum": 0, "maximum": 18446744073709551615},
        "frame_slots": {"type": "integer", "minimum": 0, "maximum": 18446744073709551615},
        "serialized_buffer_bytes": {"type": "integer", "minimum": 0, "maximum": 18446744073709551615}
      }
    }
  }
}
```

Report only aggregate counters to the trusted local operator. If per-tenant
fairness records are needed, use a separate private bounded slot-index report;
do not publish tenant identity or broaden the remote mailbox DTO. Planned
reservation capacity, current ownership, observed high-water and measured
traffic are four separate facts.

## Required boundary tests

- Distinct links for two child-process endpoints share one owner and cannot
  each reserve the host maximum; a separately constructed owner is correctly
  labeled independent and does not establish host-wide exclusion.
- Host and tenant link/frame/byte limits, arithmetic overflow and invalid scope
  refuse without mutation. Control and application frames cannot spoof a scope.
- Credit is acquired before allocating/read payload; failed/partial read,
  malformed route, cancellation and partial write destroy buffers and release
  once. Forwarding does not claim destination acceptance.
- Dropped link with a live frame remains charged; last frame release frees it;
  slow forwarding bounds occupancy while a different tenant progresses.
- No socket I/O or object handler runs while the transport ledger mutex is held.
  Test-only blocked I/O must not block a snapshot or another permitted reservation.
- Snapshot/report high-water and actual stream counters are checked separately;
  accounting does not claim total process memory or guest cache measurement.

Independent code/security review must inspect the exact source after these
focused tests pass. Native backend/WAN and persistent canonical plaintext
capacity remain separate acceptance gates.
