# jq -n --argjson expected PLAN --arg source PATH --slurpfile r result.json
#   --slurpfile host host-events.jsonl -f combined-session.jq
def require($condition; $message): if $condition then . else error($message) end;
def count_bytes: .envelope_bytes_sent + .envelope_bytes_received + .framing_bytes_sent + .framing_bytes_received;
def numeric: type=="number" and .>=0 and floor==.;
def counters_match($a; $b):
  all([["envelope_bytes_sent","envelope_bytes_received"],
       ["framing_bytes_sent","framing_bytes_received"],["frames_sent","frames_received"]][];
       . as $k | all([$a[$k[0]],$a[$k[1]],$b[$k[0]],$b[$k[1]]][]; numeric)
       and $a[$k[0]]==$b[$k[1]] and $a[$k[1]]==$b[$k[0]]);
def worker_ok: .ok==true and .mailbox_bandwidth_profile=="experimental64" and
  .mailbox_rate_bytes_per_second==67108864 and .mailbox_burst_bytes==8388608;
$r[0] as $r |
require(($r.event=="session_done" and $r.ok==true and $r.plan==($expected|del(.name))); "session/plan mismatch: "+$source) |
require(($r.mailbox_rate_bytes_per_second==67108864 and $r.mailbox_burst_bytes==8388608); "mailbox profile mismatch") |
require(($r.receiver.exact_reopened==true and counters_match($r.sender.counters;$r.receiver.counters)); "readback or directional counters mismatch") |
require((($r.sender_worker|worker_ok) and ($r.plan.stage!="two" or
  (($r.receiver_worker|worker_ok) and ($r.recovery_worker|worker_ok) and $r.receiver_worker.backing_disk_synced==true and $r.recovery_worker.backing_disk_synced==true))); "worker completion/profile/disk synchronization missing") |
(if $r.plan.stage=="one" then $host else $r.receiver_worker.guest_events end) as $receiver |
[$r.sender_worker.guest_events[]|select(.event=="sender_operation")] as $sent |
[$receiver[]|select(.event=="receiver_operation")] as $received |
[$receiver[]|select(.event=="verified")] as $verified |
(if $r.plan.stage=="two" then [$r.recovery_worker.guest_events[]|select(.event=="verified")] else [] end) as $reboot |
(if $r.plan.arm=="whole-mixed" then 8 else 10 end) as $n |
require((($sent|length)==$n and ($received|length)==$n and ($verified|length)==$n and $r.sender.operations==$n and $r.receiver.operations==$n); "operation/readback count mismatch") |
require(($r.plan.stage!="two" or ($reboot|length)==$n); "reboot count mismatch") |
require(([$sent[].kind]|unique|length)==$n; "duplicate operation kind") |
require(($r.sender.counters|count_bytes)==$r.logical_envelope_and_framing_bytes; "logical byte total mismatch") |
require(($r.physical_vsock_stream_bytes==$r.logical_envelope_and_framing_bytes*(if $r.plan.stage=="two" then 2 else 1 end)); "physical stream total mismatch") |
[$sent[] as $s |
  [$received[]|select(.position==$s.position and .kind==$s.kind and .object==$s.object and .bytes==$s.bytes)] as $matches |
  require(($matches|length)==1; "receiver operation mismatch") |
  $matches[0] as $v |
  require(counters_match($s.counters;$v.counters); "operation counter mismatch") |
  require(([$verified[]|select(.position==$s.position and .kind==$s.kind and .object==$s.object and .bytes==$s.bytes and .exact==true and .read_only==true)]|length)==1; "operation readback mismatch") |
  require(($r.plan.stage!="two" or ([$reboot[]|select(.position==$s.position and .kind==$s.kind and .object==$s.object and .bytes==$s.bytes and .exact==true and .read_only==true)]|length)==1); "operation reboot verification mismatch") |
  require(all([$s.bytes,$s.cpu_us,$s.wall_us,$s.exchange_cpu_us,$s.exchange_wall_us,$v.cpu_us,$v.wall_us][]; numeric); "missing/invalid timing or size") |
  ($s.counters|count_bytes) as $bytes |
  {stage:$r.plan.stage,arm:$r.plan.arm,size:$r.plan.size,trial:$r.plan.trial,
   kind:$s.kind,object_bytes:$s.bytes,object_id:$s.object,mode:$s.mode,
   setup:($s.kind=="schema" or $s.kind=="runtime-checkpoint"),
   logical_encrypted_stream_bytes:$bytes,physical_vsock_stream_bytes:($bytes*(if $r.plan.stage=="two" then 2 else 1 end)),
   sender_cpu_us:$s.cpu_us,receiver_processing_cpu_us:$v.cpu_us,
   endpoint_partial_cpu_us:($s.cpu_us+$v.cpu_us),elapsed_us:$s.wall_us,
   exchange_cpu_us:$s.exchange_cpu_us,exchange_wall_us:$s.exchange_wall_us,
   sender_nonexchange_cpu_us:($s.cpu_us-$s.exchange_cpu_us),
   receiver_processing_wall_us:$v.wall_us,
   planned_application_bytes:($s.application_request_bytes+$s.application_response_bytes),
   selection:($s.selection // null),exact_reopened:true,
   exact_guest_reboot:($r.plan.stage=="two"),source:$source}
] as $rows |
require(all([$r.sender.cpu_us,$r.receiver.cpu_us,$r.sender.wall_us,$r.lifecycle_wall_us,$r.orchestrator_process_cpu_us,$r.sender_worker.worker_process_cpu_us][]; numeric); "missing session timing") |
{session:{stage:$r.plan.stage,arm:$r.plan.arm,size:$r.plan.size,trial:$r.plan.trial,
  operations:$n,payload_bytes:([$rows[]|select(.setup|not)|.object_bytes]|add),
  logical_encrypted_stream_bytes:$r.logical_envelope_and_framing_bytes,
  physical_vsock_stream_bytes:$r.physical_vsock_stream_bytes,
  sender_cpu_us:$r.sender.cpu_us,receiver_processing_cpu_us:$r.receiver.cpu_us,
  endpoint_partial_cpu_us:($r.sender.cpu_us+$r.receiver.cpu_us),
  elapsed_us:$r.sender.wall_us,lifecycle_wall_us:$r.lifecycle_wall_us,
  operation_elapsed_us:([$rows[].elapsed_us]|add),
  payload_elapsed_us:([$rows[]|select(.setup|not)|.elapsed_us]|add),
  sender_worker_process_cpu_us:$r.sender_worker.worker_process_cpu_us,
  receiver_worker_process_cpu_us:($r.receiver_worker.worker_process_cpu_us // null),
  recovery_worker_process_cpu_us:($r.recovery_worker.worker_process_cpu_us // null),
  orchestrator_process_cpu_us:$r.orchestrator_process_cpu_us,
  relay_process_cpu_us:($r.relay.cpu_us // null),
  exact_reopened:true,exact_guest_reboot:($r.plan.stage=="two"),source:$source},
 operation_rows:$rows}
