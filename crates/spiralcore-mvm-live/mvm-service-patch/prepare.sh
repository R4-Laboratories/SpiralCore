#!/usr/bin/env bash
# Prepare an isolated, checked source tree. Never edit the supplied checkout.
set -euo pipefail
umask 077

fail() { printf 'prepare: %s\n' "$*" >&2; exit 1; }
[[ $# -eq 2 ]] || fail 'usage: prepare.sh EXISTING_PINNED_MVM_CHECKOUT NEW_OUTPUT_DIRECTORY'
for tool in git tar df awk; do
    command -v "$tool" >/dev/null || fail "required tool is missing: $tool"
done
if command -v sha256sum >/dev/null; then
    hash_command=(sha256sum)
elif command -v shasum >/dev/null; then
    hash_command=(shasum -a 256)
else
    fail 'SHA-256 tool is missing: install sha256sum or shasum'
fi
bundle_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
input_dir=$(cd -- "$1" && pwd -P)
output_parent=$(cd -- "$(dirname -- "$2")" && pwd -P)
output_dir="$output_parent/$(basename -- "$2")"
case "$output_dir" in *$'\n'*|*$'\r'*|*$'\t'*) fail 'control characters in output paths are unsupported';; esac
[[ ! -e "$output_dir" && ! -L "$output_dir" ]] || fail 'output directory must be new'
base=6ae461a54d85a1b70477c1efb97ea4a33ef3b331
[[ $(git -C "$input_dir" rev-parse "$base^{commit}") == "$base" ]] || fail 'input repository does not contain the exact pinned revision'
(cd -- "$bundle_dir" && "${hash_command[@]}" -c bundle.sha256 >&2)

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
git -c core.autocrlf=false -c core.eol=lf -C "$input_dir" archive --format=tar "$base" | tar -xf - -C "$source_dir"
git -c init.templateDir= init --quiet --initial-branch=codex/spiralcore-mvm-service-patch "$source_dir"
git -C "$source_dir" config core.autocrlf false
git -C "$source_dir" config core.eol lf
git -C "$source_dir" apply --check --whitespace=error-all "$bundle_dir/0001-mailbox-service-primitives.patch"
git -C "$source_dir" apply --whitespace=error-all "$bundle_dir/0001-mailbox-service-primitives.patch"
git -C "$source_dir" apply --check --whitespace=error-all "$bundle_dir/0002-reply-operation-authorization.patch"
git -C "$source_dir" apply --whitespace=error-all "$bundle_dir/0002-reply-operation-authorization.patch"
git -C "$source_dir" apply --check --whitespace=error-all "$bundle_dir/0003-prepared-header-accessor.patch"
git -C "$source_dir" apply --whitespace=error-all "$bundle_dir/0003-prepared-header-accessor.patch"
(cd -- "$source_dir" && "${hash_command[@]}" -c "$bundle_dir/patched-files.sha256" >&2)

toml_string() {
    local value=$1
    value=${value//\\/\\\\}
    value=${value//\"/\\\"}
    printf '"%s"' "$value"
}
config="$output_dir/patch.toml"
{
    for source_url in https://github.com/tinylabscom/mvm https://github.com/Casey-allard/mvm; do
        printf '[patch."%s"]\n' "$source_url"
        for crate in mvm-core mvm-contract mvm-runtime mvm-build mvm-fs mvm-agentd mvm-backends mvm-vmm mvm-net; do
            printf '%s = { path = ' "$crate"
            crate_path="$source_dir/crates/$crate"
            if command -v cygpath >/dev/null; then
                crate_path=$(cygpath -m "$crate_path") || fail 'native Cargo path conversion failed'
            fi
            toml_string "$crate_path"
            printf ' }\n'
        done
        printf '\n'
    done
} > "$config"
printf '%s\n' "$base" > "$output_dir/base-revision"
cp -- "$bundle_dir/provenance.json" "$output_dir/provenance.json"
trap - ERR
printf '%s\n' "$config"
