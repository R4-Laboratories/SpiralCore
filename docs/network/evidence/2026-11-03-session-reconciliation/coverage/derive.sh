#!/usr/bin/env bash
# Union executable line coverage, using the new side of the pinned source diff.
set -euo pipefail
cd "$(dirname "$0")"
awk '
 FILENAME==ARGV[1] {cutoff[$1]=$2; next}
 FILENAME==ARGV[2] {
   if ($0 ~ /^\+\+\+ b\//) {file=substr($0,7); next}
   if ($0 ~ /^@@ /) {
     n=split($3,a,","); start=substr(a[1],2)+0; count=n>1?a[2]+0:1;
     for(i=start;i<start+count;i++) changed[file SUBSEP i]=1;
   }
   next
 }
 ($1 in cutoff) && $2+0<cutoff[$1] {
   OFS="\t"; print $1,$2,$3,(($1 SUBSEP $2) in changed ? 1 : 0)
 }
' production-cutoffs.tsv changed-source.diff line-hits.tsv > focused-lines.tsv
awk '
 {all[$1]++; total++; if($3+0>0) {covered[$1]++; hits++}
  if($4==1) {changed[$1]++; changed_total++; if($3+0>0) {changed_hit[$1]++; changed_hits++}}}
 END {OFS="\t";
  print "scope","file","covered_executable_lines","total_executable_lines","percent";
  for(f in all) {
    print "all-production-in-file",f,covered[f]+0,all[f],sprintf("%.2f",100*(covered[f]+0)/all[f]);
    if(changed[f]>0) print "changed-production",f,changed_hit[f]+0,changed[f],sprintf("%.2f",100*(changed_hit[f]+0)/changed[f]);
  }
  print "changed-production-total","reviewed-reconciliation",changed_hits,changed_total,sprintf("%.2f",100*changed_hits/changed_total);
  print "all-production-total","reviewed-reconciliation",hits,total,sprintf("%.2f",100*hits/total);
 }
' focused-lines.tsv > summary.tsv
awk '$3==0 && $4==1' focused-lines.tsv > uncovered-changed-lines.tsv
awk '$3==0' focused-lines.tsv > uncovered-production-lines.tsv
awk '
 FNR==NR {cutoff[$1]=$2; next}
 ($1 in cutoff) && $2+0<cutoff[$1] {print}
' production-cutoffs.tsv branch-hits.tsv > focused-branches.tsv
awk 'BEGIN {OFS="\t"} {total++; if($5+0>0) hit++} END {
 print "scope","covered_mapped_branches","total_mapped_branches","percent";
 if(total>0) print "five-production-files",hit+0,total,sprintf("%.2f",100*(hit+0)/total);
 else print "five-production-files","NA",0,"NA-no-mapped-branches";
}' focused-branches.tsv > branch-summary.tsv
jq -Rn '[inputs|split("\t")|select(.[0]!="scope")|{scope:.[0],file:.[1],covered_executable_lines:(.[2]|tonumber),total_executable_lines:(.[3]|tonumber),percent:(.[4]|tonumber)}]' < summary.tsv > summary.json
awk -F '\t' '$1=="changed-production-total" {if($5+0<80) exit 1; found=1} END {if(!found) exit 1}' summary.tsv
cat summary.tsv
cat branch-summary.tsv
