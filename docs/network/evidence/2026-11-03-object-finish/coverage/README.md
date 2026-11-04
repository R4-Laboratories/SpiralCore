# Selective object-finish coverage

Accepted derivation: [final/summary.tsv](final/summary.tsv), with the exact
new side of the86ca baseline diff in [changed-source.diff](changed-source.diff).

| Production scope | Covered / mapped executable lines | Coverage |
| --- | ---: | ---: |
| Changed object_channel.rs | 12 / 12 | 100% |
| Changed transport.rs | 13 / 13 | 100% |
| Changed object_service.rs timing literal | 1 / 1 | 100% |
| **All changed production** | **26 / 26** | **100%** |
| Whole three production files | 965 / 998 | 96.69% |

The broader scope has33 uncovered mapped lines, retained in
[final/uncovered-production-lines.tsv](final/uncovered-production-lines.tsv).
No changed mapped line is uncovered. Branch coverage is **NA** because LLVM
exported no mapped branches. This is not whole-crate/project coverage.

Only `spiralcore-gateway` was instrumented through the selective wrapper;
dependencies, crypto and shared codec were not instrumented. The existing
owned Cargo target was reused after cleaning only the gateway package. All
fourteen test executables were freshly compiled and copied outside Cargo
with mode500 before execution. Each object's profiles were exported separately
and line unions take maximum hits, rather than adding overlapping counts.
The accepted scopes cover51 test entries across14 targets, including4 zero-test
targets. Test/export stderr is empty, and source/normal-binary hash checks pass.
The original normal executed binaries and three normal gateway binaries were
protected outside Cargo before its reuse; normal performance was not run here.

The first collector missed one additional executable: `tests/probe.rs` uses a
compile-time `CARGO_BIN_EXE` path to spawn the instrumented probe binary. That
initial custody claim was incomplete and is not hidden. The first collection
and derivation remain retained. The
[corrective script](repair-probe-custody.sh) preserved and hashed that fresh
child before rerunning **only the three probe tests**. Parent PID separated
its profiles from those of its child, and both objects were exported
independently with empty stderr. Its original mixed probe export is excluded
from the accepted final union. Thus54 instrumented test executions occurred,
with3 corrective repeats; the accepted14-target suite still has51 entries.
No product, test, deadline or assertion changed for this custody correction.

The executed collection/correction scripts, fresh-object identities, protected
modes, source identities and compact receipts are adjacent. Raw profiles,
LCOV, build JSONL and binaries remain only in the owned Wick directory
`/var/tmp/spiralcore-object-finish-20261103/coverage`. The full source closure
and the separately required fixture manifest were verified unchanged after
collection. Coverage timings are not performance measurements.

After retained-object verification, only the gateway package's instrumented
release artifacts were removed from Cargo's reusable target, preventing a
later normal build from silently reusing wrapper-instrumented cache entries.
Normal and instrumented retained copies remain outside Cargo and verify after
that cleanup. The original Cargo child path is intentionally absent; its
protected copy and the parent object are verified separately afterward.
