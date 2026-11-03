#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
# Cutoffs are the first cfg(test) line at exact product revision 7531985.
# Denominators are executable DA lines, not source-text lines or branches.
awk '
BEGIN {
  cutoff["crates/spiralcore-mvm-live/src/host_budget.rs"]=328;
  cutoff["crates/spiralcore-mvm-live/src/orchestrator.rs"]=683;
  cutoff["crates/spiralcore-mvm-live/src/campaign/canonical.rs"]=432;
}
FNR==NR {
  if ($0 ~ /^\+\+\+ b\//) { file=substr($0,7); next }
  if ($0 ~ /^@@ /) {
    n=split($3,a,","); start=substr(a[1],2)+0; count=n>1?a[2]+0:1;
    for(i=start;i<start+count;i++) changed[file SUBSEP i]=1;
  }
  next
}
($1 in cutoff) && $2+0<cutoff[$1] {
  OFS="\t"; print $1,$2,$3,(($1 SUBSEP $2) in changed ? 1 : 0)
}
' changed-source.diff line-hits.tsv > focused-lines.tsv
awk '
{ all[$1]++; all_total++; if($3+0>0) { covered[$1]++; all_covered++ }
  if($4==1) { changed[$1]++; total++; if($3+0>0) { hit[$1]++; hits++ } }
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
}
' focused-lines.tsv > summary.tsv
awk '$3==0 && $4==1 {print}' focused-lines.tsv > uncovered-changed-lines.tsv
cat summary.tsv
