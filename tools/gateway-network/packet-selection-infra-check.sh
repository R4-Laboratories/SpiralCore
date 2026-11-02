#!/usr/bin/env bash
# Pure registration/syntax checks. Never creates namespaces or invokes sudo.
set -euo pipefail
[ "$#" = 1 ] || exit 2
out=$1
[ ! -e "$out" ] || { echo "check output must be fresh" >&2; exit 2; }
mkdir -p "$out"
registration=docs/network/evidence/2026-11-01-packet-selection/network-registration.json
driver=tools/gateway-network/packet-selection-netns.sh
bash -n "$driver"
bash -n tools/gateway-network/packet-selection-infra-check.sh
# Check the proposed executable contract while the committed document may
# still be NOT_READY. This does not enable launch of the original registration.
jq '.launch_ready=true' "$registration" > "$out/ready-fixture.json"
bash "$driver" validate "$out/ready-fixture.json" > "$out/positive.stdout" 2> "$out/positive.stderr"
printf '0\n' > "$out/positive.exit"
for name in unregistered dose modes mtu payload retry shaping queue smoke; do
  case "$name" in
    unregistered) mutation='.launch_ready=false';;
    dose) mutation='.dose.trials=[0,1,2,3]';;
    modes) mutation='.modes=["raw","zstd","fast-zstd","adaptive","warm"]';;
    mtu) mutation='.topology.tun_mtu=1500';;
    payload) mutation='.dose.primary_transfers[0].bytes=67108864';;
    retry) mutation='.custody.automatic_retries=true';;
    shaping) mutation='.profiles[1].rate_bits_per_second_per_direction=100000000';;
    queue) mutation='.topology.queue_limit_packets=1000';;
    smoke) mutation='.dose.smoke_sessions=12';;
  esac
  jq "$mutation" "$out/ready-fixture.json" > "$out/$name.json"
  set +e
  bash "$driver" validate "$out/$name.json" > "$out/$name.stdout" 2> "$out/$name.stderr"
  code=$?
  set -e
  printf '%s\n' "$code" > "$out/$name.exit"
  [ "$code" -ne 0 ] || { echo "negative unexpectedly accepted: $name" >&2; exit 1; }
done
printf '{"status":"ok","denied_packets":0,"invalid_frames":0,"capacity_drops":0,"expired_packets":3}\n' > "$out/expiry-cost.json"
printf '[{"kind":"netem","drops":7,"qlen":0}]\n' > "$out/queue-cost.json"
bash "$driver" check-statistics gateway "$out/expiry-cost.json" > "$out/expiry-cost-accepted.txt"
bash "$driver" check-statistics qdisc "$out/queue-cost.json" > "$out/queue-cost-accepted.txt"
for name in malformed capacity auth capture-backlog missing-counter; do
  kind=gateway; input="$out/expiry-cost.json"
  case "$name" in
    malformed) mutation='.invalid_frames=1';;
    capacity) mutation='.capacity_drops=1 | .invalid_frames=1';;
    auth) mutation='.status="error"';;
    capture-backlog) kind=qdisc; input="$out/queue-cost.json"; mutation='.[0].qlen=1';;
    missing-counter) kind=qdisc; input="$out/queue-cost.json"; mutation='del(.[0].drops)';;
  esac
  jq "$mutation" "$input" > "$out/$name.json"
  set +e
  bash "$driver" check-statistics "$kind" "$out/$name.json" > "$out/$name.stdout" 2> "$out/$name.stderr"
  code=$?
  set -e
  printf '%s\n' "$code" > "$out/$name.exit"
  [ "$code" -ne 0 ] || { echo "statistics negative unexpectedly accepted: $name" >&2; exit 1; }
done
printf '{"scope":"pure Bash syntax, registration and congestion classification gates; does not enable the committed registration or claim Linux runtime","registration_positive":1,"registration_negatives":9,"congestion_positives":2,"statistics_negatives":5,"passed":true}\n' > "$out/result.json"
