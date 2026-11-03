#!/usr/bin/env bash
# Existing Rust engine, two guests maximum; retain failed cells and all disks
# until verified archive/member custody permits separate owned cleanup.
set -euo pipefail
umask 077
root=/var/tmp/spiralcore-session-events-20261103
results="$root/native"
before=/var/tmp/spiralcore-duplex-20261103/bin-final
after="$root/bin"
assets=/var/tmp/spiralcore-linux-mvm-20261102
link=/usr/local/bin/firecracker
target="$assets/bin/firecracker"
[[ ! -e "$results" && ! -L "$results" ]]
[[ ! -e "$link" && ! -L "$link" ]]
[[ $(sha256sum "$target" | awk '{print $1}') == 99ad0f5cd0514a88aad0e9ae8cfdb3cc3b4ab9d190e1194602406c786b5de7a5 ]]
[[ $(sha256sum "$assets/vmlinux" | awk '{print $1}') == 1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6 ]]
[[ $(sha256sum "$before/host" | awk '{print $1}') == 0b851e9df95cdfb1f7f999035b4e551707fd29ca3744d532041cb2bcadc991e6 ]]
[[ $(sha256sum "$before/guest" | awk '{print $1}') == c9fba260cbbbb16fb18d9b9bf7e0f87df26ceacdb8d5cab037699094c70b5431 ]]
(cd "$root/source"; sha256sum --status -c ../qualified-source-sha256.txt)
(cd "$after"; sha256sum --status -c SHA256SUMS)
mkdir -m 700 "$results"
sha256sum "$before/host" "$before/guest" "$after/host" "$after/guest" "$assets/vmlinux" "$target" > "$results/binaries.sha256"
for guest in "$before/guest" "$after/guest"; do readelf -l "$guest" >> "$results/guest-elf.txt"; done
if rg -q INTERP "$results/guest-elf.txt"; then exit 1; fi
uname -a > "$results/host.txt"
uptime >> "$results/host.txt"
free -b >> "$results/host.txt"
pgrep -x firecracker > "$results/processes-before.txt" || true
[[ ! -s "$results/processes-before.txt" ]]
sudo -n ln -s "$target" "$link"
inode=$(stat -c %i "$link")
cleanup() {
  if [[ -L "$link" && $(readlink "$link") == "$target" && $(stat -c %i "$link") == "$inode" && $(stat -c %u "$link") == 0 ]]; then
    sudo -n unlink "$link"
  else
    printf 'Owned Firecracker link changed; leaving untouched\n' >&2
    return 1
  fi
  [[ ! -e "$link" && ! -L "$link" ]]
}
trap cleanup EXIT
while read -r version arm; do
  bin="$after"
  if [[ "$version" == before ]]; then bin="$before"; fi
  name="$version-two-$arm-16777216"
  set +e
  timeout 300 sudo -n /usr/bin/setpriv --reuid callard --regid kvm --init-groups \
    /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 \
    DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
    PATH=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin:/usr/local/bin:/usr/bin:/bin \
    "$bin/host" session "$results/$name" "$assets/vmlinux" "$bin/guest" \
    two "$arm" 16777216 0 experimental64 > "$results/$name.stdout" 2> "$results/$name.stderr"
  status=$?
  set -e
  printf '%s\n' "$status" > "$results/$name.exit"
  [[ "$status" == 0 ]]
  jq -e '.ok == true and .receiver.exact_reopened == true and
    .host_transport_budget.usage == {links:0,frame_slots:0,serialized_buffer_bytes:0} and
    .host_transport_budget.limits == {links:2,frame_slots:2,serialized_buffer_bytes:524288} and
    .host_transport_budget.high_water == {links:2,frame_slots:2,serialized_buffer_bytes:524288} and
    .host_transport_budget.refusals == 0 and .host_transport_budget.fenced == false and
    .physical_vsock_stream_bytes == (2 * .logical_envelope_and_framing_bytes)' \
    "$results/$name/result.json" > /dev/null
done <<'CELLS'
before whole-mixed
before combined-cpu
after whole-mixed
after combined-cpu
CELLS
pgrep -x firecracker > "$results/processes-after.txt" || true
[[ ! -s "$results/processes-after.txt" ]]
uptime >> "$results/host.txt"
printf 'PASS: four exact native cells; retain disks pending archive custody.\n'
