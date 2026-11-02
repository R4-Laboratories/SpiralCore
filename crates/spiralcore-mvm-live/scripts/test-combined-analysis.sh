#!/usr/bin/env bash
# Small synthetic accounting fixtures; these are not transfer measurements.
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/spiralcore-combined-analysis.XXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
jq -n '[range(0;3) as $trial | ["one","two"][] as $stage | [1048576,16777216][] as $size | ["combined","combined-cpu","whole-mixed"][] as $arm | {name:($stage+"-"+($size|tostring)+"-t"+($trial|tostring)+"-"+$arm),stage:$stage,size:$size,trial:$trial,arm:$arm,bandwidth_profile:"experimental64"}]' > "$scratch/schedule.json"
while IFS= read -r item; do
    name=$(jq -r .name <<< "$item")
    mkdir -- "$scratch/$name"
    jq -n --argjson item "$item" '
      ($item | del(.name)) as $plan |
      ["generated","residual","repeat","unrelated","opaque-edited","opaque-edited-repeat","schema-fit","schema-drift"] as $common |
      (if $plan.arm == "whole-mixed" then $common else ["runtime-checkpoint","schema"] + $common end) as $kinds |
      (if $plan.arm == "whole-mixed" then 200 else 100 end) as $bytes |
      {envelope_bytes_sent:$bytes,envelope_bytes_received:20,framing_bytes_sent:4,framing_bytes_received:4,frames_sent:1,frames_received:1} as $sc |
      {envelope_bytes_sent:20,envelope_bytes_received:$bytes,framing_bytes_sent:4,framing_bytes_received:4,frames_sent:1,frames_received:1} as $rc |
      [$kinds | to_entries[] | {event:"sender_operation",position:.key,kind:.value,object:(.value+"-"+($plan.size|tostring)),bytes:$plan.size,mode:"RawV1",cpu_us:10,wall_us:30,exchange_cpu_us:3,exchange_wall_us:20,counters:$sc,application_request_bytes:30,application_response_bytes:10}] as $se |
      [$se[] | .event="receiver_operation" | .cpu_us=5 | .wall_us=10 | .counters=$rc] as $re |
      [$se[] | {event:"verified",position,kind,object,bytes,exact:true,read_only:true}] as $ve |
      ($kinds|length) as $n |
      ($sc|map_values(. * $n)) as $st |
      ($rc|map_values(. * $n)) as $rt |
      {event:"session_done",ok:true,plan:$plan,mailbox_rate_bytes_per_second:67108864,mailbox_burst_bytes:8388608,
       sender:{operations:$n,cpu_us:($n*10+2),wall_us:($n*30+4),counters:$st},
       receiver:{operations:$n,cpu_us:($n*5+2),wall_us:($n*10+3),counters:$rt,exact_reopened:true},
       sender_worker:{guest_events:$se,worker_process_cpu_us:1000,ok:true,mailbox_bandwidth_profile:"experimental64",mailbox_rate_bytes_per_second:67108864,mailbox_burst_bytes:8388608},
       receiver_worker:(if $plan.stage=="two" then {guest_events:($re+$ve),worker_process_cpu_us:1000,ok:true,backing_disk_synced:true,mailbox_bandwidth_profile:"experimental64",mailbox_rate_bytes_per_second:67108864,mailbox_burst_bytes:8388608} else null end),
       recovery_worker:(if $plan.stage=="two" then {guest_events:$ve,worker_process_cpu_us:500,ok:true,backing_disk_synced:true,mailbox_bandwidth_profile:"experimental64",mailbox_rate_bytes_per_second:67108864,mailbox_burst_bytes:8388608} else null end),
       relay:(if $plan.stage=="two" then {cpu_us:6,wall_us:7} else null end),
       logical_envelope_and_framing_bytes:(($bytes+28)*$n),
       physical_vsock_stream_bytes:(($bytes+28)*$n*(if $plan.stage=="two" then 2 else 1 end)),
       orchestrator_process_cpu_us:1200,lifecycle_wall_us:3000,_host:($re+$ve)}
    ' > "$scratch/$name/full.json"
    jq 'del(._host)' "$scratch/$name/full.json" > "$scratch/$name/result.json"
    jq -c '._host[]' "$scratch/$name/full.json" > "$scratch/$name/host-events.jsonl"
    rm -- "$scratch/$name/full.json"
