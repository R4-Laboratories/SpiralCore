def median: sort | .[(length / 2 | floor)];
def stats:
  {n: length,
   envelope_bytes: (map(.envelope_bytes) | median),
   frame_bytes: (map(.frame_bytes) | median),
   cpu_ms: (map(.cpu_us / 1000) | median),
   elapsed_ms: (map(.wall_us / 1000) | median)};
def pairs:
  map(select(length == 2) | sort_by(.arm) |
    {size: .[0].size, trial: .[0].trial, kind: .[0].kind,
     warm_to_whole_bytes: (.[0].envelope_bytes / .[1].envelope_bytes),
     warm_to_whole_cpu: (.[0].cpu_us / .[1].cpu_us),
     warm_to_whole_elapsed: (.[0].wall_us / .[1].wall_us)});
def paired_summary:
  group_by([.size, .kind]) | map({size: .[0].size, kind: .[0].kind, n: length,
    bytes_ratio: (map(.warm_to_whole_bytes) | median),
    cpu_ratio: (map(.warm_to_whole_cpu) | median),
    elapsed_ratio: (map(.warm_to_whole_elapsed) | median)});
{
  counts: (group_by(.event) | map({event: .[0].event, count: length})),
  operations: (map(select(.event == "operation")) | group_by([.size, .kind, .arm]) |
    map({size: .[0].size, kind: .[0].kind, arm: .[0].arm} + stats)),
  lifecycle: (map(select(.event == "session_complete")) | group_by([.size, .arm]) |
    map({size: .[0].size, arm: .[0].arm} + stats)),
  operation_pairs: (map(select(.event == "operation" and .kind != "schema")) |
    group_by([.size, .kind, .trial]) | pairs | paired_summary),
  lifecycle_pairs: (map(select(.event == "session_complete")) |
    group_by([.size, .trial]) | pairs | paired_summary),
  all_reopened_exact: (map(select(.event == "verified")) | length == 78 and all(.exact)),
  all_owned_stores_removed: (map(select(.event == "session_complete")) |
    length == 12 and all(.owned_store_removed))
}
