#!/bin/bash
# Correct only the missed child custody/export; keep the first collection intact.
set -euo pipefail
root=/var/tmp/spiralcore-object-finish-20261103
out=$root/coverage
repair=$out/probe-custody-correction
export PATH=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin:/usr/bin:/bin
llvm=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
test ! -e "$repair"
mkdir -m 700 "$repair"
mkdir "$repair/profiles" "$repair/parent-profiles" "$repair/child-profiles"
parent=$(jq -r '.[]|select(.target=="probe")|.retained_executable' "$out/executed-binaries.json")
child=/var/tmp/spiralcore-mvm-service-20261102/target/release/spiralcore-gateway-probe
jq 'select(.reason=="compiler-artifact" and .target.name=="spiralcore-gateway-probe" and .profile.test==false)' "$out/build.jsonl" > "$repair/child-build.json"
jq -e '.fresh==false and .executable!=null' "$repair/child-build.json" >/dev/null
cp -p "$child" "$out/bin/retained-instrumented-probe-child"
chmod 500 "$out/bin/retained-instrumented-probe-child"
cmp "$child" "$out/bin/retained-instrumented-probe-child"
sha256sum "$child" "$out/bin/retained-instrumented-probe-child" "$parent" > "$repair/binaries-before.sha256"
# The parent has a compile-time CARGO_BIN_EXE path. Match its actual child to
# the protected copy before and after execution; every other parent was already
# protected before its original run. Parent PID identifies disjoint profiles.
LLVM_PROFILE_FILE="$repair/profiles/%m-%p.profraw" "$parent" --test-threads=1 > "$repair/tests.txt" 2> "$repair/tests.stderr" &
parent_pid=$!
printf '%s\n' "$parent_pid" > "$repair/parent-pid.txt"
wait "$parent_pid"
test ! -s "$repair/tests.stderr"
grep -Eq '^test result: ok\. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' "$repair/tests.txt"
for profile in "$repair"/profiles/*.profraw; do
  case "$profile" in
    *-"$parent_pid".profraw) cp "$profile" "$repair/parent-profiles/" ;;
    *) cp "$profile" "$repair/child-profiles/" ;;
  esac
done
for role in parent child; do
  object=$parent
  test "$role" != child || object=$out/bin/retained-instrumented-probe-child
  "$llvm/llvm-profdata" merge -sparse "$repair/$role-profiles"/*.profraw -o "$repair/$role.profdata"
  "$llvm/llvm-cov" export -format=lcov -instr-profile="$repair/$role.profdata" -object "$object" > "$repair/$role.lcov" 2> "$repair/$role-export.stderr"
  test ! -s "$repair/$role-export.stderr"
done
sha256sum -c "$repair/binaries-before.sha256" > "$repair/binaries-after-verification.txt"
mkdir "$out/final"
for file in "$out"/*/source.lcov; do
  case "$file" in "$out"/probe-*/source.lcov) continue;; esac
  cat "$file"
done > "$out/final/combined.lcov"
cat "$repair/parent.lcov" "$repair/child.lcov" >> "$out/final/combined.lcov"
awk '
 /^SF:/ {f=substr($0,4);sub(/^.*\/crates\//,"crates/",f);next}
 /^DA:/ && f ~ /^crates\/spiralcore-gateway\/src\// {split(substr($0,4),a,",");k=f SUBSEP a[1];if(!(k in hits)||a[2]+0>hits[k])hits[k]=a[2]+0}
 END{OFS="\t";for(k in hits){split(k,a,SUBSEP);print a[1],a[2],hits[k]}}
' "$out/final/combined.lcov" | sort -k1,1 -k2,2n > "$out/final/line-hits.tsv"
awk '
 /^SF:/ {f=substr($0,4);sub(/^.*\/crates\//,"crates/",f);next}
 /^BRDA:/ && f ~ /^crates\/spiralcore-gateway\/src\// {split(substr($0,6),a,",");k=f SUBSEP a[1] SUBSEP a[2] SUBSEP a[3];if(!(k in hits)||a[4]+0>hits[k])hits[k]=a[4]+0}
 END{OFS="\t";for(k in hits){split(k,a,SUBSEP);print a[1],a[2],a[3],a[4],hits[k]}}
' "$out/final/combined.lcov" | sort -k1,1 -k2,2n > "$out/final/branch-hits.tsv"
cp "$out"/{production-cutoffs.tsv,changed-source.diff,derive.sh,test-counts.json} "$out/final/"
bash "$out/final/derive.sh" > "$out/final/derive-output.txt" 2>&1
cd "$root/source"
sha256sum -c "$out/source-before.sha256" > "$out/final/source-after-verification.txt"
sha256sum -c ../validation/finish-final2-before.sha256 > "$out/final/qualified-source-after-verification.txt"
sha256sum -c "$out/normal-binaries.sha256" > "$out/final/normal-binaries-after-verification.txt"
printf '%s\n' 'Accepted:13 original protected parent runs + protected probe parent/child rerun;51 tests total,3 replayed only for child custody. Original probe mixed export excluded from final union.' > "$out/final/custody.txt"
cat "$out/final/derive-output.txt"
