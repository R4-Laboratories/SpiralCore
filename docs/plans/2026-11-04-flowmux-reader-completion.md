# Original reader completion implementation

Implement the approved additive cleanup direction against
[the reader-completion contract](../api/flowmux-reader-completion-v1.md).

1. Capture public API absence for both non-consuming reader completion methods.
2. Qualify original-deadline timeout and retained endpoint/reader ownership with
   a real paired socket and an armed authority gate; implement the smallest
   shared-core completion operations while preserving legacy results.
3. Forward installed cleanup through the existing payload/FlowSocket seam and
   retain pending reader wakes outside the owned relay ledger until positive
   original-reader completion. Keep timeout/uncertainty custody explicit.
4. Separately reproduce stop during stalled TLS, bind an original setup wake
   immediately after canonical connect, and qualify the same socket/session
   joins without another route or thread.
5. Re-run focused exact-source component cases and scoped Clippy. Obtain
   independent Rust/code/security review; leave native/full endpoint/RSS/WAN
   acceptance open until actual integration qualification.
