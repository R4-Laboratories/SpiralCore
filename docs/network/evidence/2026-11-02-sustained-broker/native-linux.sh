#!/usr/bin/env bash
# Executes existing Rust measurements; no new protocol or measurement arithmetic.
set -euo pipefail
umask 077
root=/var/tmp/spiralcore-sustained-20261102
results="$root/native-seven-d940203"
assets=/var/tmp/spiralcore-linux-mvm-20261102
bin="$root/bin-d940203"
link=/usr/local/bin/firecracker
target="$assets/bin/firecracker"
[[ ! -e "$results" && ! -L "$results" ]]
[[ $(sha256sum "$target" | awk '{print $1}') == 99ad0f5cd0514a88aad0e9ae8cfdb3cc3b4ab9d190e1194602406c786b5de7a5 ]]
[[ $(sha256sum "$assets/vmlinux" | awk '{print $1}') == 1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6 ]]
[[ $(sha256sum "$bin/host" | awk '{print $1}') == d93f66539df9ac5b2fab3d0392a8d5963ee3f2b73aff1bf0b29f96211b3226bb ]]
[[ $(sha256sum "$bin/guest" | awk '{print $1}') == 71fc46316e526a6856bdd5bf58e32e089dbc1abe3be70feb94b5dbcccfe8a680 ]]
[[ ! -e "$link" && ! -L "$link" ]]
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
    sudo -n rm -- "$link"
  else
    printf 'Owned Firecracker link changed; leaving untouched\n' >&2
    return 1
  fi
  [[ ! -e "$link" && ! -L "$link" ]]
}
trap cleanup EXIT
pgrep -x firecracker > "$results/processes-before.txt" || true
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
  # Stop after failure: leave its owned artifacts and cleanup link on exit.
  [[ "$status" == 0 ]]
  jq -e '.ok == true and .sender.event == "sender_done" and .receiver.exact_reopened == true' "$results/$name/result.json" > /dev/null
done <<'CELLS'
one whole-mixed 1048576
one combined-cpu 1048576
two whole-mixed 1048576
two combined-cpu 1048576
two raw-mixed 16777216
two whole-mixed 16777216
two combined-cpu 16777216
CELLS
pgrep -x firecracker > "$results/processes-after.txt" || true
uptime >> "$results/host.txt"
printf 'Native cells recorded; payload cleanup requires archived verification.\n'
