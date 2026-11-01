def stats: map(select(.!=null))|sort as $x | {n:($x|length),min:$x[0],median:$x[(($x|length)/2|floor)],max:$x[-1]};
def summarize($fields): . as $rows | reduce $fields[] as $f ({}; .[$f]=([$rows[]|.[$f]]|stats));
def paired(key;axis;$num;$den;$fields):
  group_by(key) | map(
    [ .[]|select(axis==$num)] as $a | [ .[]|select(axis==$den)] as $b |
    if ($a|length)==1 and ($b|length)==1 then
      {phase:$a[0].phase,size:$a[0].size,trial:$a[0].trial,arm:$a[0].arm,kind:$a[0].kind,
       numerator:$num,denominator:$den,
       ratios:(reduce $fields[] as $f ({}; .[$f]=(if $b[0][$f]>0 then $a[0][$f]/$b[0][$f] else null end)))}
    else empty end);
def paired_stats:
  group_by([.phase,.size,.arm,.kind,.numerator,.denominator]) |
  map(.[0] as $first |
    {phase:$first.phase,size:$first.size,arm:$first.arm,kind:$first.kind,numerator:$first.numerator,denominator:$first.denominator,
     trials:map(.trial),ratios:([.[].ratios]|summarize($first.ratios|keys))});
["logical_bytes","sender_guest_cpu_us","receiver_guest_processing_cpu_us","operation_wall_us","exchange_wall_us","exchange_cpu_us","sender_pre_exchange_cpu_us","sender_pre_exchange_wall_us"] as $om |
["logical_bytes","lifecycle_wall_us","native_os_process_cpu_us","sender_guest_cpu_us","receiver_guest_processing_cpu_us","common_operation_wall_us","common_logical_bytes","common_sender_guest_cpu_us","common_receiver_guest_processing_cpu_us"] as $sm |
{
  schema:"spiralcore.mvm.overhead-summary.v1",
  interpretation:"n=3 descriptive sample on a shared host with unrelated builds allowed and endpoint load snapshots retained; not clean-host qualification. Ratios are paired by fixture trial, not ratios of separate medians. CPU domains remain separate. Exchange time includes transport framing/authentication, receiver work and waiting; it is not isolated send-only time.",
  operation_medians:($ops|group_by([.phase,.size,.arm,.kind])|map(.[0] as $x | {phase:$x.phase,size:$x.size,arm:$x.arm,kind:$x.kind,metrics:summarize($om)})),
  session_medians:($sessions|group_by([.phase,.size,.arm])|map(.[0] as $x | {phase:$x.phase,size:$x.size,arm:$x.arm,metrics:summarize($sm)})),
  warm_vs_whole_operations:($ops|map(select(.common))|paired([.phase,.size,.trial,.kind];.arm;"warm";"whole";$om)|paired_stats),
  warm_vs_whole_sessions:($sessions|paired([.phase,.size,.trial];.arm;"warm";"whole";$sm)|paired_stats),
  implementation_operations:($ops|paired([.size,.trial,.arm,.kind];.phase;"after-standard";"before-standard";$om)|paired_stats),
  implementation_sessions:($sessions|paired([.size,.trial,.arm];.phase;"after-standard";"before-standard";$sm)|paired_stats),
  profile_operations:($ops|paired([.size,.trial,.arm,.kind];.phase;"after-experimental64";"after-standard";$om)|paired_stats),
  profile_sessions:($sessions|paired([.size,.trial,.arm];.phase;"after-experimental64";"after-standard";$sm)|paired_stats)
}
