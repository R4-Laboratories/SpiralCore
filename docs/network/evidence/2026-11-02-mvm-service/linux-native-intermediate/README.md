# Intermediate Linux native results and retained two-guest failure

Source: `2a4a199f0bae224fbe82aa148feb20273bd64d84`. Wick: Ubuntu 24.04 x86-64,
Rust 1.97.1 release, canonical MVM fork `5766754`. This is one trial per arm of
the **1 MiB-tier** mixed workload. Its actual primary trace is 1,047,202 bytes.
The arms transfer the same eight payload objects; Combined also pays for its
checkpoint/schema publications and prepared state. One receiver persists for
the session. These results are unshaped guest-to-host transfers, **not WAN**.

The original records are [whole-object zstd](one-whole-mixed/result.json) and
[Combined CPU-conscious selection](one-combined-cpu/result.json). Actual host
receiver operation/readback events are retained alongside each result. Both
sessions report exact reopened objects. Every Call includes both its admission
receipt and the application's separately admitted Reply: four encrypted
envelopes, counted in the totals below.

| Complete one-guest session | Whole-object zstd | Combined CPU-conscious |
|---|---:|---:|
| Encrypted envelopes + framing, both directions | 7,345,794 B | 4,935,294 B |
| Application Calls | 73 | 61 |
| Reported sender setup + operation CPU | 411,016 us | 283,066 us |
| Reported receiver setup + processing CPU | 300,366 us | 220,839 us |
| Sender setup + operation elapsed | 683,160 us | 505,269 us |
| Complete VM/session lifecycle elapsed | 961,209 us | 834,951 us |
| Independent exact reopen | Passed | Passed |

This single pilot is an observed win on this particular local workload; it is
not a distribution, statistical acceptance or general compression claim.
Elapsed includes computation: do not add CPU time to it. Receiver processing
excludes receive/idle work and independent readback, as documented by the native
witness. Guest CPU, worker CPU and enclosing VMM CPU are distinct accounting
domains; the two endpoint CPU rows do not establish total physical-host energy
or complete VM overhead savings. Full worker and lifecycle fields remain in the
original records. The stream byte totals exclude virtio control/credit headers,
boot assets and the separate workload-exit service.

The Combined record proves active `RuntimeV1` reconstruction for the generated
trace, `SparseV1` for residual/opaque edits, warm exact repeats, and
`SchemaZstdV1` for schema-fit/drift objects. A repeat labelled `Raw` can complete
from the verified receiver object before any raw body is sent; use its actual
counters rather than treating the candidate label as proof of full-body transfer.

The binaries are pinned in [binaries.sha256](binaries.sha256): host
`d3756a173bbd05b69f4960918512517a573b7345aadb68d3592fb55f06fa96ac`,
guest `77628f40dd28050ead21f382983a87e47e963c389f5b71dc211e1f17395ffc70`.
Kernel and Firecracker hashes are included there. The
[source inventory](source-files.sha256) was checked against the isolated remote
tree before launch. Binary entries refer to retained remote files, not binaries
vendored into this report.

The subsequent two-guest whole-mixed session **failed**. Its receiver could not
boot because the current MVM driver requires prepared descriptor custody for a
persistent writable disk. See the original [receiver error](two-whole-mixed-failed/receiver.stderr),
[sender error](two-whole-mixed-failed/sender.stderr), and
[session events](two-whole-mixed-failed/host-events.jsonl). Holding a normal
backing `File` was insufficient; `VmmDriver::boot_custodied` and the canonical
`PreparedWritableDisks` ownership handoff are required. This is distinct from
the earlier clock failure, which this build corrected.

The attempted script stopped on that error; two-guest Combined did not run.
The owned VMMs and temporary standard-path Firecracker link were cleaned up.
The failed receiver image remains in its private Wick run directory, not Git.
A fresh, reviewed custody-corrected source and result directory are required
before claiming successful two-guest qualification. The successful one-guest
records are retained as intermediate evidence rather than pooled with that
future source or advertised as the final MVM delivery.
