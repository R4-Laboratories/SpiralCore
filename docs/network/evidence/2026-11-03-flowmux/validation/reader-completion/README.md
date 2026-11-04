# Original reader deadline and terminal EOF component

Linux x86_64, Rust 1.97.1. This is the explicit-path shared-core component
successor to immutable `ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1`. It is not
native MVM, an installed-resource activation, platform/WAN or performance proof.
Root's [independent Code/Rust/Security component review](../../reviews/reader-completion-code-rust-security.md)
passes on the exact final successor. Installed/native and cross-platform release
acceptance remain separate gates.

The additive non-consuming JoinGuard operation retains its original handle on
timeout and caches its original semantic result after physical join. Mapped
aliases share the same guard. A real reader blocked inside authority handling
must return timeout while its original endpoint remains alive, then positively
join after that gate is released. The actual API-absence run reports exactly
three E0599 errors before implementation. The first protected two-case execution
passes; scoped Clippy then finds one test-only enum sizing lint. After boxing
that fixture state, both cases and `-D warnings` pass. The first warning/failure
and its source identity remain retained, not relabeled as the corrected run.

The requested existing regressions uncover an older terminal TLS race:
duplex 9, mapped 1 and plain paired 8 pass, but authenticated is 7 PASS/1 FAIL
at the positive terminal guard join. The reader target is not reached in that
partial run. A controlled original-ca5 copy matches all 382 input files, and its
authenticated fixture is byte-identical to the candidate fixture. Ten
interleaved executions per protected arm yield **9/10 failures on original ca5
and 9/10 failures with the reader API**. All 20 results remain in
`terminal-eof-control`. This attributes the missing-close_notify terminal race
to existing behavior; it does not turn either arm into a passing qualification.

The successor preserves the existing successful application barrier: processed
authenticated remote FIN, local FIN under the original TX barrier and no pending
reply. Only at that barrier does a next-frame poll's typed UnexpectedEof use the
existing terminal Truncated classification. Partial-frame processing and other
TLS errors are unchanged. Actual pinned-TLS missing-remote-FIN and withheld
Published cases pass as negative checks before and after the correction. A
separate unit isolates the terminal completion transition with real socket and
endpoint ownership: cancellation previously became completed (actual RED), and
the new fresh delivery/scope check now refuses it. That unit's terminal flags
are controlled state, not claimed wire-authenticated or native evidence.

Final exact-source qualification runs **39 entries** across six protected
objects: library 9, duplex TCP 9, mapped stream 1, paired TCP 8, authenticated
stream 10 and reader completion 2. All pass, with zero ignored or filtered
entries in those full target executions. The original unstable positive
fixture additionally passes ten of ten repeated executions (each reports its
normal nine filtered sibling cases). These are repetitions of one case, not
ten new cases. Scoped release Clippy on all six targets with `-D warnings` exits
0 with no warnings. All 384 source files and six protected binaries match
before/after. Full compiler JSON, manifests, original snapshots and binaries
remain on Wick at `/var/tmp/spiralcore-flowmux-reader-20261104`.

Exact source hashes are in `final-source.sha256`. The transferred archive hash
is `7b635465e851b7bac1b7ab068ee730471734aa74d6593154b1bb22f7edf2283b`;
all 182 hashed receipt members were verified after local extraction. The public
[archive](reader-receipts.tar.gz) stores them under `compact/`, alongside its
`compact.sha256` index; the same [member index](receipt-members.sha256) is also
provided directly. The archive is 21,206 bytes and contains no executable. The API RED
command mirror and intent file were added separately from the retained local
pre-implementation script; they are not claimed as members of that archive.
No binaries, payload stores or large build tree were copied to the Mac.

The owned host adapter must still forward explicit completion, retain pending
reader wakes/codec/auxiliary ownership on timeout and wake stalled TLS setup.
The old last-mapped Drop edge remains an installed activation blocker until
that integration is executed. Existing one-Arc uncertainty quarantine holds
auxiliary/lease custody until original-child exit even after codec-pool drain.
The complete parent/helper/bootstrap/TLS/native-thread resource model, positive
original native retirement and full platform/WAN acceptance remain open.
