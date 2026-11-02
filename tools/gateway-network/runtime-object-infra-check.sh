#!/usr/bin/env bash
# Pure bounded registration checks; no namespaces, processes or sudo.
set -euo pipefail
[ "$#" = 1 ] || exit 2
out=$1
[ ! -e "$out" ] || { echo "fresh output required" >&2; exit 2; }
mkdir -p "$out"
registration=docs/network/evidence/2026-11-02-runtime-gateway/network-registration.json
driver=tools/gateway-network/runtime-object-netns.sh
bash -n "$driver"
bash -n tools/gateway-network/runtime-object-infra-check.sh
jq '.launch_ready=true' "$registration" > "$out/ready-fixture.json"
bash "$driver" validate "$out/ready-fixture.json" > "$out/positive.stdout" 2> "$out/positive.stderr"
printf '0\n' > "$out/positive.exit"
for name in unregistered trials arms size deliveries setup warmprobe retry rate queue; do
  case "$name" in
    unregistered) mutation='.launch_ready=false';;
    trials) mutation='.dose.trials=[0,1,2,3]';;
    arms) mutation='.arms=["raw","runtime"]';;
    size) mutation='.dose.tiers[1].bytes=16777216';;
    deliveries) mutation='.dose.total_object_deliveries=192';;
    setup) mutation='.dose.total_operations=288';;
    warmprobe) mutation='.dose.warm_probe=false';;
    retry) mutation='.custody.automatic_retries=true';;
    rate) mutation='.profiles[1].rate_bits_per_second_per_direction=100000000';;
    queue) mutation='.topology.queue_limit_packets=1000';;
  esac
  jq "$mutation" "$out/ready-fixture.json" > "$out/$name.json"
  set +e
  bash "$driver" validate "$out/$name.json" > "$out/$name.stdout" 2> "$out/$name.stderr"
  code=$?
  set -e
  printf '%s\n' "$code" > "$out/$name.exit"
  [ "$code" -ne 0 ] || { echo "negative unexpectedly accepted: $name" >&2; exit 1; }
done
printf '{"positive":1,"negatives":10,"passed":true,"scope":"pure registration and shell syntax; original launch gate unchanged; no network claim"}\n' > "$out/result.json"