done < <(jq -c '.[]' "$scratch/schedule.json")
"$here/analyze-combined.sh" "$scratch" > "$scratch/analysis.json"
jq -e '.sessions|length==36' "$scratch/analysis.json" >/dev/null
jq -e '.operation_rows|length==336' "$scratch/analysis.json" >/dev/null
jq -e '.payload_identity_groups==96 and .exact_operations==336 and .guest_reboot_verified_operations==168' "$scratch/analysis.json" >/dev/null
jq -e '.operation_comparisons|length==96' "$scratch/analysis.json" >/dev/null
jq -e 'all(.operation_comparisons[]|select(.candidate=="combined" and .baseline=="whole-mixed"); .median_paired_logical_bytes_ratio==(128/228) and .median_paired_sender_cpu_ratio==1)' "$scratch/analysis.json" >/dev/null
jq -e 'all(.session_comparisons[]|select(.candidate=="combined" and .baseline=="whole-mixed"); .median_paired_logical_bytes_ratio==(1280/1824) and .median_paired_elapsed_ratio==(304/244))' "$scratch/analysis.json" >/dev/null
jq -e 'all(.session_comparisons[]|select(.candidate=="combined-cpu" and .baseline=="combined"); .median_paired_logical_bytes_ratio==1 and .median_paired_elapsed_ratio==1 and .median_paired_sender_cpu_ratio==1)' "$scratch/analysis.json" >/dev/null
target="$scratch/two-16777216-t2-combined/result.json"
cp -- "$target" "$scratch/original.json"
jq '.recovery_worker.guest_events[0].exact=false' "$scratch/original.json" > "$target"
if "$here/analyze-combined.sh" "$scratch" > /dev/null 2> "$scratch/negative.txt"; then
    printf 'analyzer accepted failed reboot verification\n' >&2; exit 1
fi
cp -- "$scratch/original.json" "$target"
jq '.sender_worker.guest_events[3].object="wrong-object"' "$scratch/original.json" > "$target"
if "$here/analyze-combined.sh" "$scratch" > /dev/null 2> "$scratch/negative.txt"; then
    printf 'analyzer accepted mismatched payload identity\n' >&2; exit 1
fi
cp -- "$scratch/original.json" "$target"
jq '.sender.counters.envelope_bytes_sent+=1' "$scratch/original.json" > "$target"
if "$here/analyze-combined.sh" "$scratch" > /dev/null 2> "$scratch/negative.txt"; then
    printf 'analyzer accepted inconsistent directional counters\n' >&2; exit 1
fi
cp -- "$scratch/original.json" "$target"
jq '.recovery_worker.backing_disk_synced=false' "$scratch/original.json" > "$target"
if "$here/analyze-combined.sh" "$scratch" > /dev/null 2> "$scratch/negative.txt"; then
    printf 'analyzer accepted unsynchronized recovery disk\n' >&2; exit 1
fi
cp -- "$scratch/original.json" "$target"
jq '.sender_worker.mailbox_rate_bytes_per_second=4194304' "$scratch/original.json" > "$target"
if "$here/analyze-combined.sh" "$scratch" > /dev/null 2> "$scratch/negative.txt"; then
    printf 'analyzer accepted mismatched worker rate\n' >&2; exit 1
fi
cp -- "$scratch/original.json" "$target"
jq 'del(.sender.counters.envelope_bytes_received,.receiver.counters.envelope_bytes_sent)' "$scratch/original.json" > "$target"
if "$here/analyze-combined.sh" "$scratch" > /dev/null 2> "$scratch/negative.txt"; then
    printf 'analyzer accepted missing directional byte fields\n' >&2; exit 1
fi
printf 'PASS: 36 synthetic sessions, known paired ratios, exact readback and six rejecting mutations\n'
