#!/usr/bin/env bash
# Real package preparation under controlled hash/path tool availability.
set -euo pipefail
[[ $# == 1 ]] || { printf 'usage: test-prepare.sh EXISTING_MVM_GIT_REPOSITORY\n' >&2; exit 2; }
repo=$(cd -- "$1" && pwd -P)
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/spiralcore-prepare-test.XXXXXXXX")
scratch=$(cd -- "$scratch" && pwd -P)
trap 'rm -r -- "$scratch"' EXIT
if command -v sha256sum >/dev/null; then
    real_hash=("$(command -v sha256sum)")
else
    real_hash=("$(command -v shasum)" -a 256)
fi
printf -v hash_exec '%q ' "${real_hash[@]}"
# The fallback fixture computes real SHA-256, rather than echoing expected hashes.
for variant in sha256sum shasum missing windows; do
    bin="$scratch/$variant-bin"
    mkdir -- "$bin"
    for tool in git tar df awk dirname basename mkdir cp; do
        ln -s "$(command -v "$tool")" "$bin/$tool"
    done
    if [[ "$variant" != missing ]]; then
        selected=sha256sum
        [[ "$variant" != shasum ]] || selected=shasum
        cat > "$bin/$selected" <<HASH
#!/bin/bash
printf '%s\\n' '$selected' >> "\$SPIRALCORE_PREP_TEST_MARKER"
if [[ \${1-} == -a && \${2-} == 256 ]]; then shift 2; fi
exec $hash_exec "\$@"
HASH
        chmod +x "$bin/$selected"
    fi
    if [[ "$variant" == windows ]]; then
        cat > "$bin/cygpath" <<'CYGPATH'
#!/bin/bash
[[ $# == 2 && $1 == -m ]] || exit 2
printf 'C:%s\n' "$2"
CYGPATH
        chmod +x "$bin/cygpath"
    fi
    output="$scratch/$variant output with spaces"
    marker="$scratch/$variant.hash-used"
    # A global CRLF preference must not alter archived LF source or patch hashes.
    printf '[core]\n\tautocrlf = true\n' > "$scratch/global-gitconfig"
    if PATH="$bin" SPIRALCORE_PREP_TEST_MARKER="$marker" \
       GIT_CONFIG_GLOBAL="$scratch/global-gitconfig" \
       /bin/bash "$here/prepare.sh" "$repo" "$output" > "$scratch/$variant.stdout" 2> "$scratch/$variant.stderr"; then rc=0; else rc=$?; fi
    if [[ "$variant" == missing ]]; then
        [[ $rc != 0 && ! -e "$output" ]]
        grep -q 'SHA-256 tool' "$scratch/$variant.stderr"
        continue
    fi
    [[ $rc == 0 ]] || { cat "$scratch/$variant.stderr" >&2; exit 1; }
    [[ -s "$marker" && -f "$output/patch.toml" ]]
    grep -Fxq '[patch."https://github.com/tinylabscom/mvm"]' "$output/patch.toml"
    grep -Fxq '[patch."https://github.com/Casey-allard/mvm"]' "$output/patch.toml"
    [[ $(grep -c '^mvm-' "$output/patch.toml") == 18 ]]
    [[ $(grep '^mvm-' "$output/patch.toml" | sort -u | wc -l) -eq 9 ]]
    [[ $(git -C "$output/source" config core.autocrlf) == false ]]
    [[ $(git -C "$output/source" config core.eol) == lf ]]
    [[ $(wc -l < "$marker") -ge 2 ]]
    if [[ "$variant" == windows ]]; then
        [[ $(grep -c 'path = "C:/' "$output/patch.toml") == 18 ]]
        grep -Fq "path = \"C:$output/source/crates/mvm-core\"" "$output/patch.toml"
    else
        grep -Fq "path = \"$output/source/crates/mvm-core\"" "$output/patch.toml"
    fi
    [[ $(grep -c 'crates/.*: OK' "$scratch/$variant.stderr") == 14 ]]
    # Release each verified disposable source before the next extraction.
    rm -r -- "$output"
done
printf 'PASS: real three-patch preparation, both hash choices, missing-tool refusal, LF and native-path mapping\n'
