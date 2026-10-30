# Independent receipt arithmetic. This consumes only recorded rows, not the Rust summarizer.
def require($ok; $why): if $ok then . else error($why) end;
def stats($values):
  ($values | sort) as $s | ($s|length) as $n |
  if $n == 0 then error("empty statistic") else
    {count:$n,min:$s[0],median:(($s[(($n-1)/2|floor)]+$s[($n/2|floor)])/2),
     p95:$s[(($n*0.95|ceil)-1)],max:$s[-1]}
  end;
def id: type=="string" and test("^[0-9a-f]{64}$");
def cell($row; $summary): $row.case==$summary.case and $row.size==$summary.size and $row.arm==$summary.arm;
. as $all |
require(length==3043; "row cardinality") |
require(.[0].event=="registration" and .[-1].event=="completion"; "terminal boundaries") |
([.[]|select(.event=="registration")]) as $registrations |
([.[]|select(.event=="operation")]) as $ops |
([.[]|select(.event=="summary")]) as $summaries |
([.[]|select(.event=="cleanup")]) as $cleanup |
([.[]|select(.event=="completion")]) as $completion |
([.[]|select(.event=="paired_acceptance")]) as $acceptance |
require(($registrations|length)==1 and ($ops|length)==1800 and ($summaries|length)==40 and ($cleanup|length)==1200 and ($completion|length)==1 and ($acceptance|length)==1; "event cardinalities") |
require(all(.[]; .event=="registration" or .event=="operation" or .event=="summary" or .event=="cleanup" or .event=="completion" or .event=="paired_acceptance"); "unexpected event") |
$registrations[0] as $reg |
require($reg.trials==30 and $reg.sizes==[1048576,16777216] and $reg.expected_operations==1800; "registration counts") |
require($reg.cases==["cold_random","cold_structured","localized_32","unrelated_structured"] and $reg.arms==["raw","zstd","sparse","prefix","adaptive"]; "case and arm contract") |
require($reg.claimed_source_revision=="850272f01d580d1cf7564d25fb628fd50f58d956" and $reg.executable_sha256=="c165a97205f858578f3c25d02464a725503660e21cf7c318811201f548c4d3fd" and $reg.harness_source_sha256=="ae62780923e98b7b23dce839c8e00035c4c7e58f7630ff5a21c159478854a7c7"; "source identity") |
require(all($ops[]; .verification=="exact-after-reopen" and .verified_bytes==.object_bytes and .object_bytes==.size and (.object|id) and (.scope|id)); "exact verification identity and length") |
require(([$ops[].verified_bytes]|add)==16043212800; "compared bytes") |
require(([$ops[]|[.pair,.arm,.phase]]|unique|length)==1800; "duplicate operation identity") |
# Build the exact ordered operation keys independently from the registration.
([ $reg.sizes[] as $size | range(0;4) as $case_index | $reg.cases[$case_index] as $case |
    range(0;30) as $trial | range(0;5) as $position | $reg.arms[(($position+$trial+$case_index)%5)] as $arm |
    (if ($case|startswith("cold_")) then ["transfer"] else ["seed","transfer"] end)[] as $phase |
    [$size,$case,$trial,$arm,$phase] ]) as $schedule |
require([$ops[]|[.size,.case,.trial,.arm,.phase]]==$schedule; "registered operation order") |
require(all($ops[]; .pair==((.size|tostring)+"-"+.case+"-"+(("00"+(.trial|tostring))[-2:]))); "pair label") |
require(([$ops[].scope]|unique|length)==1200; "scope cardinality") |
require(all($ops[]; .http_body_equivalent_bytes==(.request_body_equivalent_bytes+.response_body_equivalent_bytes) and .serialized_batches==1 and .protocol_request_bytes>0 and .protocol_response_bytes>0); "body arithmetic or batches") |
require(all($ops[]; .request_body_equivalent_bytes==(.protocol_request_bytes+4*.request_frames+12*.serialized_batches) and .response_body_equivalent_bytes==(.protocol_response_bytes+4*.request_frames+28*.serialized_batches)); "independent envelope arithmetic or unexpected terminal prefix") |
require(all($ops[]; .operation_cpu_micros>=.exchange_cpu_micros and .exchange_cpu_micros>=.receiver_cpu_micros and .operation_wall_micros>=.exchange_wall_micros and .exchange_wall_micros>=.receiver_wall_micros); "nested timing boundary") |
($ops|group_by(.pair)) as $pairs |
require(($pairs|length)==240; "pair cardinality") |
([ $pairs[] | . as $pair |
   ([$pair[]|select(.phase=="transfer")]) as $targets |
   ([$pair[]|select(.phase=="seed")]) as $seeds |
   ($targets[]|select(.arm=="raw")) as $raw |
   ($targets[]|select(.arm=="adaptive")) as $adaptive |
   ([$targets[]|select(.arm!="adaptive")|.http_body_equivalent_bytes]|min) as $minimum |
   {pair:$raw.pair,
    valid: (($targets|length)==5 and ($targets|map(.arm)|sort)==($reg.arms|sort)
      and all($targets[]; .object==$raw.object and .base==$raw.base and .object_bytes==$raw.object_bytes)
      and (if $raw.base==null then ($seeds|length)==0 else
        ($seeds|length)==5 and all($seeds[]; .actual_mode==0 and .base==null and .object==$raw.base and .verified_bytes==$raw.object_bytes)
        end)
      and all($pair|group_by(.arm)[]; ([.[].scope]|unique|length)==1)
      and $adaptive.http_body_equivalent_bytes==$minimum
      and $adaptive.selection.expected_http_body_bytes==$minimum
      and $adaptive.selection.selected_mode==$adaptive.actual_mode
      and $adaptive.selection.exact_repeat_probe==false
      and ($adaptive.selection.candidates|map(.mode)|unique|length)==($adaptive.selection.candidates|length)
      and ($adaptive.selection.candidates|map(.http_body_bytes)|min)==$minimum
      and all($targets[]|select(.arm=="raw" or .arm=="zstd");
        . as $fixed | any($adaptive.selection.candidates[];.mode==$fixed.actual_mode and .http_body_bytes==$fixed.http_body_equivalent_bytes))
      and all($targets[]|select(.actual_mode==3 or .actual_mode==4); . as $fixed |
        any($adaptive.selection.candidates[];.mode==$fixed.actual_mode and .http_body_bytes==$fixed.http_body_equivalent_bytes))),
    minimum_body:$minimum}
 ]) as $pair_checks |
