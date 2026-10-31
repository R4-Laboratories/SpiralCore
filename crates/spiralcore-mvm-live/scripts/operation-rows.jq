# jq -s --slurpfile result SESSION/result.json -f operation-rows.jq SESSION/host-events.jsonl
$result[0] as $r |
(if $r.plan.stage == "one" then . else $r.receiver_worker.guest_events end) as $receiver_events |
$r.sender_worker.guest_events[] | select(.event == "sender_operation") as $sent |
($receiver_events[] | select(.event == "receiver_operation" and .kind == $sent.kind)) as $received |
($sent.counters.envelope_bytes_sent + $sent.counters.envelope_bytes_received + $sent.counters.framing_bytes_sent + $sent.counters.framing_bytes_received) as $wire |
{
  stage: $r.plan.stage, arm: $r.plan.arm, size: $r.plan.size, trial: $r.plan.trial,
  kind: $sent.kind, object_bytes: $sent.bytes, object_id: $sent.object,
  mode: $sent.mode,
  planned_application_batch_bytes: ($sent.application_request_bytes + $sent.application_response_bytes),
  application_count_scope: "planned BatchSender frames; early Published can skip prepared frames; use actual socket counters for transferred bytes",
  logical_encrypted_stream_bytes: $wire,
  physical_vsock_stream_bytes: ($wire * (if $r.plan.stage == "two" then 2 else 1 end)),
  sender_reported_cpu_us: $sent.cpu_us, receiver_processing_cpu_us: $received.cpu_us,
  sender_plus_receiver_processing_cpu_us: ($sent.cpu_us + $received.cpu_us),
  cpu_scope: "sender process CPU plus receiver processing CPU excluding framed receive; guest and host OS accounting domains differ; not physical energy",
  encode_through_publication_us: $sent.wall_us,
  sender_exchange_cpu_us: $sent.exchange_cpu_us,
  sender_exchange_wall_us: $sent.exchange_wall_us,
  receiver_processing_wall_us: $received.wall_us,
  exact_reopened: $r.receiver.exact_reopened,
  exact_guest_reboot: (if $r.plan.stage == "two" then ([ $r.recovery_worker.guest_events[] | select(.event == "verified" and .kind == $sent.kind and .exact == true) ] | length == 1) else null end),
  source_result: ($r.plan.stage + "-" + ($r.plan.size|tostring) + "-" + $r.plan.arm + "/result.json")
}
