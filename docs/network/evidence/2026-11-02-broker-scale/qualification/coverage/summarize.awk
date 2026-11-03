BEGIN { FS="\t"; OFS="\t" }
FILENAME==ARGV[1] { changed[$1 SUBSEP $2]=1; next }
FILENAME==ARGV[2] { cutoff[$1]=$2; next }
/^SF:/ { file=substr($0,4); sub(/^.*\/crates\//,"crates/",file); next }
/^DA:/ && file ~ /^crates\/spiralcore-mvm-(adapter|live)\/src\// {
    split(substr($0,4),a,","); key=file SUBSEP a[1];
    if (!(key in hits) || a[2]+0 > hits[key]) hits[key]=a[2]+0;
}
END {
    for (key in hits) {
        split(key,a,SUBSEP); f=a[1]; line=a[2]+0; covered=(hits[key]>0);
        total[f]++; covered_total[f]+=covered;
        if (changed[key]) { added[f]++; covered_added[f]+=covered }
        if (!cutoff[f] || line<cutoff[f]) {
            prod[f]++; covered_prod[f]+=covered;
            if (changed[key]) { added_prod[f]++; covered_added_prod[f]+=covered }
        }
    }
    print "file","all_count","all_covered","production_count","production_covered","changed_count","changed_covered","changed_production_count","changed_production_covered";
    for (f in total) print f,total[f],covered_total[f]+0,prod[f]+0,covered_prod[f]+0,added[f]+0,covered_added[f]+0,added_prod[f]+0,covered_added_prod[f]+0;
}
