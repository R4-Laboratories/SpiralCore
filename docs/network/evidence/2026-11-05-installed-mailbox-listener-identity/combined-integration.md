# Combine the installed recovery authority

1. Carry the exact signed Recovery V2 selection and enrollment through Hostd
   and the original parent into Core's prepared mailbox and bound job. Keep
   the V1 constructor and one-shot callback. Add a separate recovery job and a
   default-deny parent check against the original binding, plan and enrollment.
2. In Firecracker, pin the actual already-connected API stream before it is
   dropped. Use the existing final claimed-path check before InstanceStart to
   retain the exact armed bridge and listener identity. The first guest accept
   can revoke readiness immediately after start. Carry only a weak, read-only
   route observation out of the actual original VM and publish it to Runtime
   only after the outer launch succeeds.
3. Only the original V2 worker may use its retained listener and stop wakeup.
   After the canonical clean frame-boundary FIN and quiescence, consume one
   attempt under the original deadline. Recheck the pinned listener and
   bridge, and verify the connected peer belongs to the original live
   Firecracker process. Serialize accepted-descriptor publication and Stop
   under the independent control lock. Release that lock before polling,
   authentication or joining. Keep the existing V1 path separate.
4. Authenticate the enrolled guest and exact launch-bound session before
   any cached Reply is sent. Preserve the original tracker, counters,
   canonical envelopes, queue isolation and budget. Require real socket
   tests for stale inode, foreign peer, one-attempt behavior, Stop during
   both accept and authentication, and refusal before successful launch
   publication. Then exercise same-VM redial over the existing vsock path,
   exact Reply and ACK, one application completion, and original retirement.
