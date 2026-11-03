#!/usr/bin/env bash
set -euo pipefail
root=${1:?usage: run-per-binary.sh LOCAL_COVERAGE_ROOT LLVM_BIN}
llvm=${2:?usage: run-per-binary.sh LOCAL_COVERAGE_ROOT LLVM_BIN}
mkdir "$root/per-binary"
cat "$root/adapter-objects.txt" "$root/live-objects.txt" | sort -u > "$root/per-binary/objects.txt"
while IFS= read -r binary; do
    name=$(basename "$binary")
    dir="$root/per-binary/$name"
    mkdir -p "$dir/profiles"
    LLVM_PROFILE_FILE="$dir/profiles/%m-%p.profraw" "$binary" --test-threads=1 > "$dir/tests.txt" 2> "$dir/tests.stderr"
    "$llvm/llvm-profdata" merge -sparse "$dir"/profiles/*.profraw -o "$dir/current.profdata"
    "$llvm/llvm-cov" export -format=lcov -instr-profile="$dir/current.profdata" -object "$binary" > "$dir/source.lcov" 2> "$dir/export.stderr"
    printf 'PASS %s: ' "$name"
    sed -n '/^test result:/p' "$dir/tests.txt"
done < "$root/per-binary/objects.txt"
cat "$root"/per-binary/*/source.lcov > "$root/per-binary/combined.lcov"
printf 'Exports with nonempty warning/error logs:\n'
find "$root/per-binary" -name export.stderr -size +0c -print
