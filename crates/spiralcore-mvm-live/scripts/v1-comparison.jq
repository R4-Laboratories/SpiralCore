# Input: 24 summaries checked by combined-session.jq, with implementation tags.
def require($ok; $message): if $ok then . else error($message) end;
def med: sort | length as $n | if $n==0 then error("empty median") elif $n%2==1 then .[($n/2|floor)] else (.[($n/2-1)]+.[($n/2)])/2 end;
def metrics: ["logical_encrypted_stream_bytes","physical_vsock_stream_bytes","sender_cpu_us","receiver_processing_cpu_us","endpoint_partial_cpu_us","elapsed_us","actual_frames_sent","actual_frames_received"];
def summary:
  . as $rows | {n:length} +
  (reduce metrics[] as $key ({}; .[$key]=([$rows[]|.[$key]]|med)));
def comparisons($rows; $keys):
  [$rows|group_by([$keys[] as $key|.[$key]])[] as $group |
   [range(0;3) as $trial |
    [$group[]|select(.trial==$trial and .implementation=="legacy")] as $a |
    [$group[]|select(.trial==$trial and .implementation=="v1")] as $b |
    require(($a|length)==1 and ($b|length)==1; "paired trial count mismatch") |
    {trial:$trial} + (reduce (metrics + ["lifecycle_wall_us","planned_application_bytes"])[] as $key ({};
      if $a[0][$key]!=null and $b[0][$key]!=null then
        .[$key]={legacy:$a[0][$key],v1:$b[0][$key],delta:($b[0][$key]-$a[0][$key]),ratio:(if $a[0][$key]>0 then $b[0][$key]/$a[0][$key] else null end)}
      else . end))] as $trials |
   ($group[0]|with_entries(select(.key as $key|$keys|index($key)))) +
   {trials:$trials,median_paired_ratios:(reduce (metrics + ["lifecycle_wall_us","planned_application_bytes"])[] as $key ({};
    if $trials[0][$key]!=null then .[$key]=([$trials[]|.[$key].ratio]|med) else . end))}];
require(length==24; "expected exactly 24 sessions") |
[.[].session] as $sessions | [.[].operation_rows[]] as $ops |
require(([$sessions[]|[.implementation,.stage,.size,.trial]]|unique|length)==24; "duplicate session") |
require(all($sessions[]; .backend=="firecracker" and .arm=="combined-cpu" and .operations==10 and .exact_reopened==true and (.exact_guest_reboot==(.stage=="two"))); "wrong backend/arm/readback") |
([ ["legacy","v1"][] as $i | ["one","two"][] as $stage | [1048576,16777216][] as $size | range(0;3) as $trial | [$i,$stage,$size,$trial] ]|sort) as $expected |
require(([$sessions[]|[.implementation,.stage,.size,.trial]]|sort)==$expected; "missing registration cell") |
require(all($ops[]; all([.actual_frames_sent,.actual_frames_received][]; type=="number" and .>0 and floor==.)); "missing actual frame counts") |
require(all(($sessions|group_by([.stage,.size,.trial]))[]; length==2 and ([.[]|[.actual_frames_sent,.actual_frames_received]]|unique|length)==1); "session actual frame-count mismatch") |
($ops|group_by([.stage,.size,.trial,.kind])) as $pairs |
require(($pairs|length)==120 and all($pairs[]; length==2 and ([.[].implementation]|sort)==["legacy","v1"] and ([.[]|[.object_id,.object_bytes,.mode,.planned_application_bytes,.actual_frames_sent,.actual_frames_received]]|unique|length)==1); "object/mode/planned-byte/actual-frame-count mismatch") |
{format:"spiralcore-v1-endpoint-comparison-v1",backend:"firecracker",sessions:$sessions,operation_rows:$ops,
 exact_operations:($ops|length),guest_reboot_verified_operations:([$ops[]|select(.exact_guest_reboot)]|length),
 identical_planned_application_and_actual_frame_count_groups:($pairs|length),
 session_medians:[$sessions|group_by([.implementation,.stage,.size])[]|.[0] as $first|summary+($first|{implementation,stage,size})+{lifecycle_wall_us:([.[].lifecycle_wall_us]|med)}],
 operation_medians:[$ops|group_by([.implementation,.stage,.size,.kind])[]|.[0] as $first|summary+($first|{implementation,stage,size,kind,object_bytes})+{planned_application_bytes:([.[].planned_application_bytes]|med)}],
 session_comparisons:comparisons($sessions;["stage","size"]),
 operation_comparisons:comparisons($ops;["stage","size","kind"]),
 limits:["Three paired trials are descriptive; no p95, significance or performance qualification claim.",
 "Both implementations use identical combined-cpu codec selection and paid setup; this isolates endpoint extraction/profile validation, not a new codec.",
 "Planned application bytes count complete encoded SCMOBJ01 batches before callback; early Published can skip frames. Actual carrier-frame counts are separately compared. Canonical encrypted envelope sizes vary with per-session identities/encoding.",
 "Mailbox stream counts exclude separate workload-exit service and virtio control/credit traffic. Two guest hops count twice logical stream bytes.",
 "Endpoint CPU is a partial sum; guest/host worker/VMM CPU domains must remain separate.",
 "Sender session elapsed includes fixture and codec setup, excludes independent readback/reboot; lifecycle includes VM startup and readback.",
 "Exchange wall time includes crypto, receiver work and waiting; not send-only time.",
 "Local Linux/vsock run with no enforced mailbox rate limiter; not WAN, production MVM mailbox, physical-energy or total-host qualification."]}
