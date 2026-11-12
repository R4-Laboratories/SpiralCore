# Returning a completed warm-worker call during retirement

A guest's configured call limit can require retiring a worker immediately after it returns its last response. The caller can use that response as soon as the agent has read the complete frame. Another call can use the same slot only after the agent has collected the original process and installed a replacement.

On macOS, a one-slot worker echoed all 257 bytes, including a trailing zero. With the previous synchronous retirement it returned the response after **2,174 ms**. The corrected agent returned it after **60 ms**, while the slot was still busy. The replacement became available at **2,089 ms**. The fixture honors the normal two-second termination grace by ignoring SIGTERM. These are consecutive implementation checkpoints on a single fixture, with one observation per reported path. [Exact observations, CPU scope and test executable hashes](results.json)

## What changed

- **Own the child at launch.** The PID-1 reaper and worker or lifecycle-hook launcher use the same registry lock across the system process launch. If exec fails, Rust can collect the child and return the real error. On success the launcher binds the kernel child to a generation-specific exit record before releasing the lock. A test pauses a real fork before a missing executable is invoked, checks exclusion, receives the original missing-file error, and launches a successful hook.
- **Return the response before recycling.** The pool gives its one existing maintenance thread custody of the original worker through a bounded queue. The slot remains busy during retirement. An uncertain exit keeps the original handle and its slot. The worker's configured privileges, termination grace and exit checks still apply.
- **Close the worker's original process group.** A process in the worker's group can inherit stdout and keep a call waiting after the leader exits. The owner or the reaper uses the kernel's non-consuming exit observation, `waitid(WNOWAIT)`, and signals the group while its original leader still holds the process ID. It then collects that leader. Hooks use single-process signaling. A four-second ordinary pipe holder reproduced the stalled response and exercises the new order.

A separate failure test uses an actual worker that voluntarily exits just after writing a valid response. With the original PID-1 reaper active the old pool exceeded its eight-second observation deadline. The revised pool returns the original bytes, reports the subsequent stale pipe as a call failure and serves the next exact request on a replacement.

## Native observation and remaining scope

In the earlier Firecracker runs, a single-call worker reached recycle, and the SDK received no response inside the 55-second observation window. Allowing two calls per worker let the first call return with a diagnostic. Other boots showed a Rust panic while the standard library was waiting for a child following an exec error. Those original runs did not record who collected the child's status or which errno exec returned. The isolated process tests exercise both failure mechanisms directly. This packet measures the agent component on native macOS and Linux. Corrected-guest behavior at the SDK boundary requires its own booted test.

The payload moved over local worker pipes. Carrier byte counts, send-only time, guest CPU, WAN performance and zstd comparison were outside these tests. Group cleanup covers the original same-credential process group. A descendant changing its group or credentials, or an uninterruptible child, needs separate containment and fairness evidence.

## Reproduction

The patch applies to MVM `1755a2dea880ae5b7fcdf0479ca7f9807face651`. After applying it to that clean checkout and staging the changed files, `git write-tree` must return `6f69fdd79020de99f5336a96ee35f57f6ab86627`. The contributor branch contains the individual failing tests as well as the product fixes. [Patch](worker.patch) · [Tested MVM source](https://github.com/Casey-allard/mvm/commit/0b35a72f3adcbe843b09700401f466f4d7952868)

```sh
cargo test --locked -p mvm-agentd --test worker_pool_warm -- --test-threads=1 --nocapture
cargo test --locked -p mvm-agentd --lib child_wait::tests
cargo test --locked -p mvm-agentd --lib lifecycle_hooks::tests
```
