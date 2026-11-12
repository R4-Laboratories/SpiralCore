# Concurrent volume lease resolution during mailbox guest launch

Two independent SDK starts on the same host share one volume registry. In [Firecracker run 9](../addressed-native-launch/paced-runs.md), guest A requested no explicit volumes and received a busy-registry error before admission. Guest B then reached the pair deadline. The initial request cannot establish whether the VM already has registered mounts or whether a previous launch holds an exclusive lease.

The Unix launch path now waits up to five seconds to acquire the existing lifecycle lock. It opens the named lock once, refuses links and special files, and retries kernel-reported contention on the same descriptor. Before entering the transaction it checks the deadline and compares the held inode to the current name. Once acquired, the original service reads current mount and attachment records, enforces ownership and profile rules, and uses its existing recovery and release paths. Other volume operations keep their previous lock policy.

The regression holds the real registry lock while a second thread starts with an empty explicit volume list. On release the second start succeeds with zero attachments against a valid empty catalog. If the writer leaves a malformed durable lease catalog, the second start refuses it and preserves its bytes. Additional tests exercise timeout while the original holder retains ownership, an invalid lock directory, a dangling symbolic link and a FIFO with no reader. The baseline and mitigation use the same public launch service.

The Apple Silicon macOS results exercise actual filesystem locks and disposable lease records. This suite starts zero guests and transfers zero workload objects. Its elapsed times cover test execution. End-to-end setup and transfer have separate measurements in the native launch reports.

[Revision, commands and results](volume-results.json) · [MVM source changes](https://github.com/Casey-allard/mvm/compare/c492c52c3e6054e640367ad72f857c5314ff22b5...16eabe49c7981cf41a0a6dcb788cd1d86f8fbf62) · [Mailbox delivery issue](https://github.com/R4-Laboratories/SpiralCore/issues/43)
