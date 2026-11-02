# Independent measurement review

Reviewer: `mvm_tests`, cross-author arithmetic and interpretation review. Verdict:
**PASS for the recorded descriptive results; no arithmetic mismatch found.**
No builds, network trials, production changes or performance reruns were made for
this review. Source, executable and full artifact custody are separately checked
by the parent reviewer.

## Reviewed evidence and method

Hosted run `41465450384`, artifact `13153845111`, measured source
`30792f17d0a596c4344e4780ebd4b8f455e3bedb`. Independently read the 18 client,
18 server and 18 capture receipts from the extracted campaign under
`/Users/casey.allard/spiralcore/local/cpu-conscious-20261102/candidate-evidence/campaign`.
Used separate jq extraction and JavaScript arithmetic, without invoking the
production Rust analyzer.

The raw joins reconcile all 18 sessions and 336 operations, including 288 object
deliveries and 48 auxiliary operations. Receiver batches join by transfer ID;
sender request counts, accepted receiver batches, actual request/response channel
bytes, object identities and lengths, namespace, successful status and the
12 unique reopened exact files per session agree. No unmatched numerical row was
found. Recomputed 324 paid-group cumulative rows and 324 connection-prefix rows,
charging checkpoint and schema transfers, codec preparation and connection setup
at their respective accounting boundaries. Finish and whole-session bookkeeping
remain in the whole-session measurements rather than being silently assigned to
an object. All these rows match `analysis.json`.

All 234 signed policy opportunity rows reconcile independently: 120 operation
pairs, 108 paid-group cumulative pairs and six complete-session pairs. Every
CPU-conscious-minus-exhaustive logical-byte difference is **zero** in this dose.
The 48 required mechanism rows (eight per link/trial pair) preserve the selected
mode, publication path and logical transfer receipt. These checks establish
receipt and length equality; the network receipts are not a new byte-for-byte
canonical frame witness. Core tests separately cover canonical encoding and exact
reconstruction.

Reviewed identities:

| Item | SHA-256 |
|---|---|
| `analysis.json` | `42797d50a77329c0794e7e285bbdf7ec282eb4be6c06ae33ae08427bd5d5718a` |
| Original extracted `artifact.sha256` manifest | `0603c434d6aa8c1c41d1fd8c7e1215e0b5ae2fcfeea51c4826f6b21eabc6eeb0` |
| Original extracted `source.json` | `dbadbb3a9943c5bc7b4a079978275886708d77c2303946fd98a73160eb954330` |
| Retained `compact-files.sha256` | `207c4ccf03a2248f5766df1768a23c640aa803487c402f0d4c59a226ce913d97` |
| Retained `compact-receipts.tar.gz` | `a57fb382b8f7bddea342495e963faba4b1a69b1d83df8a6e9b9cc81f7612b101` |

## Recomputed outcomes

Displayed times and bytes below are descriptive medians of three trials.
Percentage changes are medians of **same-trial ratios**, not ratios of displayed
medians; negative changes mean less cost. This distinction matters where the
ordering of separate medians differs from paired comparisons.

| Link | Arm | Captured IP bytes | Summed process CPU, us | Client elapsed, us |
|---|---|---:|---:|---:|
| LAN | Exhaustive | 33,078,569 | 1,356,389 | 1,469,797 |
| LAN | CPU-conscious | 33,082,117 | 1,373,229 | 1,507,931 |
| LAN | Whole zstd | 56,779,205 | 1,461,381 | 1,442,637 |
| 80 ms / 10 Mbps | Exhaustive | 33,946,691 | 1,954,898 | 31,037,964 |
| 80 ms / 10 Mbps | CPU-conscious | 33,943,850 | 1,924,081 | 31,060,953 |
| 80 ms / 10 Mbps | Whole zstd | 58,149,609 | 2,471,298 | 49,862,399 |

CPU-conscious versus exhaustive changes complete-session process CPU by
**-0.1809% LAN / -1.1752% shaped** and elapsed time by
**-0.5928% LAN / +0.0454% shaped**. Its IP-byte differences are only
+0.00446% LAN / -0.00756% shaped while logical bytes are identical; these tiny
capture differences do not establish a new encoding-byte saving.

Versus Whole, CPU-conscious uses **41.7387% fewer captured IP bytes on LAN** and
**41.6271% fewer on the shaped link**. Process CPU changes are -6.0321% and
-21.8255%, respectively. Elapsed time is **1.4147% worse on LAN** and
**37.7067% lower on the shaped link**. Most of this advantage belongs to the
existing combined mechanisms, not to the new policy: the policy changes no
logical bytes in this campaign. The new fit/drift controls also change the session
mix from prior six-delivery campaigns, so cross-campaign totals are not a
controlled before/after comparison.

