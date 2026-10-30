# Run with --arg expected_source <frozen revision> --arg expected_executable <SHA256>.
# This checker independently consumes raw child samples; producer completion is
# checked only after reconstructing counts, order, identity and statistics.
def require($ok; $why): if $ok then . else error($why) end;
def digest: type=="string" and test("^[0-9a-f]{64}$");
def whole: type=="number" and .>=0 and floor==.;
def stats($v): ($v|sort) as $s | ($s|length) as $n |
  {count:$n,min:$s[0],median:(($s[(($n-1)/2|floor)]+$s[($n/2|floor)])/2),p95:$s[(($n*0.95|ceil)-1)],max:$s[-1]};
def geom($v): ((($v|map(log)|add)/($v|length))|exp);
["prefix_insert","middle_insert","unrelated","repeat","localized","scattered","periodic"] as $cases |
.[0] as $reg | .[-1] as $completion |
require(length==142; "row cardinality") |
require($reg.event=="registration" and $completion.event=="completion"; "terminal boundaries") |
require($reg.schema=="spiralcore.sparse-pruning.local.v1" and $reg.baseline==false and $reg.observations==140 and $reg.cases==$cases and $reg.size==16777216 and $reg.calls_per_process==1 and $reg.minimum_encode_cpu_micros==100 and $reg.allocation_instrumentation==false and $reg.transport=="none"; "registration contract") |
require(($expected_source|test("^[0-9a-f]{40}$")) and ($expected_executable|digest) and $reg.source_revision==$expected_source and $reg.executable_sha256==$expected_executable; "source or executable pin") |
([.[]|select(.event=="sample")]) as $rows |
require(($rows|length)==140 and all(.[1:-1][];.event=="sample"); "sample cardinality") |
([ $cases[] as $case | range(0;10) as $pair | (if $pair%2==0 then ["old","new"] else ["new","old"] end)[] as $arm | [$case,$pair,$arm] ]) as $schedule |
require([$rows[]|[.case,.pair,.arm]]==$schedule; "pair identity or AB/BA schedule") |
require(all($rows[];.source_revision==$expected_source and .executable_sha256==$expected_executable); "sample source or executable pin") |
require(all($rows[];(.base_object|digest) and (.target_object|digest) and .target_bytes==16777216 and .base_bytes==(if .case=="prefix_insert" or .case=="middle_insert" then 16773120 else 16777216 end)); "fixture identity or length") |
require(([$rows[].target_object]|unique|length)==70 and all($rows[]; if .case=="repeat" then .base_object==.target_object else .base_object!=.target_object end); "fixture distinctness or exact repeat") |
require(all($rows[];(.pid|whole) and .pid>0) and ([$rows[].pid]|unique|length)==140; "child process identity") |
["encode_cpu_micros","encode_wall_micros","drop_cpu_micros","drop_wall_micros","encode_plus_drop_cpu_micros","encode_plus_drop_wall_micros","verification_cpu_micros","verification_wall_micros","entry_to_release_including_verification_cpu_micros","entry_to_release_including_verification_wall_micros"] as $metrics |
require(all($rows[];. as $r | all($metrics[]; $r[.]|whole)); "invalid timer") |
require(all($rows[];.encode_plus_drop_cpu_micros==(.encode_cpu_micros+.drop_cpu_micros) and .encode_plus_drop_wall_micros==(.encode_wall_micros+.drop_wall_micros) and .entry_to_release_including_verification_cpu_micros>=(.encode_plus_drop_cpu_micros+.verification_cpu_micros) and .entry_to_release_including_verification_wall_micros>=(.encode_plus_drop_wall_micros+.verification_wall_micros) and .cpu_resolution_qualified==(.encode_cpu_micros>=100)); "timing arithmetic or resolution label") |
require(all($rows[];.recipe as $r | if $r.decision=="raw" then $r.frames==0 and $r.operations==0 and $r.serialized_bytes==0 and $r.frame_sha256==null and $r.retained_frame_capacity==0 and $r.verification=="raw-fallback-original-bytes" else $r.decision=="recipe" and ($r.frames|whole) and $r.frames>0 and ($r.operations|whole) and $r.operations>0 and $r.operations<=8192 and ($r.serialized_bytes|whole) and $r.serialized_bytes>=9*$r.frames and $r.serialized_bytes<=65480*$r.frames and ($r.frame_sha256|digest) and $r.retained_frame_capacity>=$r.serialized_bytes and $r.verification=="legacy-decoder-exact-original-bytes" end); "recipe verification or counters") |
($rows|group_by([.case,.pair])) as $pairs |
require(($pairs|length)==70 and all($pairs[]; length==2 and .[0].base_object==.[1].base_object and .[0].target_object==.[1].target_object and .[0].recipe==.[1].recipe); "paired fixture or output mismatch") |
require(all($rows[]; if .case=="unrelated" then .recipe.decision=="raw" else .recipe.decision=="recipe" end); "registered fixture decision") |
([ $cases[] as $case |
   [$rows[]|select(.case==$case)] as $cell |
   [$cell[]|select(.arm=="old")] as $old |
   [$cell[]|select(.arm=="new")] as $new |
   [range(0;10) as $pair | ($old[]|select(.pair==$pair)) as $a | ($new[]|select(.pair==$pair)) as $b |
    {pair:$pair,legacy_over_candidate_cpu:(if $b.encode_cpu_micros>0 then $a.encode_cpu_micros/$b.encode_cpu_micros else null end),candidate_win:($b.encode_cpu_micros<$a.encode_cpu_micros)}] as $ratios |
   all($cell[];.cpu_resolution_qualified) as $resolution |
   (if all($ratios[];.legacy_over_candidate_cpu!=null and .legacy_over_candidate_cpu>0) then geom([$ratios[].legacy_over_candidate_cpu]) else null end) as $gmean |
   ($case=="prefix_insert" or $case=="middle_insert" or $case=="unrelated") as $primary |
   ([$ratios[]|select(.candidate_win)]|length) as $wins |
   {case:$case,pairs:10,primary:$primary,cpu_resolution_qualified:$resolution,
    legacy_over_candidate_cpu_geometric_mean:$gmean,candidate_over_legacy_cpu_geometric_mean:(if $gmean==null then null else 1/$gmean end),candidate_cpu_wins:$wins,
    cpu_gate_pass:($resolution and $gmean!=null and (if $primary then $gmean>=1.10 and $wins>=7 else 1/$gmean<=1.05 end)),
    paired_cpu_ratios:$ratios,
    old:([$metrics[] as $metric | {key:$metric,value:stats([$old[]|.[$metric]])}]|from_entries),
    new:([$metrics[] as $metric | {key:$metric,value:stats([$new[]|.[$metric]])}]|from_entries)}
 ]) as $cells |
require($completion.samples==140 and $completion.exact_pair_decisions_and_frames==true and $completion.cpu_resolution_qualified==all($rows[];.cpu_resolution_qualified); "producer completion disagrees with independent counts") |
{schema:"spiralcore.sparse-pruning.independent-timing-verification.v1",status:"VALID_RECORDS",source_revision:$expected_source,executable_sha256:$expected_executable,samples:140,pairs:70,
 exact_pair_decisions_and_frames:70,cpu_acceptance_pass:all($cells[];.cpu_gate_pass),cells:$cells,
 limitations:["Receipt and statistical verification; no independent fixture regeneration or repeat of all decoded byte comparisons.","Encoder CPU/wall and separate recipe-drop times only; no WAN, complete transfer, total RSS or energy result.","Allocation acceptance is a separate diagnostic gate and is not inferred from this timing record."]}
