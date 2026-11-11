# Direct SDK-fix source checkpoint

These are the actual Wick outputs for standalone host and guest consumers pinned to public MVM source `18b58e1ad2262a1c0bfa23d51f8e5679d3376f60`. Rust 1.97.1, release mode, locked dependencies, two jobs, incremental compilation disabled and nice level 10 were used. All Cargo resolution, compilation and testing occurred on Wick.

The receipt includes executed commands, source and binary identities, and the size and SHA-256 of each retained output. Eighteen distinct focused tests passed. The canonical-request test runs a child copy of itself; its nested success is counted once. Strict host all-target Clippy, both host release binaries, the musl producer build and strict musl Clippy passed. Both lockfiles changed solely by exact replacement of the MVM revision; registry packages and versions were preserved.

This is a consumer-component checkpoint. Native transfer, exact guest readback, ownership release, WAN performance and full-service acceptance are not established by these tests. The previously recorded EE006 failure remains historical negative evidence. Runtime preparation at the new source is recorded separately; the earlier EE archive is not relabeled.
