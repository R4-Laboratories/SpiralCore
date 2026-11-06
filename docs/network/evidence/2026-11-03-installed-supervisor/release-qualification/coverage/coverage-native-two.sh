#!/usr/bin/env bash
set -euo pipefail
umask 077
base=/var/tmp/spiralcore-installed-supervisor-20261103
assets=/var/tmp/spiralcore-linux-mvm-20261102
link=/usr/local/bin/firecracker
target="$assets/bin/firecracker"
id=5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b
if [ -e "$link" ] || [ -L "$link" ]; then exit 91; fi
if [ "$(sha256sum "$target" | awk '{print $1}')" != 99ad0f5cd0514a88aad0e9ae8cfdb3cc3b4ab9d190e1194602406c786b5de7a5 ]; then exit 92; fi
if [ "$(sha256sum "$assets/vmlinux" | awk '{print $1}')" != 1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6 ]; then exit 93; fi
if pgrep -x firecracker > "$base/validation/coverage-native-processes-before.txt"; then exit 94; fi
"$base/coverage-selective/bin/host-coverage" installation-init "$base/coverage-selective/installation" "$id" > "$base/validation/coverage-native-reconcile.txt" 2>&1
sudo -n ln -s "$target" "$link"
inode=$(stat -c %i "$link")
cleanup() {
    saved_rc=$?
    trap - EXIT
    if [ -L "$link" ] && [ "$(readlink "$link")" = "$target" ] && [ "$(stat -c %i "$link")" = "$inode" ] && [ "$(stat -c %u "$link")" = 0 ]; then
        if sudo -n unlink "$link"; then
            printf '0\n' > "$base/validation/coverage-native-link-cleanup.exit"
        else
            printf '1\n' > "$base/validation/coverage-native-link-cleanup.exit"
            saved_rc=1
        fi
    else
        printf 'Owned Firecracker link changed; leaving untouched\n' >&2
        saved_rc=1
    fi
    exit "$saved_rc"
}
trap cleanup EXIT
sha256sum "$base/coverage-selective/bin/host-coverage" "$base/bin/guest-integrated" "$assets/vmlinux" "$target" > "$base/validation/coverage-native-binaries.sha256"
uname -a > "$base/validation/coverage-native-host.txt"
uptime >> "$base/validation/coverage-native-host.txt"
free -b >> "$base/validation/coverage-native-host.txt"
for cell in two-coverage-whole-mixed-1048576 two-coverage-combined-cpu-1048576; do
    if [ -e "$base/native/$cell" ] || [ -L "$base/native/$cell" ]; then exit 95; fi
    stage=two; size=1048576; if [ "$cell" = two-coverage-whole-mixed-1048576 ]; then arm=whole-mixed; else arm=combined-cpu; fi
    set +e
    timeout 180 sudo -n /usr/bin/setpriv --reuid callard --regid kvm --init-groups \
        /usr/bin/env LLVM_PROFILE_FILE="$base/coverage-selective/profiles/%m-%p.profraw" XDG_RUNTIME_DIR=/run/user/1000 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
        PATH=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin:/usr/local/bin:/usr/bin:/bin \
        "$base/coverage-selective/bin/host-coverage" session-installed "$base/coverage-selective/installation" "$id" "$base/native/$cell" \
        "$assets/vmlinux" "$base/bin/guest-integrated" "$stage" "$arm" "$size" 0 \
        > "$base/validation/coverage-$cell.txt" 2>&1
    run_rc=$?
    set -e
    printf '%s\n' "$run_rc" > "$base/validation/coverage-$cell.exit"
    tail -8 "$base/validation/coverage-$cell.txt"
    if [ "$run_rc" -ne 0 ]; then exit "$run_rc"; fi
    jq -e '.ok == true and .receiver.exact_reopened == true and
        .sender_worker.installed_host_lease == true and .sender_worker.backend_exit_confirmed == true and
        (if .plan.stage == "two" then
            .receiver_worker.installed_host_lease == true and .receiver_worker.backend_exit_confirmed == true and
            .recovery_worker.installed_host_lease == true and .recovery_worker.backend_exit_confirmed == true
        else true end)' "$base/native/$cell/result.json" > "$base/validation/coverage-$cell-check.txt"
done
if pgrep -x firecracker > "$base/validation/coverage-native-processes-after.txt"; then exit 96; fi
printf 'PASS: installed native one-guest and two-guest functional cells; not WAN/performance acceptance.\n'
