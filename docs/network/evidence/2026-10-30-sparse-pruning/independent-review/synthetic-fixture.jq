# Synthetic checker qualification only. These are NOT measured observations.
def id($n): (("0"*64)+($n|tostring))[-64:];
("1"*40) as $source | ("2"*64) as $exe |
["prefix_insert","middle_insert","unrelated","repeat","localized","scattered","periodic"] as $cases |
{event:"registration",schema:"spiralcore.sparse-pruning.local.v1",baseline:false,source_revision:$source,executable_sha256:$exe,observations:140,cases:$cases,size:16777216,calls_per_process:1,minimum_encode_cpu_micros:100,allocation_instrumentation:false,transport:"none",synthetic_checker_qualification:true},
(range(0;7) as $ci | $cases[$ci] as $case | range(0;10) as $pair |
 (if $pair%2==0 then ["old","new"] else ["new","old"] end) | to_entries[] as $position | $position.value as $arm |
 (if $arm=="old" then 1000 elif $ci<3 then 800 else 1020 end) as $cpu |
 id($ci*10+$pair+1) as $target |
 {event:"sample",case:$case,pair:$pair,arm:$arm,source_revision:$source,executable_sha256:$exe,pid:(100+$ci*20+$pair*2+$position.key),
 target_object:$target,base_object:(if $case=="repeat" then $target else id(1000+$ci*10+$pair) end),target_bytes:16777216,base_bytes:(if $ci<2 then 16773120 else 16777216 end),
 encode_cpu_micros:$cpu,encode_wall_micros:($cpu+100),drop_cpu_micros:10,drop_wall_micros:20,encode_plus_drop_cpu_micros:($cpu+10),encode_plus_drop_wall_micros:($cpu+120),verification_cpu_micros:20,verification_wall_micros:40,entry_to_release_including_verification_cpu_micros:1120,entry_to_release_including_verification_wall_micros:1300,cpu_resolution_qualified:true,
 recipe:(if $case=="unrelated" then {decision:"raw",frames:0,operations:0,serialized_bytes:0,frame_sha256:null,retained_frame_capacity:0,verification:"raw-fallback-original-bytes"} else {decision:"recipe",frames:1,operations:1,serialized_bytes:22,frame_sha256:id(2000+$ci*10+$pair),retained_frame_capacity:36,verification:"legacy-decoder-exact-original-bytes"} end)}),
{event:"completion",samples:140,exact_pair_decisions_and_frames:true,cpu_resolution_qualified:true}
