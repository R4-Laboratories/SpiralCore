def total_bytes: .envelope_bytes_sent + .envelope_bytes_received + .framing_bytes_sent + .framing_bytes_received;
def common: IN("seed","repeat","localized","unrelated","localized-after","truncate","append");
. as $r |
[.sender_worker.guest_events[] | select(.event=="sender_operation") | . as $s |
  [$r.receiver_worker.guest_events[] | select(.event=="receiver_operation" and .kind==$s.kind)] as $rx |
  [$r.recovery_worker.guest_events[] | select(.event=="verified" and .kind==$s.kind)] as $v |
  if ($rx|length)!=1 or ($v|length)!=1 or $rx[0].object!=$s.object or $v[0].object!=$s.object or
    $v[0].bytes!=$s.bytes or $v[0].exact!=true or $v[0].read_only!=true then error("operation/recovery identity mismatch") else
  {phase:$phase,source:$source,environment:$environment[0],arm:$r.plan.arm,size:$r.plan.size,trial:$r.plan.trial,
   kind,common:(.kind|common),object_id:.object,object_bytes:.bytes,mode,
   logical_bytes:(.counters|total_bytes),physical_vsock_bytes:((.counters|total_bytes)*2),
   planned_application_request_bytes:.application_request_bytes,planned_application_response_bytes:.application_response_bytes,
   application_count_scope:"planned BatchSender frames; an early Published may skip prepared frames; actual transferred bytes use socket counters",
   sender_guest_cpu_us:.cpu_us,receiver_guest_processing_cpu_us:$rx[0].cpu_us,
   operation_wall_us:.wall_us,exchange_wall_us:.exchange_wall_us,exchange_cpu_us:.exchange_cpu_us,
   sender_pre_exchange_cpu_us:(.cpu_us-.exchange_cpu_us),sender_pre_exchange_wall_us:(.wall_us-.exchange_wall_us),
   receiver_processing_wall_us:$rx[0].wall_us,frames_sent:.counters.frames_sent,frames_received:.counters.frames_received,
   exact_reopened:true,exact_keyless_reboot:true} end] as $ops |
{session:{phase:$phase,source:$source,environment:$environment[0],arm:.plan.arm,size:.plan.size,trial:.plan.trial,
  operations:(.sender.operations),logical_bytes:.logical_envelope_and_framing_bytes,
  physical_vsock_bytes:.physical_vsock_stream_bytes,lifecycle_wall_us,
  native_os_process_cpu_us:([.orchestrator_process_cpu_us,.sender_worker.worker_process_cpu_us,.receiver_worker.worker_process_cpu_us,.recovery_worker.worker_process_cpu_us]|add),
  sender_guest_cpu_us:.sender.cpu_us,receiver_guest_processing_cpu_us:.receiver.cpu_us,
  native_vcpu_thread_cpu_us:([.sender_worker.usage.cpu_ms.value,.receiver_worker.usage.cpu_ms.value,.recovery_worker.usage.cpu_ms.value]|add*1000),
  receiver_disk_sync_us:.receiver_worker.backing_disk_sync_us,recovery_disk_sync_us:.recovery_worker.backing_disk_sync_us,
  pairwise_clock_offset_ms,
  common_operation_wall_us:([$ops[]|select(.common)|.operation_wall_us]|add),
  common_logical_bytes:([$ops[]|select(.common)|.logical_bytes]|add),
  common_sender_guest_cpu_us:([$ops[]|select(.common)|.sender_guest_cpu_us]|add),
  common_receiver_guest_processing_cpu_us:([$ops[]|select(.common)|.receiver_guest_processing_cpu_us]|add),
  exact_reopened:.receiver.exact_reopened,exact_keyless_reboot:true},operations:$ops}
