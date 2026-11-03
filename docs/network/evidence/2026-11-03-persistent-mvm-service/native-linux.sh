#!/usr/bin/env bash
# Runs the existing Rust native supervisor; no alternate mailbox or codec.
set -euo pipefail
umask 077
root=/var/tmp/spiralcore-persistent-20261103
results="$root/native"
assets=/var/tmp/spiralcore-linux-mvm-20261102
bin="$root/after/bin"
link=/usr/local/bin/firecracker
target="$assets/bin/firecracker"
[[ ! -e "$results" && ! -L "$results" ]]
[[ ! -e "$link" && ! -L "$link" ]]
[[ $(sha256sum "$target" | awk '{print $1}') == 99ad0f5cd0514a88aad0e9ae8cfdb3cc3b4ab9d190e1194602406c786b5de7a5 ]]
[[ $(sha256sum "$assets/vmlinux" | awk '{print $1}') == 1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6 ]]
[[ $(sha256sum "$bin/host" | awk '{print $1}') == 289bb89749dd6190b244a278c77183ea1634532b92256e4b8e1b64c5683dc9e4 ]]
[[ $(sha256sum "$bin/guest" | awk '{print $1}') == 9fdd15d61292cd64aedee71861e7623c9be03476adf5be518e778f02508f2fa1 ]]
( cd "$root/source"; sha256sum --status -c ../source-sha256.txt )
mkdir -m 700 "$results"
sha256sum "$bin/host" "$bin/guest" "$assets/vmlinux" "$target" > "$results/binaries.sha256"
readelf -l "$bin/guest" > "$results/guest-elf.txt"
if rg -q INTERP "$results/guest-elf.txt"; then exit 1; fi
uname -a > "$results/host.txt"
uptime >> "$results/host.txt"
free -b >> "$results/host.txt"
df -h "$root" >> "$results/host.txt"
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
pgrep -x firecracker > "$results/processes-before.txt" || true
[[ ! -s "$results/processes-before.txt" ]]
while read -r stage arm size; do
  name="$stage-$arm-$size"
  set +e
  sudo -n /usr/bin/setpriv --reuid callard --regid kvm --init-groups \
    /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 \
    DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
    PATH=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin:/usr/local/bin:/usr/bin:/bin \
    "$bin/host" session "$results/$name" "$assets/vmlinux" "$bin/guest" \
    "$stage" "$arm" "$size" 0 experimental64 \
    > "$results/$name.stdout" 2> "$results/$name.stderr"
  status=$?
  set -e
  printf '%s\n' "$status" > "$results/$name.exit"
  [[ "$status" == 0 ]]
  jq -e '.ok == true and .receiver.exact_reopened == true and .lifecycle_wall_us < 180000000' "$results/$name/result.json" > /dev/null
  if [[ "$stage" == two ]]; then
    jq -e '.host_transport_budget.scope == "orchestrator" and .host_transport_budget.fenced == false and .host_transport_budget.usage == {links:0,frame_slots:0,serialized_buffer_bytes:0} and .host_transport_budget.high_water == {links:2,frame_slots:1,serialized_buffer_bytes:262144}' "$results/$name/result.json" > /dev/null
  else
    jq -e '.host_transport_budget == null and (.host_transport_budget_coverage | startswith("not accounted"))' "$results/$name/result.json" > /dev/null
  fi
done <<'CELLS'
one whole-mixed 1048576
two whole-mixed 1048576
two combined-cpu 1048576
two whole-mixed 16777216
two combined-cpu 16777216
CELLS
pgrep -x firecracker > "$results/processes-after.txt" || true
[[ ! -s "$results/processes-after.txt" ]]
uptime >> "$results/host.txt"
printf 'Native cells verified; payload cleanup waits for compact evidence custody.\n'
