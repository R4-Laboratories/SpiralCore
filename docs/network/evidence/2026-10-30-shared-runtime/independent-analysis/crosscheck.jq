# A second implementation, directly from raw rows, of all cell statistics.
def stats:
  sort | length as $n |
  {n:$n,min:.[0],median:(if $n%2==0 then (.[($n/2)-1]+.[$n/2])/2 else .[($n/2|floor)] end),
   p95_nearest_rank:.[((0.95*$n|ceil)-1)],max:.[-1],sum:add};
def names: ["body_bytes","operation_cpu_us","operation_wall_us","receiver_cpu_us","receiver_wall_us","exchange_cpu_us","exchange_wall_us","verification_cpu_us","verification_wall_us","request_body_bytes","response_body_bytes","batches"];
def key: [.dataset,.case,.arm,.phase]|join("|");
def metrics:
  . as $rows | names | map(. as $field|{key:$field,value:([$rows[]|.[$field]]|stats)}) | from_entries;
map(select(.event=="operation")|.dataset=(if .width==48 then "small" else "large" end)) as $ops |
($ops|group_by([.dataset,.case,.trial,.arm])|
 map(. as $pair| reduce names[] as $field ({};.[$field]=([$pair[]|.[$field]]|add))|
     .dataset=$pair[0].dataset|.case=$pair[0].case|.arm=$pair[0].arm|.phase="seed_plus_target")) as $combined |
(($ops+$combined)|group_by(key)|map({key:(.[0]|key),value:metrics})|from_entries) as $actual |
($expected[0].cells|map({key:key,value:.metrics})|from_entries) as $rust |
{schema:"spiralcore.independent-statistics-crosscheck.v1",
 method:"Independent jq arithmetic from all raw operation rows, including per-pair seed+target before aggregation",
 groups:($actual|length),metrics_per_group:12,
 mismatches:[$actual|to_entries[]|select(.value!=$rust[.key])|.key],
 keys_equal:(($actual|keys)==($rust|keys)),
 pass:($actual==$rust)}