require(all($pair_checks[];.valid); "paired identity, seed, or selector minimum mismatch") |
require(([$ops[]|select(.phase=="transfer")|.object]|unique|length)==240; "fixture distinctness") |
require(([$cleanup[].label]|unique|length)==1200 and all($cleanup[];.status=="removed-after-all-phase-reopen-checks"); "cleanup identity/status") |
require(([$cleanup[].label]|sort)==([$ops[]|(.pair+"-"+.arm)]|unique|sort); "cleanup coverage") |
require(all($cleanup[]; .cache==($completion[0].cache_root_retained_with_owner_marker+"/"+.label)); "cleanup path binding") |
require($completion[0].operations_exact==1800 and $completion[0].status=="exact" and $acceptance[0].pairs==240 and $acceptance[0].adaptive_equals_best_fixed_body_pairs==240 and $acceptance[0].all_fixture_and_seed_identities_match==true; "terminal claims") |
["http_body_equivalent_bytes","operation_wall_micros","operation_cpu_micros","receiver_cpu_micros","receiver_wall_micros","exchange_cpu_micros","exchange_wall_micros"] as $metrics |
require(([$summaries[]|[.size,.case,.arm]]|unique|length)==40; "duplicate summary identity") |
([ $summaries[] as $summary |
   ([$ops[]|select(.phase=="transfer" and cell(.;$summary))]) as $rows |
   ([$ops[]|select(.phase=="seed" and cell(.;$summary))]) as $seeds |
   ($rows | group_by(.actual_mode) | map({key:(.[0].actual_mode|tostring),value:length}) | from_entries) as $modes |
   {case:$summary.case,size:$summary.size,arm:$summary.arm,count:($rows|length),selected_modes:$modes} as $header |
   ([ $metrics[] as $field |
       ($rows|map(.[$field])) as $values |
       ([ $rows[] as $row | ($ops[]|select(.phase=="transfer" and .arm=="raw" and .pair==$row.pair)) as $raw |
          select($row[$field]>0) | $raw[$field]/$row[$field] ]) as $ratios |
       ([ $rows[] as $row | $seeds[] | select(.pair==$row.pair) | .[$field] ]) as $seed_values |
       ([ $rows[] as $row | $seeds[] | select(.pair==$row.pair) | .[$field]+$row[$field] ]) as $lifecycle |
       {key:$field,value:stats($values)},
       (if ($ratios|length)==30 then {key:("paired_raw_over_candidate_"+$field),value:stats($ratios)} else empty end),
       (if ($seeds|length)>0 then {key:("seed_"+$field),value:stats($seed_values)}, {key:("seed_plus_transfer_"+$field),value:stats($lifecycle)} else empty end)
    ] | from_entries) as $distributions |
   ($header+$distributions+{event:"summary"}) as $recomputed |
   require($recomputed==$summary; "summary mismatch for \($summary.size)/\($summary.case)/\($summary.arm)") |
   $recomputed
 ]) as $recomputed |
{schema:"spiralcore.selector.independent-numeric-verification.v1",status:"PASS",
 row_count:($all|length),exact_operations:($ops|length),exact_compared_bytes:([$ops[].verified_bytes]|add),
 target_operations:([$ops[]|select(.phase=="transfer")]|length),paid_seed_operations:([$ops[]|select(.phase=="seed")]|length),
 verified_pairs:($pair_checks|length),adaptive_equals_best_fixed_pairs:([$pair_checks[]|select(.valid)]|length),
 verified_summaries:($recomputed|length),verified_cleanup_rows:($cleanup|length),
 source_revision:$reg.claimed_source_revision,executable_sha256:$reg.executable_sha256,
 mode_totals:([$ops[]|select(.phase=="transfer" and .arm=="adaptive")]|group_by(.actual_mode)|map({mode:.[0].actual_mode,count:length})),
 summaries:$recomputed,
 limitations:["Recomputes recorded receipt arithmetic, pairing, counts, schedule and statistics; does not independently rerun all byte reconstruction or regenerate every fixture.","Actual reopen comparisons executed in the pinned harness before cache deletion; this checker validates those receipts and separately recorded source/executable custody.","Local body-equivalent experiment; no new WAN, pure send-time, energy or peak-memory claim."]}
