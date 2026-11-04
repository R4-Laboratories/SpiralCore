# FlowMux component checkpoint

Latest frozen continuation: [component31](component31.md), with31
release tests and strict Clippy passing against the current shared dependency
bytes. It uses mutual TLS1.3 RPK, explicit scope enrollment and the120-byte
full-facts header. The preceding [component30](component30.md) is immutable at a521ac73910ee9c7f421a166a639bdc62024e499. The following component20 text and byte table are retained
historical evidence for the earlier72-byte prototype, not current totals.


Issue [44](https://github.com/R4-Laboratories/SpiralCore/issues/44) remains open.
This packet records the new standalone shared-core TCP payload component, not
canonical MVM activation, UDP, native VM/WAN qualification or a performance win.
See the [reviewed contract](../../../api/mvm-flowmux-v1.md), [bounded plan](plan.md)
and [exact proposed canonical seam](canonical-seam.md).

Actual Wick qualification: twenty socket tests passed (seven transport tests,
four object-v1 tests, eight duplex tests and one standard-stream test; Runtime
and Combined both run inside one object-v1 test). No tests
ignored. Library-unit and doc-test scopes each contain zero tests. Strict
all-target Clippy passed. All builds used two jobs in the task-owned target;
no VM, host-network change or Mac target/build was used. The owned Wick target
was 247 MiB after the first debug component and 464 MiB after release qualification.

The TCP component preserves exact bytes, independent reverse traffic after
half-close, Raw/whole-zstd selection, size/refusal/truncation, original deadline,
reset/cancel and actual socket counters. ObjectChannel carries canonical core
frames and uses actual ObjectSender/ObjectEndpoint Warm, Runtime and Combined
paths, explicit artifact publication and verified Published responses. Wrong-ID
Published fences reentry. Context namespace/epochs are checked before I/O.
Capabilities are trusted-pinned caller input and grant no admission/authentication.
Duplex provides one owned reader worker, one pending response, four decoded
blocks plus one pending, backpressure before the canonical Published response,
role/index transfer identities and distinct artifact/application delivery.
ApplicationHalfClose retains physical TCP response handling for reverse traffic.
The standard Read/Write facade preserves short reads, clones and repeated EOF;
the last clone cancels and joins the exact worker. The32-event guard was exercised
with actual interleaved canonical requests while a local operation awaited its
response; the33rd request refused with Limit. These are sockets, not booted VMs.

The successful `component-counts.txt` receipt records these actual mapping-byte
totals. Paired counters are equal; these include application mapping headers and
canonical object replies, but exclude TCP/IP and MVM carrier overhead.

| Fixture | Sender TX | Sender RX | Peer TX | Peer RX |
| --- | ---: | ---: | ---: | ---: |
| TCP whole baseline (4096 out, 3072 back, both FINs) | 215 | 215 | 215 | 215 |
| Warm (8192-byte base plus 8192-byte edited target, both FINs) | 9202 | 940 | 940 | 9202 |
| Runtime (checkpoint plus target, both FINs) | 1534 | 1076 | 1076 | 1534 |
| Combined (checkpoint, schema, target, both FINs) | 2002 | 1510 | 1510 | 2002 |
| Duplex Combined (checkpoint, schema, two logical sends of the same runtime target, both application FINs) | 2176 | 1670 | 1670 | 2176 |

These are different functional workloads, not matched performance arms. Setup,
sender/receiver CPU, lifecycle, wall time, RSS, native transport, shaped network
and WAN measurements have not been qualified here. Core candidate costs keep
their original cost model; no CPU/energy/savings claim follows from these rows.

Failures are retained under validation/: initial missing-library API RED;
missing ObjectChannel/ServeEvent API RED; a negative fixture's Protocol versus
InvalidBinding expectation correction; a runtime fixture exceeding 16 KiB
corrected by reducing trace steps rather than widening the cap; two Clippy test
loop style failures; and the actual immutable-namespace semantic RED followed
by GREEN. Do not relabel missing APIs or fixture mistakes as behavior regressions.
Further actual failures retained: missing Duplex/Role and MappedStream APIs;
first duplex normal-close race (Truncated at join); standard-stream repeated EOF
and write-after-half-close cancelling reverse traffic, each followed by focused
GREEN. The four-block backpressure test holds the delivery queue full until the
workers finish; it does not infer receiver non-delivery merely from sender error.
`source-sha256.txt` pins the initial component snapshot; later named source
manifests pin the successive copied crate/dependency snapshots. Unrelated
worktree edits are outside those snapshots.

Required remaining gates: independent code/security review, exact approved
MVM hook plus trusted signed cooperating-peer selection and actual TLS peer
authentication, canonical rate/window/transform/audit/reset tests,
separate UDP/ingress, native supported-platform/backend and matched performance
qualification, real WAN and scale acceptance. Full issue44/Ari delivery is open.
