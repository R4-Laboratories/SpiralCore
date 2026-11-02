# Publication verification — 2026-11-02

The public GitHub prerelease `evidence-2026-11-02` was published at
2026-11-02T17:04:40Z, targeting documentation commit
`f6fc2bc83a5bc96edd69da23bcaf1eb7c025942f`. It is an evidence publication, not a
new benchmark run or production software release.

All six assets were fetched through fresh anonymous HTTPS requests without
Authorization headers, cookies or stored credentials. Every response returned
HTTP 200; every size and SHA-256 matched the reviewed local file. See
[public-downloads.json](public-downloads.json). Downloads were streamed into
hashes, avoiding another local archive copy.

The GitHub README was opened in the browser. All four Mermaid regions rendered
their intended node labels and edges without a syntax error. The core diagram
and expanded MVM diagram were visually inspected. The narrow in-app browser
scales horizontal diagrams down; GitHub provides expand, pan and zoom controls.
This is not a claim of a comprehensive responsive-design test.

Documentation checks found 59 valid local Markdown links and no missing targets.
`git diff --check` passed. The documented release-mode loopback demo completed
three transfers and verified store restart. Both gateway binaries built with
Rust 1.97.1 and the locked dependencies. The retained smoke receipts are
`demo.txt` and `gateway-build.txt`; their timings are not new benchmark results.
No Rust implementation changed in this publication.

Independent code/documentation and security/artifact reviews are recorded in
`readme-review.md`, `artifact-review.md` and `redistribution-notices.md`. Their
blocking findings are resolved. The installed plugin has no executable
`/quality-gate` command in this environment; the two required review functions
were performed directly by the eligible reviewers rather than claiming that
unavailable command ran.

Original complete archives and historical evidence remain preserved. The four
public subsets retain every included original file byte and omit only the nine
listed executables, with added publication/license metadata. Original SpiralCore
code and documentation use the owner's MIT grant; imported notices and the
explicit third-party research exclusions remain unchanged.
