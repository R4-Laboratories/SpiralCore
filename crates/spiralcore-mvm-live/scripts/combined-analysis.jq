def require($condition; $message): if $condition then . else error($message) end;
def med: sort | length as $n | if $n==0 then error("empty median") elif $n%2==1 then .[($n/2|floor)] else (.[($n/2-1)]+.[($n/2)])/2 end;
def summary:
  . as $rows | {n:length,
    logical_encrypted_stream_bytes:([$rows[].logical_encrypted_stream_bytes]|med),
    physical_vsock_stream_bytes:([$rows[].physical_vsock_stream_bytes]|med),
    sender_cpu_us:([$rows[].sender_cpu_us]|med),
    receiver_processing_cpu_us:([$rows[].receiver_processing_cpu_us]|med),
    endpoint_partial_cpu_us:([$rows[].endpoint_partial_cpu_us]|med),
    elapsed_us:([$rows[].elapsed_us]|med)};
def ratios($rows; $keys):
  [$rows | group_by([$keys[] as $key|.[$key]])[] as $group |
   [["combined","whole-mixed"],["combined-cpu","whole-mixed"],["combined-cpu","combined"]][] as $pair |
   [$group[]|select(.arm==$pair[0])] as $a |
   [$group[]|select(.arm==$pair[1])] as $b |
   require(($a|length)==3 and ($b|length)==3; "paired trial count mismatch") |
   [range(0;3) as $trial |
     [$a[]|select(.trial==$trial)] as $as | [$b[]|select(.trial==$trial)] as $bs |
     require(($as|length)==1 and ($bs|length)==1; "duplicate or missing trial") |
     $as[0] as $x | $bs[0] as $y |
     {trial:$trial,logical_bytes_ratio:($x.logical_encrypted_stream_bytes/$y.logical_encrypted_stream_bytes),
      logical_bytes_delta:($x.logical_encrypted_stream_bytes-$y.logical_encrypted_stream_bytes),
      sender_cpu_ratio:($x.sender_cpu_us/$y.sender_cpu_us),
      endpoint_partial_cpu_ratio:($x.endpoint_partial_cpu_us/$y.endpoint_partial_cpu_us),
      elapsed_ratio:($x.elapsed_us/$y.elapsed_us)}] as $trials |
   ($group[0] | with_entries(select(.key as $key|$keys|index($key)))) +
   {candidate:$pair[0],baseline:$pair[1],n:3,trials:$trials,
    median_paired_logical_bytes_ratio:([$trials[].logical_bytes_ratio]|med),
    median_paired_sender_cpu_ratio:([$trials[].sender_cpu_ratio]|med),
    median_paired_endpoint_partial_cpu_ratio:([$trials[].endpoint_partial_cpu_ratio]|med),
    median_paired_elapsed_ratio:([$trials[].elapsed_ratio]|med)}];
require(length==36; "expected exactly 36 registered sessions") |
[.[].session] as $sessions | [.[].operation_rows[]] as $ops |
require(([$sessions[]|[.stage,.size,.trial,.arm]]|unique|length)==36; "duplicate session") |
[$ops[]|select(.setup|not)] as $payloads |
($payloads|group_by([.stage,.size,.trial,.kind])) as $identities |
require(($identities|length)==96 and all($identities[]; length==3 and ([.[].arm]|unique|sort)==["combined","combined-cpu","whole-mixed"] and ([.[]|[.object_id,.object_bytes]]|unique|length)==1); "paired payload identity mismatch") |
{format:"spiralcore-mvm-combined-analysis-v1",sessions:$sessions,operation_rows:$ops,
 exact_operations:($ops|length),guest_reboot_verified_operations:([$ops[]|select(.exact_guest_reboot)]|length),
 payload_identity_groups:($identities|length),
 session_medians:[$sessions|group_by([.stage,.size,.arm])[]|.[0] as $first|summary+($first|{stage,size,arm})],
 operation_medians:[$ops|group_by([.stage,.size,.arm,.kind])[]|.[0] as $first|summary+($first|{stage,size,arm,kind,object_bytes})],
 session_comparisons:ratios($sessions;["stage","size"]),
 operation_comparisons:ratios($payloads;["stage","size","kind"]),
 limits:["Three paired trials are descriptive; no p95 or significance claim.",
 "Logical bytes include both directions of encrypted envelope/framing; physical stream bytes multiply by two for two guest hops, excluding virtio credit/control traffic.",
 "Sender elapsed includes fixture/codec setup and operations, excludes JSON serialization, ownership copies and independent readback; full lifecycle is separate.",
 "Endpoint CPU partial sum excludes receiver framed receive and idle wait; keep guest CPU separate from enclosing host worker/VMM CPU.",
 "Exchange wall time includes crypto, receiver work and waiting; it is not send-only time.",
 "Local HVF/vsock, not a two-host WAN, production mailbox or energy qualification."]}
