# Review correction: preserve FIN writer synchronization

Disposition: candidate7024f2fa remains HOLD. The preceding independent source PASS missed the effect of removing the original Tx mutex before checking local_closed in the Truncated-after-AppFIN branch. The refactored complete() acquires that mutex too late: instrumentation reproduced EOF before the writer flushed FIN and published its closed flag. Ordinary34-test success does not qualify this race.

Restore the original Tx guard before reading local_closed/pending, then release it before complete() reacquires Tx. Preserve original Tx-to-Pending lock ordering, fatal fencing and normal drain. No timeout, frame, codec or authority change is warranted. Final approval requires exact restored-source review, full functional/strict checks and fresh source-qualified coverage. Preserve the failed instrumented run and this correction rather than rewriting the earlier review as if it found the defect.
