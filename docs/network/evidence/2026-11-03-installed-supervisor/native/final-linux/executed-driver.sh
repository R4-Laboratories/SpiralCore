#!/usr/bin/env bash
set -euo pipefail
umask 077
base=/var/tmp/spiralcore-installed-supervisor-20261103
assets=/var/tmp/spiralcore-linux-mvm-20261102
link=/usr/local/bin/firecracker
target="$assets/bin/firecracker"
id=5959595959595959595959595959595959595959595959595959595959595959
if [ -e "$link" ] || [ -L "$link" ]; then exit 91; fi
if [ "$(sha256sum "$target" | awk '{print $1}')" != 99ad0f5cd0514a88aad0e9ae8cfdb3cc3b4ab9d190e1194602406c786b5de7a5 ]; then exit 92; fi
if [ "$(sha256sum "$assets/vmlinux" | awk '{print $1}')" != 1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6 ]; then exit 93; fi
if pgrep -x firecracker > "$base/validation/native-final-processes-before.txt"; then exit 94; fi
"$base/bin/host-integrated" installation-reconcile "$base/native/installation" "$id" > "$base/validation/native-final-reconcile.txt" 2>&1
sudo -n ln -s "$target" "$link"
inode=$(stat -c %i "$link")
cleanup() {
    saved_rc=$?
    trap - EXIT
    if [ -L "$link" ] && [ "$(readlink "$link")" = "$target" ] && [ "$(stat -c %i "$link")" = "$inode" ] && [ "$(stat -c %u "$link")" = 0 ]; then
        if sudo -n unlink "$link"; then
            printf '0\n' > "$base/validation/native-final-link-cleanup.exit"
        else
            printf '1\n' > "$base/validation/native-final-link-cleanup.exit"
            saved_rc=1
        fi
    else
        printf 'Owned Firecracker link changed; leaving untouched\n' >&2
        saved_rc=1
    fi
    exit "$saved_rc"
}
trap cleanup EXIT
sha256sum "$base/bin/host-integrated" "$base/bin/guest-integrated" "$assets/vmlinux" "$target" > "$base/validation/native-final-binaries.sha256"
uname -a > "$base/validation/native-final-host.txt"
uptime >> "$base/validation/native-final-host.txt"
free -b >> "$base/validation/native-final-host.txt"
for cell in two-final-whole-mixed-16777216 two-final-combined-cpu-16777216; do
    if [ -e "$base/native/$cell" ] || [ -L "$base/native/$cell" ]; then exit 95; fi
    stage=two; size=16777216; if [ "$cell" = two-final-whole-mixed-16777216 ]; then arm=whole-mixed; else arm=combined-cpu; fi
    set +e
    timeout 180 sudo -n /usr/bin/setpriv --reuid callard --regid kvm --init-groups \
        /usr/bin/env XDG_RUNTIME_DIR=/run/user/1000 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
        PATH=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin:/usr/local/bin:/usr/bin:/bin \
        "$base/bin/host-integrated" session-installed "$base/native/installation" "$id" "$base/native/$cell" \
        "$assets/vmlinux" "$base/bin/guest-integrated" "$stage" "$arm" "$size" 0 \
        > "$base/validation/native-$cell.txt" 2>&1
    run_rc=$?
    set -e
    printf '%s\n' "$run_rc" > "$base/validation/native-$cell.exit"
    tail -8 "$base/validation/native-$cell.txt"
    if [ "$run_rc" -ne 0 ]; then exit "$run_rc"; fi
    jq -e '.ok == true and .receiver.exact_reopened == true and
        .sender_worker.installed_host_lease == true and .sender_worker.backend_exit_confirmed == true and
        (if .plan.stage == "two" then
            .receiver_worker.installed_host_lease == true and .receiver_worker.backend_exit_confirmed == true and
            .recovery_worker.installed_host_lease == true and .recovery_worker.backend_exit_confirmed == true
        else true end)' "$base/native/$cell/result.json" > "$base/validation/native-$cell-check.txt"
done
if pgrep -x firecracker > "$base/validation/native-final-processes-after.txt"; then exit 96; fi
printf 'PASS: installed native one-guest and two-guest functional cells; not WAN/performance acceptance.\n'
