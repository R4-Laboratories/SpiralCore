# Interrupted registration: Linux daemon recovery

A host can lose a caller while replacing a VM's registration. The original
handler may still be running, so forgetting its retirement or refunding its
resource charge would be incorrect. In this test, the next signed deregistration
waited for that handler to finish, settled the original operation, then removed
the requested VM in a **separate** operation. A second VM's registration stayed
in the journal. The tenant remained in the Draining phase.

This is a development checkpoint for [#114](https://github.com/R4-Laboratories/SpiralCore/issues/114).
It exercises Rust daemon code, a physical registration journal and Unix sockets
on Linux with a controlled handler. It starts no guest or signer process.

| Case | Before | After |
| --- | --- | --- |
| Signed deregistration after a cancelled replacement | Returned “original daemon mutation remains busy” before the handler finished; neither operation completed | Original handler joined and both distinct operations completed |
| A two-VM journal was replaced by two copies of one valid record | The request returned success and completed both operations | Refused; exact journal bytes, original charged row, lease and permit retained. No new removal completed |
| Compare complete registrations | A duplicate could substitute for a missing VM | Order is irrelevant; repeated identities and changed record contents refuse |

The first row uses the unchanged original cancellation fixture on its two
binaries. For the inventory comparison, both binaries ran the same three tests:
the valid recovery control, a pure comparison test and the conflicting physical
journal. Before the inventory fix, the control passed and both negatives failed
their assertions. After the fix, all three passed. Rows and file contents were
observed before fixture cleanup. The release-profile test processes on the
repaired binary took **26.82 ms wall / 5.58 ms CPU** for valid recovery and
**24.96 ms / 3.82 ms** for the conflict. Those are single whole-fixture
invocations, including setup, a deliberate wait and teardown. They do not measure
service latency, transfer time or bandwidth. [Commands, binary identities, raw
test output and individual timings](results.json).

## Reproduce the two inventory builds

Start in a disposable Linux checkout with the exact
[startup composition](../2026-11-11-startup-publication/README.md) restored.
The parent uses MVM 0.27.0; this evidence is not qualification of the newer
upstream release candidate. Set `REPORT` to this report's absolute directory
in the SpiralCore checkout. Apply [the five-file candidate delta](candidate.patch.gz)
from the MVM source root:

```sh
gzip -dc "$REPORT/candidate.patch.gz" | patch -p1
MVM_EMBED=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 cargo +1.97.1 test \
  --release --locked --offline -p mvm-hostd --lib \
  broker::daemon::recovery_tests:: -- --nocapture --test-threads=1
```

To run the pre-fix comparison, reverse **only** the [inventory change](inventory-fix.patch.gz)
with `gzip -dc "$REPORT/inventory-fix.patch.gz" | patch -R -p1` and repeat the
command. The test module is identical on the two builds. The recorded runs
invoked each named test separately on Cargo's identified executable.
[Source hashes](source.json) pin the two builds. Both patches were applied and
the five resulting files checked against the Linux run; reversing the inventory
patch reproduced the prior daemon byte for byte. Source in these patches retains
the [MVM Apache-2.0 license](../2026-11-11-startup-publication/composition/LICENSE).

## Still open

The comparison now consumes each VM identity exactly once, and fresh storage
records pass canonical registration validation before restoration writes.
In this inventory-only candidate, a separate concurrency issue remains: after releasing a mutation loan, a caller
must verify that it reacquired the original ledger row before changing or
settling it. The [subsequent row-binding repair](original-row/README.md) covers
that issue on a separate build. The installed worker must also join its control loop through the
reply before stopping the signer. This candidate is not ready to deploy.
Native guest retirement, session recovery, WAN transfer and platform
qualification remain tracked in [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
