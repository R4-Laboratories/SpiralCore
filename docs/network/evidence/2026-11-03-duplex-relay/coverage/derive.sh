#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
# New-side -U0 diff 7c480a9..f964c03, executable DA lines only. The parent
# orchestrator test helper is cfg(test) at 299..320; its test suffix starts592.
awk '
BEGIN {
  cutoff["crates/spiralcore-mvm-adapter/src/live.rs"]=1000000;
  cutoff["crates/spiralcore-mvm-live/src/orchestrator.rs"]=592;
  cutoff["crates/spiralcore-mvm-live/src/orchestrator/duplex.rs"]=1000000;
}
FNR==NR {
  if ($0 ~ /^\+\+\+ b\//) { file=substr($0,7); next }
  if ($0 ~ /^@@ /) {
    n=split($3,a,","); start=substr(a[1],2)+0; count=n>1?a[2]+0:1;
    for(i=start;i<start+count;i++) changed[file SUBSEP i]=1;
  }
  next
}
($1 in cutoff) && $2+0<cutoff[$1] && !($1=="crates/spiralcore-mvm-live/src/orchestrator.rs" && $2>=299 && $2<=320) {
  OFS="\t"; print $1,$2,$3,(($1 SUBSEP $2) in changed ? 1 : 0)
}
' changed-source.diff line-hits.tsv > focused-lines.tsv
awk '
{ all[$1]++; all_total++; if($3+0>0) { covered[$1]++; all_covered++ }
  if($4==1) { changed[$1]++; total++; if($3+0>0) { hit[$1]++; hits++ } }
  if($1=="crates/spiralcore-mvm-adapter/src/live.rs" && $2>=159 && $2<=195) {
    eof_total++; if($3+0>0) eof_hit++;
  }
}
END {
  OFS="\t";
  print "scope","file","covered_executable_lines","total_executable_lines","percent";
  for(f in all) {
    print "all-production-in-file",f,covered[f]+0,all[f],sprintf("%.2f",100*(covered[f]+0)/all[f]);
    if(changed[f]>0) print "changed-production",f,hit[f]+0,changed[f],sprintf("%.2f",100*(hit[f]+0)/changed[f]);
  }
  print "changed-production-total","three-reviewed-files",hits,total,sprintf("%.2f",100*hits/total);
  print "all-production-total","three-reviewed-files",all_covered,all_total,sprintf("%.2f",100*all_covered/all_total);
  print "EOF-helper-lines","live.rs:159-195",eof_hit,eof_total,sprintf("%.2f",100*eof_hit/eof_total);
}
' focused-lines.tsv > summary.tsv
awk '$3==0 && $4==1 {print}' focused-lines.tsv > uncovered-changed-lines.tsv
awk '$1=="crates/spiralcore-mvm-adapter/src/live.rs" && $2>=159 && $2<=195 {print}' branch-hits.tsv > eof-helper-branches.tsv
awk 'BEGIN {OFS="\t"} {total++; if($5+0>0) hit++} END {
  print "scope","covered_mapped_branches","total_mapped_branches","percent";
  if(total>0) print "EOF-helper",hit+0,total,sprintf("%.2f",100*(hit+0)/total);
  else print "EOF-helper","NA",0,"NA-no-mapped-branches";
}' eof-helper-branches.tsv > branch-summary.tsv
cat summary.tsv
cat branch-summary.tsv