Unrelated inputs have exact lengths **1,047,202** and **14,851,384** bytes. All
three arms transfer respectively **1,049,988** and **14,883,962 logical bytes**.

| Link / input | CPU-conscious sender CPU, us | CPU-conscious elapsed, us | Sender CPU vs exhaustive | Elapsed vs exhaustive | Sender CPU vs Whole | Elapsed vs Whole |
|---|---:|---:|---:|---:|---:|---:|
| LAN / small | 9,178 | 10,846 | -20.4197% | -20.5000% | +0.1428% | +1.5447% |
| LAN / large | 130,488 | 197,112 | -18.7031% | -14.3053% | +3.6384% | +5.0923% |
| Shaped / small | 18,994 | 1,243,041 | -11.4871% | -0.1534% | +6.3416% | +15.7772% |
| Shaped / large | 208,266 | 12,907,807 | -16.7747% | -0.1444% | +5.1811% | +0.1057% |

The large shaped case reduces sender-plus-receiver-handler stage CPU by
15.0992% versus exhaustive. This is a nested stage measure, not an additional
quantity to add to process CPU. Three observations have visible variation: for
example, large shaped sender CPU ratios against Whole are approximately
0.7507, 1.0518 and 1.0867. These results do not support a tail or confidence claim.

## Skips, fitting controls and remaining costs

There are **48 policy-skip receipts**, all with zero useful template records:
12 each for generated, residual, unrelated and opaque-edited deliveries. Only
the **12 unrelated cases** replace an exhaustive candidate that was actually
evaluated. The other **36** replace an existing proven SchemaZstd prune. Therefore
48 skip receipts must not be described as 48 avoided full schema compressions.
Both exact-repeat families bypass selection, and neither schema-fit nor
schema-drift receives a policy skip.

Fit/drift costs remain material. For the large shaped schema-fit case, the policy
uses 1.1231% fewer logical bytes than Whole but 47.4268% more sender CPU and
0.5636% more elapsed time. Large shaped drift saves 0.6874% logical bytes but
costs 47.7465% more sender CPU and 0.9018% more elapsed time. Against exhaustive,
their sender CPU changes are +6.6773% and +4.2059%, with elapsed changes +0.2236%
and +0.2359%; neither case takes the new skip branch. Large LAN fitting and drift
cases also remain slower than Whole (elapsed +50.7911% and +51.7933%). These
measured losses must remain visible rather than being discarded as outliers or
attributed causally to a branch they do not execute.

No extra logical bytes were observed across this finite dose. This does not prove
that a zero-template intermediate can never compress better: CPU-conscious is an
explicit opt-in policy and exhaustive selection remains the default. The gate
must continue reporting signed opportunity costs on future workloads.

The two endpoints share a hosted Linux runner with a calibrated namespace link.
This is real QUIC transfer through an emulated WAN profile, **not an Internet-WAN
or MVM measurement**. Captured IP bytes are complete-session totals and cannot be
assigned to individual objects. Per-operation CPU/wall includes the stated
loading, hashing, selection and publication boundaries; CPU cannot be subtracted
from elapsed time to invent a send-only time. Neither process CPU nor these
measurements establish physical energy savings. No finding requires a source or
analysis correction.

## Final report check

Reviewed `docs/network/cpu-conscious-results-2026-11-02.md`, SHA-256
`b2cb10991a78d24c60b259d04d52ccb1bf38474ddbf3a7d6a064c66d6b0bff5e`.
Its tables, matched comparisons, 2,289,396,384 requested delivery bytes, skip
qualification and measured-scope limits agree with the independent calculations.
Also checked the reported non-target losses directly: large shaped residual
sender CPU +8.5484%, opaque-edited sender CPU +20.7833%, generated elapsed
+2.0225%, opaque-edited elapsed +2.4816%, and the two repeat elapsed changes
+0.7483% / +0.1141% versus exhaustive. No unfavorable observation was removed.

The client-elapsed/readback wording is supported by service order: the server
drops and reopens the store, verifies committed objects and expected files, then
calls `acknowledge_finish`; the client's `finish` requires that explicit end ACK
and FIN before returning. Its whole-session wall snapshot therefore includes
waiting for receiver readback. Receiver process CPU includes readback directly;
it is not added again from the nested timer. No report correction is required.
