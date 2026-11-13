#!/usr/bin/env bash
# One finite disposable filesystem and the original installed SDK resource owners.
set -euo pipefail
umask 077
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
usage() {
  printf '%s\n' 'Usage: run-linux.sh INSTALLER PEER CONTROLLER HELPERS FIRECRACKER ISSUER_INPUTS PEER_INPUTS RESULTS_PARENT'
  printf '%s\n' 'Run only on an explicitly approved Linux/KVM test host. Results contain private signed evidence.'
}
if [[ ${1:-} == --help && $# == 1 ]]; then usage; exit 0; fi
if [[ $# != 8 ]]; then usage >&2; exit 2; fi
fail() { printf '%s\n' "$1" >&2; exit 2; }
[[ $(uname -s) == Linux ]] || fail 'this installed-host witness requires Linux'
[[ -f $1 && -x $1 ]] || fail 'installer is not an executable file'
[[ -f $2 && -x $2 ]] || fail 'peer is not an executable file'
[[ -f $3 && -x $3 ]] || fail 'controller is not an executable file'
[[ -d $4 && -f $5 && -x $5 ]] || fail 'helper directory and Firecracker executable required'
[[ -d $6 && -d $7 && -d $8 ]] || fail 'existing input and results directories required'
installer=$(realpath -e -- "$1")
peer=$(realpath -e -- "$2")
controller=$(realpath -e -- "$3")
helpers=$(realpath -e -- "$4")
fc=$(realpath -e -- "$5")
issuer_inputs=$(realpath -e -- "$6")
peer_inputs=$(realpath -e -- "$7")
results_parent=$(realpath -e -- "$8")
[[ $issuer_inputs != "$peer_inputs" ]] || fail 'issuer and peer require separate input directories'
if ! tests=$("$controller" --list --exact installed_original_flowmux_workload) ||
   ! printf '%s\n' "$tests" | grep -qx 'installed_original_flowmux_workload: test'; then
  fail 'controller does not contain the installed workload test'
fi
for tool in sudo unshare setpriv prlimit truncate mkfs.ext4 mount findmnt blockdev losetup sha256sum getent ps jq; do
  command -v "$tool" >/dev/null || fail "required Linux tool unavailable: $tool"
done
[[ -x /usr/bin/time && -r /dev/kvm && -w /dev/kvm ]] || fail 'GNU time and authorized KVM access required'
for helper in mvm-host-agent mvm-network-endpoint mvm-signer-helper; do
  [[ -f $helpers/$helper && -x $helpers/$helper ]] || fail "missing installed helper: $helper"
done
for exe in "$fc" "$helpers"/mvm-{network-endpoint,host-agent,signer-helper}; do
  while read -r pid state; do
    [[ -n $pid && $state != Z* ]] || continue
    if [[ $exe == "$fc" || /proc/$pid/exe -ef $exe ]]; then
      fail "occupied original native lane: $exe"
    fi
  done < <(ps -C "${exe##*/}" -o pid=,stat=)
done
uid=$(id -u); gid=$(id -g)
groups=$(id -G | tr ' ' ',')
run=$(mktemp -d "$results_parent/native.XXXXXXXX")
mountpoint=$(mktemp -d /tmp/fm-XXXXXXXX)
printf '%s\n' "$mountpoint" > "$run/mount-name"
sha256sum "$installer" "$peer" "$controller" "$helpers"/mvm-{host-agent,network-endpoint,signer-helper} "$fc" > "$run/executables.sha256"
truncate -s 536870912 "$run/finite.ext4"
mkfs.ext4 -q -m 0 -F "$run/finite.ext4"
set +e
sudo -n unshare --mount --propagation private bash -se -- "$run" "$mountpoint" "$uid" "$gid" "$groups" "$installer" "$peer" "$controller" "$helpers" "$fc" "$issuer_inputs" "$peer_inputs" <<'NAMESPACE' | tee "$run/mount.txt"
set -euo pipefail
run=$1; mp=$2; uid=$3; gid=$4; groups=$5; installer=$6; peer=$7; controller=$8; helpers=$9; fc=${10}; issuer_inputs=${11}; peer_inputs=${12}
mount -o loop,nodev,nosuid "$run/finite.ext4" "$mp"
chown "$uid:$gid" "$mp"
install -o "$uid" -g "$gid" -m 0700 -d "$mp/h" "$mp/t" "$mp/peer-tmp"
findmnt -n -T "$mp" -o SOURCE,FSTYPE,OPTIONS
dev=$(findmnt -n -T "$mp" -o SOURCE)
blockdev --getsize64 "$dev"
operator_user=$(getent passwd "$uid" | cut -d: -f1)
operator_home=$(getent passwd "$uid" | cut -d: -f6)
setpriv --reuid="$uid" --regid="$gid" --groups="$groups" --no-new-privs env -i \
  HOME="$operator_home" USER="$operator_user" LOGNAME="$operator_user" XDG_RUNTIME_DIR="/run/user/$uid" DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
  MVM_HOME="$mp/h" TMPDIR="$mp/t" MVM_FIRECRACKER_BIN="$fc" MVM_FC_IDLE_POOL=0 \
  MVM_HOST_AGENT_PATH="$helpers/mvm-host-agent" MVM_SIGNER_HELPER_PATH="$helpers/mvm-signer-helper" \
  MVM_SUBSTITUTION_ENDPOINT_PATH="$helpers/mvm-network-endpoint" PATH="$helpers:$PATH" \
  bash -se -- "$run" "$mp" "$installer" "$peer" "$controller" "$issuer_inputs" "$peer_inputs" <<'ORIGINAL'
set -euo pipefail
run=$1; mp=$2; issuer=$3; peer=$4; controller=$5; issuer_inputs=$6; peer_inputs=$7
umask 077
id > "$run/controller-user.txt"
test -r /dev/kvm && test -w /dev/kvm
/usr/bin/time -v -o "$run/install.time" prlimit --as=536870912:536870912 -- "$issuer" --inputs "$issuer_inputs" > "$run/installation.log" 2>&1
test -f "$MVM_HOME/flowmux-operator.json"
df -B1 "$MVM_HOME" > "$run/installed-fs.txt"
coproc PEER { exec /usr/bin/time -v -o "$run/peer.time" env TMPDIR="$mp/peer-tmp" prlimit --as=201326592:201326592 -- "$peer" --inputs "$peer_inputs" 2>"$run/peer.err"; }
original_peer=$PEER_PID
exec {peer_output}<&"${PEER[0]}"
if ! IFS= read -r -t 15 ready <&"$peer_output" || [[ $ready != READY\ * ]]; then
  printf '%s\n' "${ready:-}" > "$run/peer.log"
  cat <&"$peer_output" >> "$run/peer.log" || :
  wait "$original_peer" || :
  exit 1
fi
printf '%s\n' "$ready" > "$run/peer.log"
cat <&"$peer_output" >> "$run/peer.log" &
peer_drain=$!
set +e
/usr/bin/time -v -o "$run/controller.time" prlimit --as=805306368:805306368 -- "$controller" --exact installed_original_flowmux_workload --ignored --nocapture --test-threads=1 > "$run/controller.log" 2>&1
sdk=$?
wait "$original_peer"; observed_peer=$?
wait "$peer_drain"; drained=$?
exec {peer_output}<&-
if (( drained != 0 )); then observed_peer=1; fi
printf 'sdk=%s\npeer=%s\n' "$sdk" "$observed_peer" > "$run/results"
df -B1 "$MVM_HOME" > "$run/final-fs.txt"
find "$MVM_HOME" -type f \( -name 'console.log' -o -name 'firecracker.log' -o -name 'fc.log' \) -printf '%f %s\n' > "$run/guest-log-sizes"
# Retain signed evidence before the namespace releases the original inode custody.
mkdir -m0700 "$run/drained"
for file in flowmux-operator.json flowmux-guest.json installation/.archived-audit-0 audit/local.fm-a.workload.jsonl vms/fm-a/console.log vms/fm-a/firecracker.log vms/fm-a/network-endpoint.stderr.log; do
  if [[ -f $MVM_HOME/$file && ! -L $MVM_HOME/$file ]]; then
    cp -- "$MVM_HOME/$file" "$run/drained/${file##*/}" || exit 1
    stat --format='%d %i %s %a %u %n' "$MVM_HOME/$file" >> "$run/drained/original-stat"
  fi
done
( cd "$run/drained" && find . -type f -print0 | sort -z | xargs -0 -r sha256sum ) > "$run/drained.sha256" || exit 1
exit $(( sdk != 0 || observed_peer != 0 ))
ORIGINAL
NAMESPACE
status=$?
set -e
printf 'namespace_exit=%s\n' "$status" >> "$run/results"
sudo -n losetup -j "$run/finite.ext4" | tee "$run/loop-after" >/dev/null
if [[ -s $run/loop-after ]] || findmnt -n -M "$mountpoint" >/dev/null; then
  printf '%s\n' 'original mount or loop attachment still present; retaining backing' >&2
  exit 1
fi
rmdir "$mountpoint"
printf '%s\n' "$run"
if (( status == 0 )); then bash "$script_dir/verify-run.sh" "$run" || status=1; fi
exit "$status"
