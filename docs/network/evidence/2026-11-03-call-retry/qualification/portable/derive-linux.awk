BEGIN {
    FS="\t"; OFS="\t";
    print "scope","target","passed","failed","ignored","measured","filtered";
}
/Running / {
    target=substr($0,index($0,"Running ")+8);
    sub(/ \(.*/,"",target);
}
/test result: ok\./ {
    summary=substr($0,index($0,"test result: ok.")+17);
    split(summary,a," ");
    if ($2=="Linux host, canonical envelope, recovery and packing tests") scope="release-live-all-targets";
    else if ($2=="Shared object-v1 and canonical mailbox conformance") {
        scope=target ~ /v1_contract/ ? "shared-object-v1" : "shared-adapter-all-targets";
    } else scope=$2;
    print scope,target,a[1]+0,a[3]+0,a[5]+0,a[7]+0,a[9]+0;
}
