# Retained initial Windows fixture failure

Portable run `41675819705`, head `26dc93a17146693af43529c09db78c7906d47b37`,
Windows Server 2022 x64 job `126056746973`, failed the TCP target: **13 passed,
one failed**. The failing deadline case expected four framing bytes of a partial
payload write after readiness; the actual framing counter remained eight
(readiness only). Its Unavailable, elapsed-deadline and zero-complete-frame
assertions had already passed. This fixture started a 100 ms I/O view before
synchronous canonical verification in a software-crypto debug build, then
incorrectly required progress even if that processing consumed the deadline.
The root-approved test-only correction uses the already canonically authorized
frame/route and starts the short view immediately before connect/write. Exact
partial counters, typed Partitioned, original timeout and fence assertions stay;
the live 900 ms cases separately exercise the full canonical service path.
There is no product change or relaxed partial-progress assertion.

The original job API JSON and raw log remain. Original artifact `13250926520`,
`mvm-mailbox-portable-windows2022-x64-1.zip`, is **20,587 bytes**, SHA-256
`d64a8b7c32e1468e0cadc6e0ff99904456806c23e2280c0d1a0751b6095037f3`,
equal to the GitHub API digest. All 17 inner listed checksums and 18 flat regular
members verify; only this collector's derived extraction was removed. The
original ZIP remains recoverable in `archives/`. Its identity records test merge
`725acb0c005318f37fc363792305b57f3ed989a8`, whose full tree equals that head.
The successful Linux run `41675819702` metadata/log is retained as **superseded**,
not final qualification. Root canceled only the superseded portable run after
failure custody; this collector canceled no workflow. Final qualification must
come from the new `b27fbba9` head, not these partial/canceled portable rows.
