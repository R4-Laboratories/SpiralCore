# Composed installed resident and canonical Runtime

One isolated Wick Cargo graph now compiles the resident launch and settings changes with the canonical MVM Runtime and VMM operator snapshot consumer. It includes the typed shutdown and same-pool pre-TLS reservation changes. The existing bounded signer framing, one-use admission and parent binding were retained. The patch and per-file map specify 26 exact SDK leaves. The before and after source inventories and protected binary hashes matched.

The composed resident batch passed 27 tests. The separately executed original host-agent and signer fixture passed with the same protected test binary and the same helper files. This checks the composition and real helper startup and shutdown. The Runtime endpoint consumer still needs execution in this graph and no Firecracker guest was booted.

Source inspection found a remaining gap in the canonical local launch. It rebuilds the network policy from the signed grant's host and port list, which drops the authenticated payload mapping. The earlier endpoint fixture provided its policy directly. A separate regression now exercises canonical dispatch with a real signed admission and bundle and a substituted peer certificate pin. The fix must carry the exact signed policy on the original token without widening the egress grant or changing existing DNS and rate enforcement.
