# Transform synthetic normal records only; never measured allocation data.
if .event=="registration" then
  .schema="spiralcore.sparse-pruning.allocations.v1" | .minimum_encode_cpu_micros=null | .allocation_instrumentation=true
elif .event=="completion" then .cpu_resolution_qualified=null | .allocation_diagnostic=true
else
  . as $row | (.case=="prefix_insert" or .case=="middle_insert" or .case=="unrelated") as $primary |
  (if .arm=="old" then 1000 elif .recipe.decision=="raw" then 0 elif $primary then 400 else 800 end) as $peak |
  (if .arm=="old" then 4 elif .recipe.decision=="raw" then 0 else 2 end) as $headers |
  (if .arm=="old" then 6 elif .recipe.decision=="raw" then 0 else 3 end) as $reallocations |
  {live_bytes:.recipe.retained_frame_capacity,peak_bytes:$peak,cumulative_allocated_bytes:($peak*2),header_allocations:$headers,reallocations:$reallocations,frees:($headers+$reallocations-.recipe.frames),active_buffers:.recipe.frames,registry_failed:false} as $at_return |
  {event:"allocation_sample",arm,case,pair,source_revision,executable_sha256,base_object,target_object,base_bytes,target_bytes,recipe,pid:(.pid+1000),legacy_source_sha256:("4"*64),candidate_source_sha256:("3"*64),at_return:$at_return,after_drop:($at_return|.live_bytes=0|.active_buffers=0|.frees=($headers+$reallocations)),timing_use:"none"}
end
