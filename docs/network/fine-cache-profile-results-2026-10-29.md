# SC-OPT-005: cache layout and finer CPU attribution

The registered diagnostic completed **72 exact observations with no failures or
retries**. It confirms that the cost of checking the cache depends strongly on its
layout. The finer category timers added too much overhead to justify selecting a
production optimization. No production candidate was adopted.

The run used a diagnostic snapshot of `d03fe1630bba787a28cc3fa3ae201c4a1a75388b`,
native hardware SHA, and a fixed 1 MiB object with a 32-byte edit. Four layouts,
raw/sparse methods and three instrumentation controls were observed in three
rotating blocks. Timing ran from 19:18:18 to 19:21:51 UTC. This is a local mechanism
diagnostic, not a new WAN speed result or an old/new implementation comparison.

## Counter-only observations

Each median below summarizes three observations. Counter-only still includes the
shared outer instrumentation. Scan time is a subset of operation time; it is not
added to it. The fraction is the median of individual scan/operation fractions.

| Background cache layout | Sparse operation CPU, median | Quota-scan CPU, median | Scan fraction, median |
|---|---:|---:|---:|
| No background objects | 4.775 ms | 0.219 ms | 4.59% |
| 64 objects in one namespace | 7.903 ms | 1.561 ms | 13.59% |
| 768 objects in one namespace | 14.047 ms | 8.381 ms | 58.64% |
| 768 objects across 384 namespaces | 28.230 ms | 23.854 ms | 84.50% |

The two 768-object layouts contain the same background object population. Their
contrast points to namespace traversal as a scaling concern. These are descriptive
observations with uncontrolled operating-system caches, not a guaranteed saving
available to an implementation change. All raw-method and control observations,
including nonmonotonic results, remain in the complete packet.

## Finer attribution failed its primary screen

In the preregistered primary `spread768/sparse` group, the median paired
fine-clock/counter-only operation CPU ratio was **1.5333159586482494**, above the
1.25 maximum. All three paired ratios exceeded that maximum. Empty-clock controls
had a median ratio of **1.5339709528869996**. Neither overhead is subtracted as an
invented exact correction.

Enumeration's fine-clock median was 21.509 ms versus 1.371 ms for its empty span,
and metadata and namespace comparison also exceeded some component thresholds.
However, **all six primary category screens fail the complete rule**. Only two of
48 screens pass elsewhere: metadata and namespace comparison in
`compact768/sparse`. Those secondary results do not replace the primary result or
revive the previously rejected formatting-hoist candidate.

A future proposal may investigate lower-overhead enumeration or metadata access.
It must preserve fresh Begin/Finish observations, quota errors and external cache
growth/addition/deletion behavior, and receive its own correctness and performance
qualification. This diagnostic does not justify a cached quota catalog.

## Exactness, accounting and review

All nine processes exited zero. The retained records contain 828 frames, 144 quota
scans, 57,744 metadata API calls and 37,860,264 modeled application-frame bytes.
Raw operations model 1,051,138 bytes each and sparse operations 536; these are
in-process protocol counts, not captured traffic. Raw still performs optional DAG
warming while sparse does not; this diagnostic is not the new fair raw comparator.

An independent Rust checker reproduced every metric in all 24 cells, all eight
paired-control groups and all 48 screens. It checked the execution schedule,
source/executable bindings, all 1,612 template hashes, the live template-file census
and the ObjectIds of 1,604 template publications. Three checker tests and five
mutated-record/overflow negatives passed. The author reopened each measured output
before removing its temporary working cache; the independent reviewer checked that
source and those receipts and rehashed retained templates, rather than reopening
already removed measured targets.

Exact syscall and allocator-event counts, energy and WAN performance remain
unmeasured. An actual invalid-UTF8 filesystem entry could not be created on this
host and remains NOT_RUN; pure path classification tests do not establish that case.

The [complete evidence packet](evidence/2026-10-29-fine-cache-profile/README.md)
contains the raw rows, all-cell analysis, source, build identity, registration,
independent method and review. Verification text was retired only after durable
structured preservation; original premeasurement bindings remain unchanged.

Later setup review found that native Bash 3.2 does not enforce a standalone failing
`[[ ... ]]` through `set -e`. Frozen diagnostic scripts were not rewritten. Their
rejection paths are not qualified by that shell behavior; the successful run's
actual source, inputs, environment, counts and manifests were independently checked.
Future script reuse must repair and requalify those rejection paths.
