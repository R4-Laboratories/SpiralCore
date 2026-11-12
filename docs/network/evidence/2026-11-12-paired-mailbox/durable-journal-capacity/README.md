# Local journal capacity after expiry and compaction

The destination journal can accept a fresh signed message after its monotonic audit cursor outgrows the configured record quota. In the original implementation the cursor also served as an admission counter. Even with one checkpoint on disk and no retained message identities, new admissions eventually returned `Full`. The installed host sets a limit of 10,000 records and 64 MiB. This regression uses small quotas of three and four records to exercise the boundary directly. [Product change](https://github.com/Casey-allard/mvm/commit/867916bbcf63cd1b2492045edde1bf26999c5e79) · [Reviewable patch](retained-credit.patch)

The local journal now charges each retained identity against its quota. The append path continues to enforce physical record and byte limits. Recovery checks the number of retained identities in the signed checkpoint, its suffix of accepted identities, and the total count of resident files. The anchor still advances monotonically. B keeps a terminal identity through the last millisecond of its signed retention window, and its signed checkpoint may retire that identity afterwards. B refuses an expired original envelope. The independent-anchor tier retains its original admission rule. These changes follow the published local-tier retention contract. [MVM ADR-057](https://github.com/tinylabscom/mvm/blob/543efc7f94aa756c09785b38bdd024b6ba8199c7/specs/adrs/057-mailbox-semantics-and-threat-model.md)

| Linux x86-64 component | Original source plus tests | With the correction |
|---|---:|---:|
| B journal and anchor | 18 passed, 2 failed with `Full` | 20 passed |
| Broader MVM core mailbox unit suite, including the 20 above | — | 59 passed |
| A host service unit tests | — | 7 passed |

**Sender capacity remains 64 distinct retained IDs per source store.** A separate two-authority component used A's host sealer, the Send credential signed by B, and the destination's authenticated opening path. B verified all 64 three-byte bodies and their original IDs. Resetting A's volatile counter halfway through preserved sequences 1–64. At capacity the 65th ID was refused, an existing ID yielded the original encoded bytes, and the same ID with changed content was refused. The fixture ended with 64 intent files, 64 attempt counters and one lock. It used local files and memory; the 4,983-byte encoded retry frame recorded for one invocation did not cross a socket. [Sender control](https://github.com/Casey-allard/mvm/commit/bffdbfde902c0deb3ee3c9eb3037672f723a5d2f) · [Sender settlement decision, #195](https://github.com/R4-Laboratories/SpiralCore/issues/195)

B's suite took 0.02 s wall and 0.01 s user CPU before and after. A's focused control took 0.03 s wall and 0.03 s user CPU. These are whole-test process readings rounded by GNU time. The mailbox transport, guest execution, WAN time and compression performance require separate measurements. The earlier native eight-object run used the unmodified host source. [Measurements and hashes](measurements.json) · [Eight native operations](../paired-durable-native/sustained/README.md)

## Reproduction

Use the contributor source pin in the measurements file. The two failing tests are present at the test-only predecessor `b6de2bde1`. The product-only patch also passes `git apply --check` against Ari's draft `543efc7f`; the executed results below use the contributor source. Run these selectors on Linux with the repository lockfile.

```sh
cargo test --locked -p mvm-core --lib mailbox::journal::anchor_tests:: -- --test-threads=1
cargo test --locked -p mvm-core --lib mailbox:: -- --test-threads=1
cargo test --locked -p mvm-hostd --lib mailbox_service::tests:: -- --test-threads=1 --nocapture
```
