# Required args: expected_source, expected_executable (diagnostic),
# expected_candidate (delta.rs SHA256), expected_legacy; --slurpfile timing <normal JSONL>.
def require($ok; $why): if $ok then . else error($why) end;
def digest: type=="string" and test("^[0-9a-f]{64}$");
def whole: type=="number" and .>=0 and floor==.;
def stats($v): ($v|sort) as $s | ($s|length) as $n |
  {count:$n,min:$s[0],median:(($s[(($n-1)/2|floor)]+$s[($n/2|floor)])/2),p95:$s[(($n*0.95|ceil)-1)],max:$s[-1]};
["prefix_insert","middle_insert","unrelated","repeat","localized","scattered","periodic"] as $cases |
.[0] as $reg | .[-1] as $completion |
require(length==142; "allocation row cardinality") |
require($reg.event=="registration" and $completion.event=="completion"; "allocation terminal boundaries") |
require($reg.schema=="spiralcore.sparse-pruning.allocations.v1" and $reg.baseline==false and $reg.observations==140 and $reg.cases==$cases and $reg.size==16777216 and $reg.calls_per_process==1 and $reg.minimum_encode_cpu_micros==null and $reg.allocation_instrumentation==true and $reg.transport=="none"; "allocation registration contract") |
require(($expected_source|test("^[0-9a-f]{40}$")) and ($expected_executable|digest) and ($expected_candidate|digest) and ($expected_legacy|digest) and $reg.source_revision==$expected_source and $reg.executable_sha256==$expected_executable; "allocation source or executable pin") |
([.[]|select(.event=="allocation_sample")]) as $rows |
require(($rows|length)==140 and all(.[1:-1][];.event=="allocation_sample"); "allocation sample cardinality") |
([ $cases[] as $case | range(0;10) as $pair | (if $pair%2==0 then ["old","new"] else ["new","old"] end)[] as $arm | [$case,$pair,$arm] ]) as $schedule |
require([$rows[]|[.case,.pair,.arm]]==$schedule; "allocation pair identity or schedule") |
require(all($rows[];.source_revision==$expected_source and .executable_sha256==$expected_executable and .candidate_source_sha256==$expected_candidate and .legacy_source_sha256==$expected_legacy and .timing_use=="none"); "allocation sample identity or timing use") |
([ $timing[]|select(.event=="sample")]) as $normal |
require(($normal|length)==140 and ([$normal[]|[.case,.pair,.arm]]|unique|length)==140; "normal join cardinality") |
require(all($rows[]; . as $a | any($normal[];.case==$a.case and .pair==$a.pair and .arm==$a.arm and .source_revision==$expected_source and .base_object==$a.base_object and .target_object==$a.target_object and .base_bytes==$a.base_bytes and .target_bytes==$a.target_bytes and .recipe==$a.recipe)); "normal and diagnostic output join") |
require(all($rows[];(.pid|whole) and .pid>0) and ([$rows[].pid]|unique|length)==140; "allocation child identity") |
["live_bytes","peak_bytes","cumulative_allocated_bytes","header_allocations","reallocations","frees","active_buffers"] as $metrics |
require(all($rows[]; . as $row | all(["at_return","after_drop"][]; . as $phase | all($metrics[]; $row[$phase][.]|whole))); "invalid allocation counter") |
require(all($rows[]; .at_return as $a | .after_drop as $b |
  $a.registry_failed==false and $b.registry_failed==false and
  $a.live_bytes==.recipe.retained_frame_capacity and $a.active_buffers==.recipe.frames and
  $a.peak_bytes>=$a.live_bytes and $a.cumulative_allocated_bytes>=$a.peak_bytes and
  $a.header_allocations+$a.reallocations==$a.frees+$a.active_buffers and
  $b.live_bytes==0 and $b.active_buffers==0 and $b.frees==$b.header_allocations+$b.reallocations and
  $b.peak_bytes==$a.peak_bytes and $b.cumulative_allocated_bytes==$a.cumulative_allocated_bytes and
  $b.header_allocations==$a.header_allocations and $b.reallocations==$a.reallocations and
  $b.frees==$a.frees+$a.active_buffers); "allocation lifetime arithmetic, registry or leak") |
($rows|group_by([.case,.pair])) as $pairs |
require(($pairs|length)==70 and all($pairs[];length==2 and .[0].recipe==.[1].recipe); "allocation paired output") |
([ $cases[] as $case |
   [$rows[]|select(.case==$case)] as $cell |
   [$cell[]|select(.arm=="old")] as $old |
   [$cell[]|select(.arm=="new")] as $new |
   ($case=="prefix_insert" or $case=="middle_insert" or $case=="unrelated") as $primary |
   [range(0;10) as $pair | ($old[]|select(.pair==$pair)) as $a | ($new[]|select(.pair==$pair)) as $b |
     {pair:$pair,old_peak:$a.at_return.peak_bytes,new_peak:$b.at_return.peak_bytes,
      reduction:(if $a.at_return.peak_bytes>0 then 1-$b.at_return.peak_bytes/$a.at_return.peak_bytes else null end),
      pruned_candidate_zero_headers:($b.at_return.header_allocations==(if $b.recipe.decision=="raw" then 0 else $b.recipe.frames+1 end))}] as $ratios |
   {case:$case,primary:$primary,pairs:10,paired_peaks:$ratios,
    memory_gate_pass:(if $primary then all($ratios[];.reduction!=null and .reduction>=0.5 and .pruned_candidate_zero_headers) else null end),
    old:([$metrics[] as $metric|{key:$metric,value:stats([$old[]|.at_return[$metric]])}]|from_entries),
    new:([$metrics[] as $metric|{key:$metric,value:stats([$new[]|.at_return[$metric]])}]|from_entries)}
 ]) as $cells |
require($completion.samples==140 and $completion.exact_pair_decisions_and_frames==true and $completion.cpu_resolution_qualified==null and $completion.allocation_diagnostic==true; "allocation completion disagrees") |
{schema:"spiralcore.sparse-pruning.independent-allocation-verification.v1",status:"VALID_RECORDS",source_revision:$expected_source,executable_sha256:$expected_executable,candidate_source_sha256:$expected_candidate,legacy_source_sha256:$expected_legacy,samples:140,pairs:70,joined_normal_samples:140,all_frees_verified:true,
 memory_acceptance_pass:all($cells[]|select(.primary);.memory_gate_pass),memory_gate:"Every one of ten pairs in each primary cell has at least50percent peak requested frame-buffer capacity reduction and no rejected candidate header materialization",cells:$cells,
 limitations:["Diagnostic tracked realloc uses allocate-copy-free and counts actual old/new overlap under that diagnostic policy; not production System realloc or RSS.","Source-bound nine-byte header discovery excludes caller inputs and planner vectors; it is frame-buffer requested-capacity attribution, not total memory.","Normal/diagnostic receipt identity and arithmetic checked; no independent rerun of all reconstructed bytes or fixture generation."]}
