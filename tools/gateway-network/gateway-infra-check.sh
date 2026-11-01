#!/usr/bin/env bash
# Pure registration/syntax checks. Does not create namespaces or invoke sudo.
set -euo pipefail
[ "$#" = 1 ] || exit 2
out=$1
[ ! -e "$out" ] || { echo "check output must be fresh" >&2; exit 2; }
mkdir -p "$out"
registration=docs/network/evidence/2026-11-01-tun-gateway/registration.json
driver=tools/gateway-network/gateway-netns.sh
bash -n "$driver"
bash -n tools/gateway-network/gateway-infra-check.sh
bash "$driver" validate "$registration" > "$out/positive.stdout" 2> "$out/positive.stderr"
printf '0\n' > "$out/positive.exit"
for name in unregistered dose modes mtu payload retry; do
  case "$name" in
    unregistered) mutation='.launch_ready=false';;
    dose) mutation='.dose.trials=[0,1,2,3]';;
    modes) mutation='.dose.packet_modes=["raw","zstd","warm"]';;
    mtu) mutation='.topology.tun_mtu=1500';;
    payload) mutation='.dose.flows_per_session[0].bytes=67108864';;
    retry) mutation='.custody.automatic_retries=true';;
  esac
  jq "$mutation" "$registration" > "$out/$name.json"
  set +e
  bash "$driver" validate "$out/$name.json" > "$out/$name.stdout" 2> "$out/$name.stderr"
  code=$?
  set -e
  printf '%s\n' "$code" > "$out/$name.exit"
  [ "$code" -ne 0 ] || { echo "negative unexpectedly accepted: $name" >&2; exit 1; }
done
printf '{"scope":"pure Bash syntax and registration gates; no Linux runtime claim","positive":1,"negatives":6,"passed":true}\n' > "$out/result.json"
