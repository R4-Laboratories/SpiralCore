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
printf 'tcpdump: 10 packets captured, 10 packets received by filter, 0 packets dropped by kernel\n' > "$out/capture-ok.stderr"
bash "$driver" capture-drained "$out/capture-ok.stderr" > "$out/capture-positive.stdout" 2> "$out/capture-positive.stderr"
for name in tail drop empty missing; do
  case "$name" in
    tail) printf 'tcpdump: 49123 packets captured, 49130 packets received by filter, 0 packets dropped by kernel\n' > "$out/capture-$name.stderr";;
    drop) printf '10 packets captured\n10 packets received by filter\n1 packets dropped by kernel\n' > "$out/capture-$name.stderr";;
    empty) printf '0 packets captured\n0 packets received by filter\n0 packets dropped by kernel\n' > "$out/capture-$name.stderr";;
    missing) printf 'tcpdump: listening on underlay\n' > "$out/capture-$name.stderr";;
  esac
  set +e
  bash "$driver" capture-drained "$out/capture-$name.stderr" > "$out/capture-$name.stdout" 2> "$out/capture-$name.error"
  code=$?
  set -e
  printf '%s\n' "$code" > "$out/capture-$name.exit"
  [ "$code" -ne 0 ] || { echo "undrained capture unexpectedly accepted: $name" >&2; exit 1; }
done
printf '{"positive":1,"negatives":10,"capture_positive":1,"capture_negatives":4,"passed":true,"scope":"pure registration, capture completion and shell syntax; no network claim"}\n' > "$out/result.json"
