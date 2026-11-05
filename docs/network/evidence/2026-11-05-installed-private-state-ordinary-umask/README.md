# Installed private state under ordinary umask and upstream source comparison

The guarded Runtime composition keeps owner verification, the canonical lifecycle ancestry check, private state-directory preparation, slot reservation and custody marker in that order. Runtime's one-use startup workspace remains held through signed verification and kernel binding, then releases before endpoint preparation. The private directory helper creates the installed state using descriptor-relative operations. It refuses an unsafe existing leaf without chmod or slot insertion.

The isolated predecessor rejected an unsafe ancestor after creating its vms directory. The reviewed successor rejected before creating vms, reserving a slot, or writing a marker. All six focused cases passed under umask 0022. The full guarded composition then passed 30 focused checks, verified 5,409 source hashes and linked four native helpers.

In the full native replay, sudo/setpriv verified inherited umask 0022. The vms parent and both VM leaves were mode 0700. The first guest rejected an authenticated empty application reply after the peer verified 11,600 request bytes. It retired through the original owners. The second guest then verified 11,600 bytes each way and retired. Both original Firecracker scopes were inactive, both endpoint PIDs were absent and the staged endpoint inventory was gone. Host test-process CPU was 2.124 s and full lifecycle wall time was 3.397 s. The run uses two sequential same-host guests.

## Source and review sequence

MVM coordination issue [#5057](https://github.com/tinylabscom/mvm/issues/5057) references the published mailbox-preparation fork at ae12e9d. The current executed Linux graph uses an isolated overlay on fork commit 9a3a964bd540d5e74c71f9b7df93cc5593295546 plus separately pinned SpiralCore components and a locked build. Its protected binary and exact per-file source pins are in this packet.

Against upstream main 6b0f740e2f02e052b3e10fe97b83a5e7d2cbb4fb, the 82 modified tracked files in the SDK snapshot split into 21 identical preimages, one candidate-identical blob, 36 differing upstream blobs and 24 paths absent upstream. The compatibility inventory also records 52 candidate Rust paths absent from the fork preimage and the parent paths omitted by the execution snapshot. These are path/blob classifications. The comparison has no executed rebase, merge, or upstream qualification. The direct directory patch touches two differing Runtime paths and one installed Runtime path absent upstream, so the lifecycle prerequisite needs its own review.

The existing mailbox adapter and its dedicated GuestService carrier remain the inputs for the next installed mailbox slice. The installed Runtime's workload specification currently carries FlowMux and the other standing ports. It lacks a dedicated mailbox route and owned listener attachment. A signed launch selection, original parent funding, a private listener with separate worker and queue custody, and verified shutdown must precede adding that route. Existing canonical mailbox capabilities remain the operation authority. The FlowMux result here does not exercise those mailbox semantics.

[Native result](result.json) · [Directory and post-stop readback](post-stop.txt) · [SDK path and blob comparison](upstream-sdk-compatibility.json) · [Exact source and test receipts](source-and-receipts.tar.xz)
