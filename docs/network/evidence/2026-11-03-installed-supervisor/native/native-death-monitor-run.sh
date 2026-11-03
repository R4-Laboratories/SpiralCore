#!/usr/bin/env bash
set -euo pipefail
umask 077
base=/var/tmp/spiralcore-installed-supervisor-20261103
assets=/var/tmp/spiralcore-linux-mvm-20261102
link=/usr/local/bin/firecracker
target="$assets/bin/firecracker"
if [ -e "$link" ] || [ -L "$link" ]; then exit 91; fi
if [ "$(sha256sum "$target" | awk '{print $1}')" != 99ad0f5cd0514a88aad0e9ae8cfdb3cc3b4ab9d190e1194602406c786b5de7a5 ]; then exit 92; fi
if pgrep -x firecracker > "$base/validation/native-death-monitor-processes-before.txt"; then exit 94; fi
sudo -n ln -s "$target" "$link"
inode=$(stat -c %i "$link")
cleanup() {
    saved_rc=$?
    trap - EXIT
    if [ -L "$link" ] && [ "$(readlink "$link")" = "$target" ] && [ "$(stat -c %i "$link")" = "$inode" ] && [ "$(stat -c %u "$link")" = 0 ]; then
        if sudo -n unlink "$link"; then printf '0\n' > "$base/validation/native-death-monitor-link-cleanup.exit";
        else saved_rc=1; fi
    else saved_rc=1; fi
    exit "$saved_rc"
}
trap cleanup EXIT
sha256sum "$base/bin/host" "$base/bin/guest" "$assets/vmlinux" "$target" "$base/bin/installed-native-death-monitor" > "$base/validation/native-death-monitor-binaries.sha256"
set +e
sudo -n /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus PATH=/usr/local/bin:/usr/bin:/bin \
 "$base/bin/installed-native-death-monitor" "$base/bin/host" "$assets/vmlinux" "$base/bin/guest" "$base/native/death-recovery-monitor" "$target" callard \
 > "$base/validation/native-death-monitor-root.txt" 2>&1
run_rc=$?
set -e
printf '%s\n' "$run_rc" > "$base/validation/native-death-monitor-root.exit"
cat "$base/validation/native-death-monitor-root.txt"
if pgrep -x firecracker > "$base/validation/native-death-monitor-processes-after.txt"; then exit 96; fi
exit "$run_rc"
