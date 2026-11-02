#!/usr/bin/env bash
# Download only pinned public test assets into a new private directory.
set -euo pipefail
umask 077

fail() { printf 'prepare-assets: %s\n' "$*" >&2; exit 1; }
[[ $# == 1 ]] || fail 'usage: prepare-assets.sh NEW_ABSOLUTE_DIRECTORY'
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || fail 'requires Linux x86_64'
destination=$1
[[ $destination == /* && ! -e $destination && ! -L $destination ]] || fail 'destination must be a new absolute directory'
for command in curl sha256sum tar readelf stat; do
    command -v "$command" >/dev/null || fail "missing command: $command"
done
mkdir -- "$destination"
cd -- "$destination"
mkdir bin
fetch() {
    local url=$1 name=$2 digest=$3
    curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
        --connect-timeout 20 --max-time 300 --output "$name" "$url"
    printf '%s  %s\n' "$digest" "$name" | sha256sum -c -
}

image_release=https://github.com/tinylabscom/mvm-images/releases/download/image-set/v0.2.4
fetch "$image_release/image-set.json" image-set.json \
    5033520a59514f5d206d76d13db8c9b7e5841f068c83dd8585ef141879a1bb24
fetch "$image_release/default-microvm-vmlinux-x86_64" vmlinux \
    1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6
[[ $(stat -c %s vmlinux) == 19995452 ]] || fail 'kernel size differs from image-set manifest'
readelf -h vmlinux | grep -Fq 'Advanced Micro Devices X86-64' || fail 'kernel is not x86_64 ELF'

firecracker_release=https://github.com/firecracker-microvm/firecracker/releases/download/v1.17.0
archive=firecracker-v1.17.0-x86_64.tgz
fetch "$firecracker_release/$archive" "$archive" \
    06094a1108ae9e82aa4c23a775aa92758f53f1175d422270d9d6162cb9ade558
# Extract only the two reviewed archive member names after the whole-archive hash check.
for binary in firecracker jailer; do
    tar -xOf "$archive" "release-v1.17.0-x86_64/$binary-v1.17.0-x86_64" > "bin/$binary"
    chmod 0700 "bin/$binary"
done
bin/firecracker --version | grep -Fq 'Firecracker v1.17.0' || fail 'unexpected Firecracker version'
sha256sum image-set.json vmlinux "$archive" bin/firecracker bin/jailer > assets.sha256
printf 'assets ready: %s\n' "$destination"
cat assets.sha256
# This script deliberately does not boot a guest, install globally, or change KVM access.
