#!/usr/bin/env bash
# Prepare an isolated, checked source tree. Never edit the supplied checkout.
set -euo pipefail
umask 077

fail() { printf 'prepare: %s\n' "$*" >&2; exit 1; }
[[ $# -eq 2 ]] || fail 'usage: prepare.sh EXISTING_PINNED_MVM_CHECKOUT NEW_OUTPUT_DIRECTORY'
for tool in git tar shasum df awk; do
    command -v "$tool" >/dev/null || fail "required tool is missing: $tool"
done
bundle_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
input_dir=$(cd -- "$1" && pwd -P)
output_parent=$(cd -- "$(dirname -- "$2")" && pwd -P)
output_dir="$output_parent/$(basename -- "$2")"
case "$output_dir" in *$'\n'*|*$'\r'*|*$'\t'*) fail 'control characters in output paths are unsupported';; esac
[[ ! -e "$output_dir" && ! -L "$output_dir" ]] || fail 'output directory must be new'
base=3104a12c5b071a6851323fa8115d2afaf3f0bb8b
[[ $(git -C "$input_dir" rev-parse HEAD) == "$base" ]] || fail 'input checkout is not at the exact pinned revision'
(cd -- "$bundle_dir" && shasum -a 256 -c bundle.sha256 >&2)

# Keep the user reserve plus twice the committed file bytes for extraction.
source_bytes=$(git -C "$input_dir" ls-tree -rl "$base" | awk '{sum += $4} END {printf "%.0f", sum}')
available_kib=$(df -Pk "$output_parent" | awk 'END {print $4}')
required_kib=$((4194304 + (source_bytes * 2 + 1023) / 1024))
[[ "$available_kib" -ge "$required_kib" ]] || fail 'preparation would cross the 4 GiB free-space reserve'

mkdir -- "$output_dir"
source_dir="$output_dir/source"
mkdir -- "$source_dir"
trap 'printf "prepare: failed; owned partial output retained at %s\n" "$output_dir" >&2' ERR
# Archive the commit, never the input working tree: local edits/private files stay put.
git -C "$input_dir" archive --format=tar "$base" | tar -xf - -C "$source_dir"
git -c init.templateDir= init --quiet --initial-branch=codex/spiralcore-mvm-patch "$source_dir"
git -C "$source_dir" apply --check --whitespace=error-all "$bundle_dir/0001-experimental-mailbox-vsock.patch"
git -C "$source_dir" apply --whitespace=error-all "$bundle_dir/0001-experimental-mailbox-vsock.patch"
(cd -- "$source_dir" && shasum -a 256 -c "$bundle_dir/patched-files.sha256" >&2)

toml_string() {
    local value=$1
    value=${value//\\/\\\\}
    value=${value//\"/\\\"}
    printf '"%s"' "$value"
}
config="$output_dir/patch.toml"
{
    printf '[patch."https://github.com/tinylabscom/mvm"]\n'
    for crate in mvm-core mvm-contract mvm-runtime mvm-build mvm-fs mvm-agentd; do
        printf '%s = { path = ' "$crate"
        toml_string "$source_dir/crates/$crate"
        printf ' }\n'
    done
} > "$config"
printf '%s\n' "$base" > "$output_dir/base-revision"
cp -- "$bundle_dir/provenance.json" "$output_dir/provenance.json"
trap - ERR
printf '%s\n' "$config"
