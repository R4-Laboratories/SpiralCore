def targets: map(select(.phase=="target"));
def key: [.mode,.trial,.workload]|@json;
def median: sort|.[length/2|floor];
def pct($new;$old): if $old>0 then 100*($new/$old-1) else null end;
def totals: group_by(.mode)|map({mode:.[0].mode,targets:length,
  frame_bytes:(map(.request_frame_bytes+.response_frame_bytes)|add),
  total_thread_cpu_us:(map(.total_thread_cpu_us)|add),wall_us:(map(.total_wall_us)|add),
  sender_outside_dispatch_cpu_us:(map(.sender_outside_dispatch_cpu_us)|add),
  receiver_dispatch_cpu_us:(map(.receiver_dispatch_cpu_us)|add)});
($before|targets|map({key:key,value:.})|from_entries) as $b |
($general|targets|map({key:key,value:.})|from_entries) as $g |
($small|targets|map(. as $a|$b[key] as $old|$g[key] as $newgeneral|
  if $old.object!=$a.object or $old.length!=$a.length or $newgeneral.object!=$a.object or $newgeneral.length!=$a.length then error("unmatched semantic input") else
  {mode,trial,workload,length,object,bytes_before:($old.request_frame_bytes+$old.response_frame_bytes),bytes_after:(.request_frame_bytes+.response_frame_bytes),
   cpu_pct:pct(.total_thread_cpu_us;$old.total_thread_cpu_us),wall_pct:pct(.total_wall_us;$old.total_wall_us),
   sender_cpu_pct:pct(.sender_outside_dispatch_cpu_us;$old.sender_outside_dispatch_cpu_us),receiver_cpu_pct:pct(.receiver_dispatch_cpu_us;$old.receiver_dispatch_cpu_us),
   general_wire_same:($old.request_frame_bytes==$newgeneral.request_frame_bytes and $old.response_frame_bytes==$newgeneral.response_frame_bytes and $old.selected==$newgeneral.selected)} end)) as $pairs |
{scope:"single-host in-process core frames/private-file publication; no TLS/network/MVM/energy or causal timing claim",
 before_source:"2a9d2197d7c73fed46203d00440555665660ff13",trials:3,workloads:7,modes:5,
 exact_readbacks:{before:($before|targets|map(select(.exact_readback))|length),after_general:($general|targets|map(select(.exact_readback))|length),after_small:($small|targets|map(select(.exact_readback))|length)},
 all_general_wire_equal:($pairs|all(.general_wire_same)),aggregate_declared_small:($small|map(.aggregate_declared_operation_bytes//empty)|unique),
 before_totals:($before|targets|totals),after_general_totals:($general|targets|totals),after_small_totals:($small|targets|totals),
 paired_by_mode:($pairs|group_by(.mode)|map({mode:.[0].mode,frame_byte_delta:(map(.bytes_after-.bytes_before)|add),median_cpu_pct:(map(.cpu_pct)|median),median_wall_pct:(map(.wall_pct)|median),median_sender_cpu_pct:(map(.sender_cpu_pct)|median),median_receiver_cpu_pct:(map(.receiver_cpu_pct)|median)})),
 losses:($pairs|map(select(.bytes_after>.bytes_before or .cpu_pct>0 or .wall_pct>0))),
 paired_rows:$pairs}
