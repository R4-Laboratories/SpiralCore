# Receipt recovery with two complete mailbox recordings

One Linux x86-64 host process ran the original MVM mailbox helper, a Unix-stream guest stand-in using the original guest agent's sender, and the boot-bound reader. After the first helper exited, the fixture collected its closed counter file and restored the same VM state directory. The restarted helper recovered the identical signed local-durable receipt for the original message. The reader had already acknowledged the original 12-byte payload `local report` and found the queue empty after restart. [Original test and source revision](https://github.com/Casey-allard/mvm/blob/8238000662f59893b6ec703ab4b37d7a86c08340/crates/mvm-hostd/src/mailbox_process.rs#L1301) · [Full measured run](linux-result.txt)

| Session | Guest → helper bytes read | Helper → guest bytes written | Receipt verification wall / sender thread CPU | Helper lifetime wall / helper thread CPU |
|---|---:|---:|---:|---:|
| First admission, readback and ACK | 4,818 | 8,722 | 2.425 / 0.256 ms | 21.651 / 5.547 ms |
| Duplicate after restart, empty read | 432 | 4,479 | 3.161 / 0.176 ms | 509.734 / 3.957 ms |

The receipt exchange within each session accounted for 80 bytes toward the helper and 1,131 bytes back. The larger totals include the boot challenge, read and ACK operations. The first recorder also recorded a terminal zero-byte exchange with one read and one write error. Both recorders ended with their expected footers. The receipt times start before the sender accepts its Unix socket. The helper lifetime includes startup, processing and shutdown. The entire test executable took 0.53 s wall, 0.01 s user CPU and 0.00 s system CPU at GNU time's precision. This single host component run exercised neither a guest VM nor a WAN link.

## Failure and correction

At the pinned revision, the optional recorder used exclusive creation of one fixed filename. Re-entering the directory with the closed file still present prevented the second helper from dialing, and the original test failed at its 10-second guest deadline (10.04 s total). The test-only overlay adds an explicit collection step after each original helper has joined. It checks the bounded transcript and moves the same inode into a fresh archive directory. The first archived transcript is compared again after collecting the second. The two original temporary roots are created with mode 0700, including under the tested 0002 umask. An incomplete transcript, group-writable parent or occupied archive slot refuses collection. Production mailbox/service code and the recorder wire format have no edits in this patch. The fixture covers orderly helper retirement. Collection after an unclean exit and operating a production collector require separate work. [Original issue checkpoint](https://github.com/R4-Laboratories/SpiralCore/issues/43#issuecomment-6827459643)

## Reproduction

Use Rust 1.97.1 and a disposable checkout of the linked MVM revision. From the MVM checkout, point `sc` to this SpiralCore checkout. Run the last test command once before applying the overlay to observe the original failure. Then apply the overlay and run both suites.

```sh
w="$sc/tools/ari-own-mailbox-witness"
m=crates/mvm-hostd/src/mailbox_process
git apply --check "$w/receipt-restart/driver-only.patch" &&
git apply "$w/receipt-restart/driver-only.patch"
cp "$w/receipt-restart/closed_meter.rs" "$m/closed_meter.rs"
cp "$w/bin/guest_stream.rs" "$m/spiralcore_guest_stream.rs"
umask 0002
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cargo +1.97.1 test --locked -p mvm-hostd --lib --features durable-guest-io-witness \
  mailbox_process::closed_meter:: -- --test-threads=1
cargo +1.97.1 test --locked -p mvm-hostd --lib --features durable-guest-io-witness \
  mailbox_process::tests::production_helper_requires_retained_custody_and_returns_signed_local_receipt \
  -- --exact --test-threads=1 --nocapture
```

Four collector/parser controls passed. The restarted-helper test also passed with the recorder disabled. The exact metered executable SHA-256 was `fc48e5c70b73a6bf96d40e3f54805f1bf02dc190a4c49b8720131093a43e9eb4`. The original failing Cargo log digest was `cc09f1a5b4c493d5da17a9e14406323d51a68a4f37da70a847875f741b6c599f`. The retained final executable output digest is `e12d80da9c9721186ca250269e805cfb630d66a9d539bfc6ffef13ce8516af08`.
