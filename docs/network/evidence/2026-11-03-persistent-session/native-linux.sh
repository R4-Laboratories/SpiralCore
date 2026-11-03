#!/usr/bin/env bash
set -euo pipefail
umask 077
root=/var/tmp/spiralcore-session-20261103
results="$root/native"
assets=/var/tmp/spiralcore-linux-mvm-20261102
bin="$root/bin"
link=/usr/local/bin/firecracker
target="$assets/bin/firecracker"
[[ ! -e "$results" && ! -L "$results" ]]
[[ ! -e "$link" && ! -L "$link" ]]
[[ $(sha256sum "$target" | awk '{print $1}') == 99ad0f5cd0514a88aad0e9ae8cfdb3cc3b4ab9d190e1194602406c786b5de7a5 ]]
[[ $(sha256sum "$assets/vmlinux" | awk '{print $1}') == 1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6 ]]
[[ $(sha256sum "$bin/host" | awk '{print $1}') == 7e521902a921567d9b8d8096171c5887d4a3278838d398ac70d121036a943c32 ]]
[[ $(sha256sum "$bin/guest" | awk '{print $1}') == 85e704f11f122b20984ba3bf6e761e7b7c5a5f7e9d76d2bfa2d2f7672220bea9 ]]
( cd "$root/source"; sha256sum --status -c ../source-sha256.txt )
mkdir -m 700 "$results"
sha256sum "$bin/host" "$bin/guest" "$assets/vmlinux" "$target" > "$results/binaries.sha256"
readelf -l "$bin/guest" > "$results/guest-elf.txt"
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
  jq -e '.ok == true and .receiver.exact_reopened == true' "$results/$name/result.json" > /dev/null
done <<'CELLS'
one whole-mixed 1048576
two whole-mixed 16777216
two combined-cpu 16777216
CELLS
pgrep -x firecracker > "$results/processes-after.txt" || true
[[ ! -s "$results/processes-after.txt" ]]
uptime >> "$results/host.txt"
printf 'PASS: three native cells; preserve payload until receipt custody verified.\n'
