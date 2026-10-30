# Local shard orchestration qualification

This packet preserves the exact local runner after producer-pin integration, its native Rust portable tests, commands and structured RED/GREEN outcomes. Only the three marked producer preflight/checkpoint/download-validation blocks were added in this implementation. The tests extract those blocks from the actual script and use generated temporary JSON plus a mock read-only gh command. They do not execute a WAN campaign, create a capability, start a tunnel/origin, or contact GitHub.

The script SHA256 is `05fee9bc61ff45a9b68b5d61ce67bb3b1f04e0681f7875d4eb610e6b4bf2db57`. Three tests cover29 synthetic shell executions and pass on Bash3.2.57; the full script passes Bash syntax checking. The local test executable is preserved outside Git and its identity is recorded in qualification.json. No executable binary is included here.

Runtime preparation still requires a fresh successful producer for the final repository source, independent artifact-custody review, `qualification/linux-qualified.json` with the real run/artifact/digest pins, and the exact producer manifest at `qualification/producer-build-manifest.json`. This packet does not substitute for those checks. The recorded script and Rust test source contain the actual author's absolute local paths; reproduction elsewhere requires an explicit path adaptation and a newly identified test receipt.

The former shell-only80-case guard receipt is superseded as rejection evidence because of Mac Bash3.2 conditional-command behavior under `-e`. These successor blocks use explicit failing exits, and the recorded native tests assert observed process statuses. The separate workflow validator's91-case guard review is recorded in its own build-reuse packet.

`SHA256SUMS` binds the included script, Rust source, structured qualification, method and this note. It does not cover a future modified script or claim that the campaign ran. Independent source-review status is stated in qualification.json and may have a later explicit review receipt; no approval should be inferred from file presence alone.
