#!/usr/bin/env bash
set -euo pipefail
umask 077
root=/var/tmp/spiralcore-installed-owner-20261103
toolchain=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu
export PATH="$toolchain/bin:$PATH"
export CARGO_TARGET_DIR="$root/handoff-coverage-target"
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER CARGO_ENCODED_RUSTFLAGS LLVM_PROFILE_FILE
export RUSTFLAGS=-Cinstrument-coverage
out="$root/handoff-coverage"
mkdir "$out"
mkdir "$out/bin" "$out/profiles" "$out/exports"
cd "$root/source"
module=crates/spiralcore-mvm-live/src/installed_owner.rs
manifest=crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml
printf 'source_commit=88f4ea56eb3b7409d09af77faef916f3d9f157d8\n' > "$out/source-identity.txt"
printf '%s  %s\n' fc0b3dd44256da7bcfd7cdc793e77f0312ce421d365d6e295fda68ce17225279 "$module" | sha256sum -c - > "$out/product-pin-check.txt"
sha256sum "$module" crates/spiralcore-mvm-live/src/installed_owner/tests.rs "$manifest" "${manifest%Cargo.toml}Cargo.lock" "${manifest%Cargo.toml}tests/lifecycle.rs" > "$out/source-before.sha256"
"$toolchain/bin/cargo" test --locked --manifest-path "$manifest" --no-run --message-format=json > "$out/build.json" 2> "$out/build.stderr"
jq -r 'select(.reason == "compiler-artifact" and .profile.test == true and .executable != null) | .executable' "$out/build.json" > "$out/executables.txt"
test "$(wc -l < "$out/executables.txt")" -eq 2
llvm="$toolchain/lib/rustlib/x86_64-unknown-linux-gnu/bin"
while IFS= read -r executable; do
  name=$(basename "$executable")
  cp "$executable" "$out/bin/$name"
  chmod 500 "$out/bin/$name"
  sha256sum "$out/bin/$name" >> "$out/executed-binaries.sha256"
  mkdir "$out/profiles/$name"
  LLVM_PROFILE_FILE="$out/profiles/$name/%m-%p.profraw" "$out/bin/$name" --test-threads=1 > "$out/$name.tests.txt" 2> "$out/$name.tests.stderr"
  find "$out/profiles/$name" -name '*.profraw' -type f > "$out/profiles/$name/inputs.txt"
  "$llvm/llvm-profdata" merge -sparse -f "$out/profiles/$name/inputs.txt" -o "$out/profiles/$name/merged.profdata" 2> "$out/profiles/$name/merge.stderr"
  "$llvm/llvm-cov" export --format=lcov --instr-profile="$out/profiles/$name/merged.profdata" "$out/bin/$name" > "$out/exports/$name.lcov" 2> "$out/exports/$name.stderr"
done < "$out/executables.txt"
sha256sum "$module" crates/spiralcore-mvm-live/src/installed_owner/tests.rs "$manifest" "${manifest%Cargo.toml}Cargo.lock" "${manifest%Cargo.toml}tests/lifecycle.rs" > "$out/source-after.sha256"
cmp "$out/source-before.sha256" "$out/source-after.sha256"
awk '/#\[cfg\(test\)\]/{print NR; getline; print NR; if ($0 ~ /#\[path/) {getline; print NR}}' "$module" > "$out/excluded-test-only-lines.txt"
awk 'FNR==NR {excluded[$1]=1; next} /^SF:/ {active=($0 ~ /\/installed_owner\.rs$/); next} active && /^DA:/ {split(substr($0,4),a,","); if (!(a[1] in excluded) && (!(a[1] in hits) || a[2]>hits[a[1]])) hits[a[1]]=a[2]} END {for (line in hits) print line "\t" hits[line]}' "$out/excluded-test-only-lines.txt" "$out"/exports/*.lcov | sort -n > "$out/line-hits.tsv"
awk '$2==0 {print $1}' "$out/line-hits.tsv" > "$out/uncovered-lines.txt"
awk '{mapped++; if($2>0) covered++} END {printf "mapped_lines=%d\ncovered_lines=%d\nuncovered_lines=%d\nline_coverage_percent=%.2f\n",mapped,covered,mapped-covered,100*covered/mapped; if (covered/mapped < .8) exit 1}' "$out/line-hits.tsv" > "$out/summary.txt"
printf 'branch_coverage=NA (no branch records emitted for this scope)\n' >> "$out/summary.txt"
{
  printf 'TN:installed-owner-union-maxhits\nSF:%s\n' "$module"
  awk '{printf "DA:%s,%s\n",$1,$2}' "$out/line-hits.tsv"
  printf 'end_of_record\n'
} > "$out/module.lcov"
find "$out" -name '*.stderr' -type f -printf '%s %P\n' > "$out/stderr-sizes.txt"
sha256sum -c "$out/executed-binaries.sha256" > "$out/executed-binary-after-check.txt"
cat "$out/summary.txt"
cp "$root/holder-handoff-changed-lines.txt" "$out/changed-lines.txt"
awk 'FNR==NR {changed[$1]=1; next} $1 in changed {print}' "$out/changed-lines.txt" "$out/line-hits.tsv" > "$out/changed-line-hits.tsv"
awk '$2==0 {print $1}' "$out/changed-line-hits.tsv" > "$out/changed-uncovered-lines.txt"
awk '{mapped++; if($2>0) covered++} END {printf "changed_mapped_lines=%d\nchanged_covered_lines=%d\nchanged_uncovered_lines=%d\nchanged_line_coverage_percent=%.2f\n",mapped,covered,mapped-covered,mapped?100*covered/mapped:0; if (!mapped || covered/mapped < .8) exit 1}' "$out/changed-line-hits.tsv" > "$out/changed-summary.txt"
awk '/^BRDA:/ {n++} END {print n+0}' "$out"/exports/*.lcov > "$out/branch-record-count.txt"
test "$(cat "$out/branch-record-count.txt")" -eq 0
cat "$out/changed-summary.txt"
