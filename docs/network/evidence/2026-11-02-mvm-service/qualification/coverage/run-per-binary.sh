#!/usr/bin/env bash
set -euo pipefail
root=/Users/casey.allard/spiralcore/local/mvm-service-20261102/coverage-final
llvm=/Users/casey.allard/.rustup/toolchains/1.97.1-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin
mkdir "$root/per-binary"
cat "$root/adapter-final-objects.txt" "$root/live-final-objects.txt" | sort -u > "$root/per-binary/objects.txt"
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
printf 'Exports with nonempty warning/error logs: '
find "$root/per-binary" -name export.stderr -size +0c -print
