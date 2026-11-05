# Installed mailbox socket readiness

The Runtime worker now pins its original private directory and socket entry. A replacement socket, directory, or symlink fails the readiness check. The worker withdraws readiness before invoking the single accepted job. Stop and worker exit also withdraw it. The worker regression reproduced stale readiness before the fix and passed after it. Five standalone identity checks and twelve installed Runtime/kernel checks passed.

This proves a physical socket observation under the private directory's exclusive writer model. It does not authorize a launch, guarantee readiness after the check, or register a Firecracker bridge. The next integration must bind that proof to the original enrolled plan and VM grant and revalidate it around the dedicated 5257 bridge. A Linux cross-process descriptor probe reached the original listener after a same-path rebind. That technique needs independent process/descriptor lifetime and Firecracker privilege qualification.

The patch applies to the exact MVM candidate at the previous resident ownership checkpoint. The three resulting file hashes were checked after patch replay. The Runtime build's before/after source inventories matched.
