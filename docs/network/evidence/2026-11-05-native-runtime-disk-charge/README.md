# Portable Runtime disk charge in the installed native witness

The test-first native run completed exact transfer and original retirement, then refused a ledger that held two telemetry reservations but no charge for the retained portable generation. The unchanged ledger reported 8,429,568 cache bytes and zero staging bytes after shutdown.

The successor reserves 112 MiB in the same tenant pool before Runtime can publish a generation. It keeps that cache charge through fixture process exit. The existing 224 MiB staging reservation is separate. The witness releases it after exact readback, original retirement, an empty fixture temporary directory and a bounded metadata check of the retained portable generation. The final closed ledger has exactly three reservations and the exact telemetry plus retained Runtime claim. The stage also keeps uncertainty charged on an interrupted path.

The successor passed its installed Firecracker run. The retained generation occupied 104,759,296 allocated bytes against the 117,440,512-byte reservation. The guest and peer each verified 11,600 bytes. Peer TLS carried 4,321 bytes received and 3,869 sent. Peer elapsed after accept was 55.38 ms, peer application-thread CPU 0.74 ms, host process CPU 1.001 s, and witness wall time including retirement 1.670 s. Both candidates used the same single-guest profile. These one-off timings support no performance comparison.

The protected successor passed 30 focused checks and linked the four native helpers. The source and custody review covered a fresh private same-host witness. The metadata measurement covers the seven generation files and two directories. This fixture charge does not establish durable generation ownership, restart import, a filesystem quota, or concurrent guest capacity. No retained generation was reclaimed.

[Source and receipts](source-and-receipts.tar.xz) · [Post-stop readback](post-stop.txt)
