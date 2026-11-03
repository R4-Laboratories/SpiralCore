#!/usr/bin/env bash
set -euo pipefail
root=${1:?owned coverage root}
awk '
 /^SF:/ {
   file=substr($0,4); sub(/^.*\/crates\//,"crates/",file);
   keep=(file ~ /^crates\/spiralcore-mvm-adapter\/src\// || file=="crates/spiralcore-mvm-live/src/config.rs" || file=="crates/spiralcore-mvm-live/src/campaign.rs"); next;
 }
 keep && /^DA:/ {
   split(substr($0,4),a,","); key=file SUBSEP a[1];
   if (!(key in hits) || a[2]+0>hits[key]) hits[key]=a[2]+0;
 }
 END { OFS="\t"; for(key in hits){split(key,a,SUBSEP);print a[1],a[2],hits[key]}}
' "$root/per-binary/combined.lcov" | sort -k1,1 -k2,2n > "$root/line-hits.tsv"
awk 'BEGIN{FS="\t"} {
 if($1!=last){if(last!="")print "end_of_record";print "SF:"$1;last=$1}
 print "DA:"$2","$3;
} END{if(last!="")print "end_of_record"}' "$root/line-hits.tsv" > "$root/source.lcov"
awk -f "$root/summarize.awk" "$root/changed-lines.tsv" "$root/test-cutoffs.tsv" "$root/source.lcov" > "$root/lines-unsorted.tsv"
head -1 "$root/lines-unsorted.tsv" > "$root/lines.tsv"
tail -n +2 "$root/lines-unsorted.tsv" | sort >> "$root/lines.tsv"
awk 'BEGIN{FS="\t"} NR>1 {if($1~/^crates\/spiralcore-mvm-adapter\//){n+=$4;c+=$5;dn+=$8;dc+=$9} else{ln+=$4;lc+=$5;ldn+=$8;ldc+=$9}} END{printf "adapter production %d/%d (%.4f%%); changed %d/%d (%.4f%%)\n",c,n,100*c/n,dc,dn,100*dc/dn;printf "selected live modules production %d/%d; changed %d/%d\n",lc,ln,ldc,ldn;if(n==0||100*c/n<80)exit 1}' "$root/lines.tsv"
