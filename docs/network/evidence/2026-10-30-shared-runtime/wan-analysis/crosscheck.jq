# Independent arithmetic from sender observations; no use of Rust calculations.
def stats:
  sort | length as $n |
  {n:$n,min:.[0],median:(if $n%2==0 then (.[($n/2)-1]+.[$n/2])/2 else .[($n/2|floor)] end),
   p95_nearest_rank:.[((0.95*$n|ceil)-1)],max:.[-1],sum:add};
def names: ["body_bytes","operation_cpu_us","cpu_subtotal_us","operation_wall_us","receiver_cpu_us","receiver_wall_us","exchange_cpu_us","exchange_wall_us","verification_cpu_us","verification_wall_us","request_body_bytes","response_body_bytes","batches"];
def cellkey: [.dataset,.case,.arm,.phase]|join("|");
def compkey: [.dataset,.case,.scenario,.baseline]|join("|");
def metrics:
  . as $rows | names | map(. as $field|{key:$field,value:([$rows[]|.[$field]]|stats)}) | from_entries;
def summed:
  . as $rows | reduce names[] as $field ({};.[$field]=([$rows[]|.[$field]]|add));
def close($a;$b):
  if ($a|type)!=($b|type) then false
  elif ($a|type)=="number" then (($a-$b)|fabs) <= (1e-12 * ([1,($a|fabs),($b|fabs)]|max))
  elif ($a|type)=="object" then (($a|keys)==($b|keys)) and all($a|keys[]; . as $k | close($a[$k];$b[$k]))
  elif ($a|type)=="array" then ($a|length)==($b|length) and all(range(0;($a|length)); . as $i|close($a[$i];$b[$i]))
  else $a==$b end;
map(select(.event=="operation")|.dataset="wan"|.cpu_subtotal_us=(.operation_cpu_us+.receiver_cpu_us)) as $ops |
($ops|group_by([.dataset,.case,.trial,.arm])|
 map(. as $pair|summed|.dataset=$pair[0].dataset|.case=$pair[0].case|.arm=$pair[0].arm|.phase="seed_plus_target")) as $combined |
(($ops+$combined)|group_by(cellkey)|map({key:(.[0]|cellkey),value:metrics})|from_entries) as $actual |
($expected[0].cells|map({key:cellkey,value:.metrics})|from_entries) as $rust |
[$ops|group_by([.case,.trial])[]|. as $pair|
 ["raw","zstd","adaptive"][] as $baseline |
 ["warm_target_only","equal_setup_seed_plus_target","practical_cold_start"][] as $scenario |
 select($scenario!="practical_cold_start" or $baseline!="adaptive") |
 ([$pair[]|select(.arm=="runtime" and ($scenario!="warm_target_only" or .phase=="target"))]|summed) as $r |
 ([$pair[]|select(.arm==$baseline and ($scenario=="equal_setup_seed_plus_target" or .phase=="target"))]|summed) as $b |
 {dataset:"wan",case:$pair[0].case,scenario:$scenario,baseline:$baseline,r:$r,b:$b}] |
group_by(compkey) |
map(. as $rows|{key:(.[0]|compkey),value:{
 dataset:.[0].dataset,case:.[0].case,scenario:.[0].scenario,baseline:.[0].baseline,pairs:length,
 runtime:([$rows[].r]|metrics),baseline_metrics:([$rows[].b]|metrics),
 paired_body_savings_percent:([$rows[]|100*(1-(.r.body_bytes/.b.body_bytes))]|stats),
 paired_sender_cpu_inflation_runtime_over_baseline:([$rows[]|.r.operation_cpu_us/.b.operation_cpu_us]|stats),
 paired_cpu_subtotal_inflation_runtime_over_baseline:([$rows[]|.r.cpu_subtotal_us/.b.cpu_subtotal_us]|stats),
 paired_wall_speedup_baseline_over_runtime:([$rows[]|.b.operation_wall_us/.r.operation_wall_us]|stats),
 cpu_subtotal_wins:([$rows[]|select(.r.cpu_subtotal_us<.b.cpu_subtotal_us)]|length),
 wall_wins:([$rows[]|select(.r.operation_wall_us<.b.operation_wall_us)]|length)}}) | from_entries as $comparisons |
($expected[0].comparisons|map({key:compkey,value:.})|from_entries) as $rustcomparisons |
{schema:"spiralcore.independent-wan-statistics-crosscheck.v1",
 method:"Independent jq arithmetic directly from sender rows; CPU subtotal summed per observation; seed+target summed per pair before statistics; full paired ratios and wins independently computed",
 groups:($actual|length),metrics_per_group:13,comparisons:($comparisons|length),
 cell_mismatches:[$actual|to_entries[]|select(.value!=$rust[.key])|.key],
 comparison_mismatches:[$comparisons|to_entries[]|select(close(.value;$rustcomparisons[.key])|not)|.key],
 cell_keys_equal:(($actual|keys)==($rust|keys)),comparison_keys_equal:(($comparisons|keys)==($rustcomparisons|keys)),
 tolerance:"cell statistics exact; paired floating ratios relative/absolute 1e-12",
 pass:($actual==$rust and close($comparisons;$rustcomparisons))}
