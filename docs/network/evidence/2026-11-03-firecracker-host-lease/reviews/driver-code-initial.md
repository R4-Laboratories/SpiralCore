# Initial concrete FC hook: CODE/RUST

**BLOCK — three focused integration findings.** Reviewed the uncommitted four-
file product delta over MVM490860cb4e032b357345822269c6d8f43e6594dd,204 added/
24 removed lines. New untracked functional tests are separate and were not
treated as executed qualification. No builds, tests or product edits were made
by this reviewer. Findings were sent to root immediately.

1. **P1 — disk plus lease composition always fails.** daemon.rs:303–313 first
   applies reopen_host_lease_script, changing `setsid nohup sh` to bash, then
   applies reopen_handoff_script. The latter still requires the original sh
   runner at336–338. Some lease+Some disk therefore fails with "handoff launch
   shape changed" before launch. Retain the intended disk-prefix-before-dynamic-
   FD order, but accept exactly the valid composed bash shape; qualify combined
   disk and lease execution rather than only diskless lease rendering.
2. **P1 — leased diskless boot still refills an unleased idle pool.**
   driver/fc.rs:906 constructs idle_shape without considering host_lease. The
   callee filters a local copy at642, preventing claim, but caller1056–1058
   retains Some and invokes refill_in_background after readiness. That can
   create an unleased FC process despite Some requiring no idle pool. Filter
   the shared caller shape for both claim and refill, including empty disks.
3. **P2 — root/no-sudo launch profile regresses.** driver/fc.rs:920 calls the
   newly public privileged_writer_descriptor_objects unconditionally. That
   helper always constructs sudo. Existing writable verification explicitly
   uses direct /proc descriptor inspection at euid0; daemon startup likewise
   elides sudo for root. The new leased root boot therefore requires sudo even
   where canonical root launch worked without it. Preserve direct-root versus
   fixed nonroot privileged inventory. This is a call-site/profile finding;
   inventory internals were not broadly audited.

The remaining inspected structure is coherent: concrete Linux-only additive API,
private Linux module/type re-export and cfg-gated argument propagation leave
non-Linux signatures free of Linux-only types. Existing None paths use the shared
old bounded boot/scope path. Prepared disk validation remains mandatory and the
optional held-kernel recipe retains its borrow through boot. Parent lease custody
remains alive through launch/configuration/error cleanup, and successful actual
FC custody is still required before dropping it. Fixed outer/privileged gates
and disk prefix order are appropriate once composition is repaired. Inventory
object identity remains corroboration, not proof of flock retention or exit.

The typed method-pointer test checks the real concrete API signature only. A
reported one-case GREEN/real-crate compile does not execute all compiled tests,
fake launcher paths or actual guests. Combined descriptor execution, None
compatibility, idle-pool behavior, root/sudo profiles, Mac build paths and real
VMM lease/EX-contention/positive readmission remain pending qualification.

Exact reviewed initial file SHA-256:

```text
1df4f51d43bf6b02d58b113adb766dfce46215fa937850f3b0d0d7972ddddcc6  crates/mvm-backends/src/driver/fc.rs
10085db661bdbc26a19b95b6fdb6dd7b20af67d5d5fcfc8693a3708930f0be29  crates/mvm-backends/src/fc/daemon.rs
e01f30163e99fb063488c53385c903b1bc7f2dfe08409c97ed6c9da7720eba4c  crates/mvm-backends/src/driver/fc/host_lease.rs
7fbd78241ddc20e4d836c20bf8dfb8e8b063b0d0f4e02f710eda05c1426e2ecd  crates/mvm-vmm/src/driver/prepared_disks.rs
```

This initial BLOCK is retained for later narrow fix/test resolution. No native
qualification, full B6a or larger platform/performance/scale/WAN claim follows.
