# Original endpoint child collection before VMM dispatch

The protected VMM regressions start actual child processes. Before the patch, children exiting 23 and 0 without a matching `Drained` frame both left the original child status uncollected. The existing matched-`Drained` control passed. The successor collects the two original exit statuses, retains the shared holder lock, and refuses a retirement receipt. The same matched-`Drained` control still passes.

The VMM release compile succeeded in both phases. The two regressions returned 101 on the original source and 0 with the patch. The protected binaries and all 4,878 SDK input hashes were read back after the tests. The VMM test phases differ only in the original-child shutdown implementation. The tested VMM source is pinned in the compressed source maps and in the composed check record.

The separate Runtime routing patch passed a locked, offline release library check composed with the tested VMM successor. The two Runtime file changes preserve the actual typed endpoint-spawn failure in the original slot and route shutdown to the original endpoint owner. The Runtime route has compile and source-trace evidence. The actual child regressions exercise the VMM component. Runtime routing, positive pre-VMM retirement, and installed guest transfer and retirement require separate execution.

This development checkpoint does not ship the patch or claim an installed-service, WAN, capacity, or FIPS result. The previous native attempt is recorded in [qualified endpoint startup refusal](../2026-11-05-qualified-endpoint-startup-refusal/README.md).

[Source patches and exact verification receipts](source-and-receipts.tar.xz)
