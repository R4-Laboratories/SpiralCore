#!/usr/bin/env bash
set -euo pipefail
root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/sc-flowmux-runner.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
umask 077

expect_status() {
  local expected=$1
  shift
  local observed=0
  "$@" > "$work/out" 2> "$work/err" || observed=$?
  if [[ "$observed" != "$expected" ]]; then
    cat "$work/err" >&2
    printf 'expected exit %s, observed %s\n' "$expected" "$observed" >&2
    exit 1
  fi
}

expect_status 0 bash "$root/run-linux.sh" --help
grep -q 'ISSUER_INPUTS PEER_INPUTS RESULTS_PARENT' "$work/out"
expect_status 2 bash "$root/run-linux.sh"
mkdir -m 0700 "$work/bin" "$work/helpers" "$work/issuer" "$work/peer" "$work/results"
native_realpath=$(command -v realpath)
cat > "$work/bin/realpath" <<'REALPATH'
#!/usr/bin/env bash
[[ $# == 3 && $1 == -e && $2 == -- && $3 == /* && -e $3 ]] || exit 99
exec "${SC_TEST_NATIVE_REALPATH:?}" "$3"
REALPATH
chmod 0700 "$work/bin/realpath"
cat > "$work/bin/uname" <<'UNAME'
#!/usr/bin/env bash
printf 'Linux\n'
UNAME
chmod 0700 "$work/bin/uname"
for tool in sudo unshare mount truncate mkfs.ext4; do
  cat > "$work/bin/$tool" <<'REFUSE'
#!/usr/bin/env bash
printf 'unexpected privileged or allocation operation\n' >&2
exit 99
REFUSE
  chmod 0700 "$work/bin/$tool"
done
for helper in mvm-host-agent mvm-network-endpoint mvm-signer-helper; do
  ln -s /usr/bin/true "$work/helpers/$helper"
done

expect_status 2 env PATH="$work/bin:$PATH" SC_TEST_NATIVE_REALPATH="$native_realpath" bash "$root/run-linux.sh" \
  "$work/absent" /usr/bin/true /usr/bin/true "$work/helpers" /usr/bin/true \
  "$work/issuer" "$work/peer" "$work/results"
grep -q 'installer is not an executable file' "$work/err"
expect_status 2 env PATH="$work/bin:$PATH" SC_TEST_NATIVE_REALPATH="$native_realpath" bash "$root/run-linux.sh" \
  /usr/bin/true /usr/bin/true /usr/bin/true "$work/helpers" /usr/bin/true \
  "$work/issuer" "$work/peer" "$work/results"
grep -q 'controller does not contain the installed workload test' "$work/err"
if [[ -n "$(ls -A "$work/results")" ]]; then
  printf 'preflight created result state\n' >&2
  exit 1
fi
printf '4 runner preflight cases passed; no KVM, mount, or image allocation executed\n'

result="$work/result"
mkdir -m0700 "$result" "$result/drained"
printf 'sdk=0\npeer=0\nnamespace_exit=0\n' > "$result/results"
cat > "$result/controller.log" <<'CONTROLLER'
test installed_original_flowmux_workload ... {"phase":"original_entrypoint","original_workload_exit":0,"stderr_bytes":0}
{"phase":"sdk_retirement","retired":true}
{"phase":"installed_resources","tenant_stopped":true,"all_resources_released":false,"charged":{"guests":0,"workers":0}}
{"phase":"original_retained_reservations","verified_four":true}
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
CONTROLLER
cat > "$result/peer.log" <<'PEER'
READY 192.0.2.7:38443
{"stream_id":3,"echo_received":5804,"echo_written":5804,"tls_rx":100,"tls_tx":100,"accept_to_half_close_ns":100}
PEER
cp "$result/peer.log" "$work/peer-original"
for leaf in flowmux-operator.json flowmux-guest.json .archived-audit-0 local.fm-a.workload.jsonl; do
  printf 'synthetic structural fixture, not signed audit proof\n' > "$result/drained/$leaf"
done
expect_status 0 bash "$root/verify-run.sh" "$result"
mv "$result/drained/.archived-audit-0" "$result/audit-held"
expect_status 1 bash "$root/verify-run.sh" "$result"
mv "$result/audit-held" "$result/drained/.archived-audit-0"
cp "$result/controller.log" "$work/controller-original"
sed 's/1 passed/0 passed/' "$work/controller-original" > "$result/controller.log"
expect_status 1 bash "$root/verify-run.sh" "$result"
sed 's/"retired":true/"retired":false/' "$work/controller-original" > "$result/controller.log"
expect_status 1 bash "$root/verify-run.sh" "$result"
cp "$work/controller-original" "$result/controller.log"
sed 's/5804/5803/g' "$result/peer.log" > "$work/peer-short"
mv "$work/peer-short" "$result/peer.log"
expect_status 1 bash "$root/verify-run.sh" "$result"
cp "$work/peer-original" "$result/peer.log"
sed 's/^test installed_original_flowmux_workload \.\.\. //' "$work/controller-original" > "$result/controller.log"
expect_status 1 bash "$root/verify-run.sh" "$result"
cp "$work/controller-original" "$result/controller.log"
sed 's/"tls_rx":100/"tls_rx":"100"/' "$work/peer-original" > "$result/peer.log"
expect_status 1 bash "$root/verify-run.sh" "$result"
printf '7 structural result controls passed; no signature or native qualification claimed\n'
