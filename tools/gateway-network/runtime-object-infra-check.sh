#!/usr/bin/env bash
# Pure bounded registration checks; no namespaces, processes or sudo.
set -euo pipefail
[ "$#" = 1 ] || [ "$#" = 2 ] || exit 2
out=$1
[ ! -e "$out" ] || { echo "fresh output required" >&2; exit 2; }
mkdir -p "$out"
registration=${2:-docs/network/evidence/2026-11-02-runtime-gateway/network-registration.json}
driver=tools/gateway-network/runtime-object-netns.sh
bash -n "$driver"
bash -n tools/gateway-network/runtime-object-infra-check.sh
jq '.launch_ready=true' "$registration" > "$out/ready-fixture.json"
bash "$driver" validate "$out/ready-fixture.json" > "$out/positive.stdout" 2> "$out/positive.stderr"
printf '0\n' > "$out/positive.exit"
for name in unregistered trials arms size deliveries setup warmprobe retry rate queue schema fixture; do
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
    schema) mutation='.schema="unregistered"';;
    fixture) mutation='.fixture_profile="unregistered"';;
  esac
  jq "$mutation" "$out/ready-fixture.json" > "$out/$name.json"
  set +e
  bash "$driver" validate "$out/$name.json" > "$out/$name.stdout" 2> "$out/$name.stderr"
  code=$?
  set -e
  printf '%s\n' "$code" > "$out/$name.exit"
  [ "$code" -ne 0 ] || { echo "negative unexpectedly accepted: $name" >&2; exit 1; }
done
if jq -e '.fixture_profile=="candidate-cpu"' "$registration" >/dev/null; then
  for name in baseline other_baseline binary_mapping arm_order; do
    case "$name" in
      baseline) mutation='.baseline_revision="main"';;
      other_baseline) mutation='.baseline_revision=(if .schema=="spiralcore.candidate-cpu.registration.v1" then "4a8ec79c9b185c85407e9767653dbcc4af97bf7e" else "4b8cb79fe5fca9e9527a2923924c887d9e7ddbf8" end)';;
      binary_mapping) mutation='.service_binary_by_arm["combined-before"]="candidate"';;
      arm_order) mutation='.dose.mode_order_by_trial[1]=.dose.mode_order_by_trial[0]';;
    esac
    jq "$mutation" "$out/ready-fixture.json" > "$out/$name.json"
    set +e
    bash "$driver" validate "$out/$name.json" > "$out/$name.stdout" 2> "$out/$name.stderr"
    code=$?
    set -e
    printf '%s\n' "$code" > "$out/$name.exit"
    [ "$code" -ne 0 ] || { echo "candidate binding unexpectedly accepted: $name" >&2; exit 1; }
  done
fi
if jq -e '.fixture_profile=="cpu-conscious"' "$registration" >/dev/null; then
  for name in control_pin binary_mapping control_sequence group_dose; do
    case "$name" in
      control_pin) mutation='.control_revision="main"';;
      binary_mapping) mutation='.service_binary_by_arm["combined-cpu"]="other"';;
      control_sequence) mutation='.dose.per_group_sequence|=.[0:6]';;
      group_dose) mutation='.dose.deliveries_per_group=6';;
    esac
    jq "$mutation" "$out/ready-fixture.json" > "$out/$name.json"
    set +e
    bash "$driver" validate "$out/$name.json" > "$out/$name.stdout" 2> "$out/$name.stderr"
    code=$?
    set -e
    printf '%s\n' "$code" > "$out/$name.exit"
    [ "$code" -ne 0 ] || { echo "invalid CPU policy registration accepted: $name" >&2; exit 1; }
  done
fi
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
jq -n '{operations:[{group:"near1m",label:"generated",action:"send",input_bytes:1047202,
  object_id:("00"*32),selected_mode:"RuntimeV1",publication_path:"verified-transfer",
  request_frame_bytes:100,response_frame_bytes:50,data_bytes:40,requests:2,cpu_us:100,
  channel_before:{encoded_batch_bytes_sent:10,encoded_batch_bytes_received:5,framing_bytes_sent:4,framing_bytes_received:4,batches_sent:1,batches_received:1},
  channel_after:{encoded_batch_bytes_sent:110,encoded_batch_bytes_received:55,framing_bytes_sent:12,framing_bytes_received:12,batches_sent:3,batches_received:3}}]}' > "$out/parity-before.json"
jq '.operations[0].cpu_us=50' "$out/parity-before.json" > "$out/parity-after.json"
bash "$driver" receipt-parity "$out/parity-before.json" "$out/parity-after.json" > "$out/parity-positive.json"
for name in mode path bytes framing missing; do
  case "$name" in
    mode) mutation='.operations[0].selected_mode="Raw"';;
    path) mutation='.operations[0].publication_path="verified-cache-hit"';;
    bytes) mutation='.operations[0].data_bytes+=1';;
    framing) mutation='.operations[0].channel_after.framing_bytes_sent+=4';;
    missing) mutation='del(.operations[0].request_frame_bytes)';;
  esac
  jq "$mutation" "$out/parity-after.json" > "$out/parity-$name.json"
  baseline="$out/parity-before.json"
  [ "$name" != missing ] || baseline="$out/parity-$name.json"
  set +e
  bash "$driver" receipt-parity "$baseline" "$out/parity-$name.json" > "$out/parity-$name.stdout" 2> "$out/parity-$name.stderr"
  code=$?
  set -e
  printf '%s\n' "$code" > "$out/parity-$name.exit"
  [ "$code" -ne 0 ] || { echo "changed/missing operation unexpectedly accepted: $name" >&2; exit 1; }
done
bindings=0
if jq -e '.fixture_profile=="candidate-cpu"' "$registration" >/dev/null; then bindings=4; fi
if jq -e '.fixture_profile=="cpu-conscious"' "$registration" >/dev/null; then bindings=4; fi
jq '.operations += [(.operations[0]+{label:"unrelated"})]' "$out/parity-before.json" > "$out/invariants-before.json"
jq '.operations[1].data_bytes+=10' "$out/invariants-before.json" > "$out/invariants-after.json"
bash "$driver" cpu-invariants "$out/invariants-before.json" "$out/invariants-after.json" > "$out/invariants-positive.json"
jq '.operations[0].data_bytes+=1' "$out/invariants-after.json" > "$out/invariants-broken.json"
set +e
bash "$driver" cpu-invariants "$out/invariants-before.json" "$out/invariants-broken.json" > "$out/invariants-negative.stdout" 2> "$out/invariants-negative.stderr"
code=$?
set -e
printf '%s\n' "$code" > "$out/invariants-negative.exit"
[ "$code" -ne 0 ] || { echo "lost mandatory byte savings accepted" >&2; exit 1; }
jq -n --argjson bindings "$bindings" '{positive:1,negatives:12,binding_negatives:$bindings,capture_positive:1,capture_negatives:4,invariants_positive:1,invariants_negative:1,
  parity_positive:1,parity_negatives:5,passed:true,scope:"pure registration, capture completion, receipt parity and shell syntax; no network claim"}' > "$out/result.json"
