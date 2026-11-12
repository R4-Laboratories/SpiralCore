# Intel macOS native timing analysis — 2026-11-10

Product source remains unchanged from 661c74e7b223736e440e1e41c3cd70603d050d80 in FlowMux, transfer and Windows storage paths. Current branch head is 1ae9fa35842c86e22b4a66c6fdf197e28c6136ca. Both hosted artifacts record actual PR-merge checkout 073f94ef35bb68e77a349dd3c843246237e8efbd, tree c0ab4c814f6012b24ea562359dfeff3dae0c5297; that tree exactly equals the branch-head tree. This distinction must be retained in an execution receipt.

## Actual executed rows

| Execution | Outcome | Lifecycle wall | Process CPU including cleanup | Progress |
|---|---|---:|---:|---|
| Prior feature-off, run42922116102/job130199439671, source661 | FAILED operation deadline | 59.840499s to snapshot; 60.17s harness including unwind | 35.786480s to snapshot | writeACK1791/1790; exactread1791/1791 |
| Feature-on, run42925924991/job130212249425 | PASS | 70.160776s | 39.570219s | ACK/read4096 each direction |
| Feature-off, run42925924918/job130212357010 | PASS | 61.437129s | 37.515710s | ACK/read4096 each direction |

The final feature-off job is completed successfully, rather than still running. Its artifact counts verify 66/66 component tests and separately 15/15 focused ordered-cache/Small-resource tests, zero ignored/failed. These are native process component scopes, not WAN or an installed MVM backend.

Both successful sustained runs compare every recovered block: 67,108,864 exact application bytes per direction. TLS left sent5,395,050/received5,395,015, with matching peer counters. Both retain64publications/1,051,712bytes and4032evictions each. Scope/profile/deadlines/counts were not relaxed. These hosted jobs are separate realizations, not randomized matched performance trials or a causal measurement of timing overhead.

## Instrumented observations

| Measurement | Side A mean / max | Side B mean / max |
|---|---:|---:|
| Session condition-variable wait | 0.063524 / 0.462782ms (1214waits) | 0.065374 / 0.718369ms (1176waits) |
| TX condition-variable wait | 0.062171 / 1.483038ms (990waits) | 0.057950 / 0.546524ms (1062waits) |
| Receiver Begin handler | 1.519915 / 39.480304ms | 1.524965 / 31.301895ms |
| Receiver Commit handler | 7.644184 / 88.628826ms | 7.648359 / 76.022419ms |
| Sender Begin response wait | 7.519606 / 88.350065ms | 7.530020 / 77.205197ms |
| Sender Data response wait | 1.129814 / 88.820942ms | 1.122231 / 48.484013ms |
| Sender Commit response wait | 8.162859 / 77.287365ms | 8.141076 / 88.967928ms |

All handler/response rows have4096observations per side. Receiver Commit handler totals31.310580/31.327680s, versus Begin6.225571/6.246256s and Data0.066322/0.065882s. As previously defined, Commit handler includes decode/hash, fresh quota accounting, durable publication/readback and ordered-cache verification. It excludes subsequent worker readback/delivery. These totals are elapsed spans, not independent CPU or send-only time.

Poll actual totals29.869415/29.782616s overlap application/receiver work; maxima74.964959/41.047154ms exceed the roughly20ms maximum requested polls. This demonstrates overshoot in this successful execution, without isolating OS scheduling, kernel/backend activity or another cause. The largest successful TLS session operation is40.393143ms. No recorded actual-elapsed phase maximum approaches the2soperationdeadline. No Session/TX lock timeout/stopped/poison result bucket is nonzero; no overflow flag is set. Session ownership pause buckets are absent, and condition-variable metrics are present.

## Condition waits and fairness

Read-only source review finds the intended release/cancellation ordering retained: original data unlock precedes gate notification; stopped is checked under the gate before the data predicate/wait; original deadline and stopped are rechecked before exposure; scope checks occur outside the gate; cancellation stores stopped then notifies the registered weak TX gate; TLS peek drops the poller before waiting. No new lost-wake, poison-recovery or late-success defect is established by this trace.

The standard mutex/condition variable does not provide FIFO acquisition or a per-waiter starvation bound. Fixed aggregate counters do not qualify tenant/fleet fairness or rare-tail absence. Initial acquisition of the short gate mutex is not a separate Session counter; its critical section has no product I/O or authority callback. These are interpretation limits, not evidence that a gate or fairness bug caused the prior failure.

The preserved feature-off failure has no phase trace. A successful feature-on run on identical product source cannot retrospectively identify or repair its2stail. Keep it as unresolved intermittent native deadline evidence alongside the current passes. No timeout/count change or product patch is justified by these rows alone. Continue the separately authorized canonical MVM implementation; if the native failure recurs during required qualification, retain the existing timing failure snapshot and choose one source-backed change from that actual failure. Do not use repeated uninstrumented attempts to erase it.

## Artifact verification

- Timing artifact13814332033: all3manifest member SHA256 checks passed. ZIP SHA256 e9a37eb7afa3b1496cea64a62a82235267dde33816b069f16d5c57651ca8cd4b.
- Feature-off artifact13814377362: all9manifest member SHA256 checks passed. ZIP SHA256 7487e8b99cde57b4dc779ce9c9752926868b847a9e5a8a5057701c7ed17d8471.
- Matching image20260824.0517.1, macOS26.6.1/25G76, native x86_64-apple-darwin Rust1.97.1; image identity does not imply the same physical hosted runner or load.
- Structured bounded derived metrics: /tmp/spiralcore-macintel-timing-analysis-20261110.json. No product edits, builds, CI dispatches or deadline relaxation were performed for this analysis.
