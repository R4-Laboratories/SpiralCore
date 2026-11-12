# Collecting an original mailbox worker after Stop times out

A signed Stop can time out while the original mailbox helper still runs. Its worker credit and process handle stay with the coordinator. The previous signed Query returned the cached Unresolved state on every stopped slot, even after the helper exited. On Linux the owner suite therefore passed 16 of 17 tests. [Preceding control-admission checkpoint](https://github.com/R4-Laboratories/SpiralCore/pull/157)

An authenticated Query admitted before the provider retirement fence can now inspect an idle Unresolved slot that still owns its original child. It exclusively borrows that child and calls the existing nonblocking process wait. A live child leaves the state Unresolved. An observed exit records the terminal status and refunds one worker credit. A subsequent Query returns that same original owner and status. Queries after the fence or while another operation owns the child remain observational. Metadata and cached-object accounting have their own retention lifetimes. [Source revision](https://github.com/Casey-allard/mvm/commit/c492c52c3e6054e640367ad72f857c5314ff22b5) · [Reviewed delta](late-worker-collection.patch)

The Linux test starts a real subprocess and keeps it alive past the signed Stop deadline. It checks that the worker remains charged, releases the fixture's hold, then asks the same owner to collect the actual exit. A fresh-nonce replay checks the original owner, exit status and zero worker count. The complete Linux mailbox owner suite passed **17/17 in 1.26 seconds**. This subprocess fixture exercises the mailbox helper lifecycle. VM termination, guest delivery and WAN performance require separate runs. [Recorded measurements](late-worker-collection.json)

```sh
cargo test --locked --offline -p mvm-hostd --test mailbox_worker_owner -- --test-threads=1
```
