# Registered one-second steady profile: Mac failure investigation

Scope: read-only source/record diagnosis; no product change, deadline increase,
quota increase, replacement successful result or Mac build. Source
`d53cb335ee3884644bfe1a26296923c1288fa540`; executable SHA-256
`dc07da9e488043cd923d38d1f02e84a7731f3601178fa23507a479f16a15fa68`.

## Observed failure

`../steady-8.jsonl` records eight logical endpoints, four concurrent pair workers,
one-second message TTL, 900-second signed authorization, ten aggregate offered
operations per second, and requested 310 seconds. The cell exits 1 and reports
`qualification=FAIL`: **974 completed operations, 1463 exact comparisons, four
admitted-but-incomplete requests, zero compared-byte mismatches and zero quota
refusals**. No full retention-cycle or sustainable 310-second capacity acceptance
follows from this cell. The four incomplete requests are not delivered successes.

| Worker | Completed | First failing operation | Recorded phase | Error |
|---:|---:|---:|---|---|
| 0 | 251 | 251 | receiver_payload | Expired |
| 1 | 253 | 253 | receiver_payload | Expired |
| 2 | 242 | 242 | receiver_payload | Expired |
| 3 | 228 | 228 | caller_reply_admission | Unauthorized |

Each worker records one admitted-but-incomplete request. Completed-operation p99
latencies are 798283/974330/835908/136130 microseconds, respectively; these exclude
the failed operation and cannot establish that operation's exact schedule.

## Source-backed interpretation

`examples/service_capacity.rs::operation_with_behavior` samples each local phase
through `observed_dispatch` and `BrokerContext::with_time`. The latter locks the
shared local dispatch gate, then samples real Unix and monotonic time. It does
not reuse a pre-lock clock. Socket waits occur outside that gate.

At `receiver_payload`, the operation calls `LocalMailboxService::receive`.
`src/service.rs::receive` refuses an otherwise selected accepted entry when
`now.monotonic_ms >= entry.deadline`. Its deadline comes from
`MailboxBinding::deadlines` and the authenticated message/authorization expiry.
This source path supports **expired accepted work being refused at consumption**.
Strict expiry is preserved; acceptance of an ephemeral message does not promise
that application consumption will complete before its deadline.

At `caller_reply_admission`, the operation first receives the Reply and then
enters canonical receipt/application admission, reply observation and polling.
`MailboxBinding::verify` maps canonical `open_verified_envelope` failures to
`Unauthorized`; binding policy, routing, authority and generation checks can also
return that error. A Reply inherits the request expiry, making expiry a plausible
cause after delay, but the record contains no exact issued/expiry/admission trace
or underlying canonical error. **Its unique cause remains unproven.**

The zero new quota counter excludes the instrumented quota-refusal explanation
for these records. It does not prove every possible error class was instrumented
or prove a particular scheduler/clock mechanism. The source inspection found no
basis to weaken time validation or alter the protocol in response to this run.

## Host context and causal limits

`../host-observations.jsonl` records load 9.60/10.47/13.11 near the steady8 start,
load 40.60/22.07/17.16 at 04:33:37 UTC after the recorded 04:32:34 UTC exit, and
5521.38 MiB swap used at 04:35:01 UTC. These are uncontrolled aggregate host
observations, including observations after failure. Other labs/processes were
not changed. They support a resource-contention concern, not per-operation proof
that OS scheduling or memory pressure alone caused any specific refusal.

The evidence does not separate OS descheduling, crypto/local-action duration,
dispatch contention, transport waits or clock changes for the failed operation.
No per-phase clock/lock-wait trace was retained. Do not present the failure as a
proven broker clock bug or dismiss it as solely an OS scheduling event.

## Next controlled qualification

Preserve this failed Mac cell and its original binary identity. Separately
register the **same one-second profile** on Wick at 2/8/32 logical endpoints,
with fresh source/binary/host/features identities, one bounded cell at a time,
the same strict fidelity/counter/completion gates, and all host interference
disclosed. A different physical host is a new qualification result, not a retry
that erases the Mac failure or a cross-host optimization comparison.

Do not run additional native Mac qualification while the observed host pressure
persists. Mark that current native-backend gate unavailable/deferred with this
reason. Keep the broader scale/platform/WAN acceptance open; no TTL relaxation,
raised budget, extra VM allocation or changes to another lab are justified here.
