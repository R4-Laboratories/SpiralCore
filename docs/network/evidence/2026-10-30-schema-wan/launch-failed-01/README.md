# Retained initial service launch failure

Source `6c4e5464bdd1f65b436603bf855be7aee80f2dce`, GitHub run
[41127963832](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41127963832),
was cancelled before its transfer step. The final API receipt records that step
as skipped. No measured schema transfer was attempted, and all three stores
contained only their writer-lock file (4 KiB allocated apiece).

The local launcher created receivers and temporary tunnels with `nohup`, then
returned. All six recorded PIDs were subsequently absent. Tunnel logs show
successful registration followed by abrupt termination; HTTPS probes returned
530. This is consistent with execution-tool descendant cleanup when its owning
command finishes. It is a service-lifecycle failure, not a codec or transfer
performance result.

Retain the original launcher, three plans, receiver/tunnel output, probe statuses
and GitHub lifecycle snapshots. The replacement keeps the owning command alive
with a foreground `wait`, verifies PIDs and origin HTTP rejection responses in a
separate call, and uses a fresh run ID, capability, URLs and stores. The codec,
benchmark, workflow and frozen executables are unchanged. The new source revision
adds only this failed-launch evidence before the next first-attempt workflow.
