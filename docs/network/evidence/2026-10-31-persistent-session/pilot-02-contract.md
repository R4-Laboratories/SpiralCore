# One subsequent performance pilot after diagnostic qualification

Registered before launch. The initial 396-transfer pilot at source `58990cec` failed
after 37 acknowledged operations; its cause was not retained. The new sanitized
diagnostic at source `dbdbc709`, GitHub `41168897547`, completed all 66 operations and
independent stopped-store exact verification. It did not reproduce or explain
the original failure. Both datasets remain separately retained.

Proceed with **one** new six-pair-per-size pilot, **396 total transfers**, using
the original measurement-contract's sizes, fixtures, six arm permutations,
timing boundaries, 32-request/500ms connection policy, equal controls and
durability. Enable the same sanitized diagnostics for every arm. No timeout,
codec, retry or transfer behavior changes. Record fresh source/run identities,
fresh namespace plans, six new bounded stores and a new temporary capability.
All99 compiled-source manifest entries remain unchanged from qualified `dbdbc709`;
reuse its verified frozen binaries and record that equivalence explicitly.

Stop on the first failed operation, retain its client/connection records and
store, and diagnose the concrete failure. Do not replace a trial or rerun this
pilot until a favorable result appears. A new failure is evidence, not a reason
to silently relax the policy. No builds, tests or uploads overlap transfer timing.
Reserve all six 617.2578125 MiB store bounds and the 4 GiB free-disk floor before launch.

Require independent stopped-store comparison for all 396 objects before any
completed-pilot claim. Preserve every loss and outlier. Keep initial pilot,
diagnostic and this pilot separate; do not pool partial or diagnostic timing
observations. Any performance conclusion applies to this observed pilot only.
Report the original failed attempt separately as an unresolved reliability
limitation, even if this pilot completes. No transport-fix or production-
reliability claim follows from a successful subsequent run.
