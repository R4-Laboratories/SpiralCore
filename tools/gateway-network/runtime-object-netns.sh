#!/usr/bin/env bash
# Reliable shared-object gateway witness; local/shaped, not actual Internet WAN.
# Public command output intentionally belongs to the unprivileged runner.
# shellcheck disable=SC2024
set -euo pipefail
capture_counters() {
  # SIGUSR1 is one line on Linux; final SIGINT output is three lines.
  # Always retain the latest complete values, not an earlier clean snapshot.
  awk '{gsub(/,/, ""); for(i=2;i<=NF;i++) {
    if($i=="packets" && $(i+1)=="captured") c=$(i-1);
    if($i=="packets" && $(i+1)=="received" && $(i+2)=="by" && $(i+3)=="filter") r=$(i-1);
    if($i=="packets" && $(i+1)=="dropped" && $(i+2)=="by" && $(i+3)=="kernel") d=$(i-1);
  }} END {if(c==""||r==""||d=="")exit 1;
    printf "{\"captured\":%s,\"received\":%s,\"dropped\":%s}\n",c,r,d}' "$1"
}
capture_drained() {
  capture_counters "$1" | jq -e 'if .captured>0 and .captured==.received and .dropped==0 then . else false end'
}
validate_registration() {
  jq -e '
    .schema=="spiralcore.runtime-object.registration.v1" and .launch_ready==true and
    .arms==["raw","whole-zstd","warm","runtime"] and
    .profiles==[
      {"name":"lan","delay_ms_per_direction":0,"rate_bits_per_second_per_direction":null},
      {"name":"wan80-10","delay_ms_per_direction":40,"rate_bits_per_second_per_direction":10000000}] and
    .dose.trials==[0,1,2] and .dose.sessions==24 and .dose.topologies==6 and
    .dose.groups_per_session==2 and .dose.deliveries_per_group==6 and
    .dose.total_object_deliveries==288 and .dose.total_operations==312 and
    .dose.operation_counts=={"send":276,"seed":12,"schema":12,"checkpoint":12} and
    .dose.total_requested_object_bytes==2289396384 and .dose.warm_probe==true and
    .dose.tiers==[
      {"label":"near1m","width":4096,"steps":289,"bytes":1047202},
      {"label":"max-trace","width":4096,"steps":4096,"bytes":14851384}] and
    .dose.per_group_sequence==["generated","residual","repeat","unrelated","opaque-edited","opaque-edited-repeat"] and
    .dose.mode_order_by_trial==[["raw","whole-zstd","warm","runtime"],["whole-zstd","warm","runtime","raw"],["warm","runtime","raw","whole-zstd"]] and
    .dose.profile_order_by_trial==[["lan","wan80-10"],["wan80-10","lan"],["lan","wan80-10"]] and
    .topology.queue_limit_packets==256 and .topology.quic_port==4433 and
    .custody.free_disk_floor_bytes==4294967296 and .custody.automatic_retries==false and
    .custody.job_timeout_minutes==40 and .custody.campaign_timeout_minutes==20
  ' "$1" >/dev/null
}
if [ "$#" = 2 ] && [ "$1" = validate ]; then validate_registration "$2"; exit; fi
if [ "$#" = 2 ] && [ "$1" = capture-drained ]; then capture_drained "$2"; exit; fi
[ "$#" = 6 ] && [ "$1" = run ] || { echo "usage: $0 validate REGISTRATION | run NEW_OUT SERVICE PROBE FIXTURES REGISTRATION" >&2; exit 2; }
out=$2; service=$3; probe=$4; fixtures=$5; registration=$6
validate_registration "$registration"
[ "$(uname -s)" = Linux ] || { echo "Linux namespaces required" >&2; exit 2; }
case "$out:$service:$probe:$fixtures" in /*:/*:/*:/*) ;; *) echo "absolute paths required" >&2; exit 2;; esac
[ ! -e "$out" ] && [ -x "$service" ] && [ -x "$probe" ] && [ -x "$fixtures" ]
for tool in ip ss ethtool tcpdump openssl jq timeout sha256sum tc ping iperf3 setpriv; do command -v "$tool" >/dev/null; done
sudo -n true
umask 077
mkdir "$out"
private=$(mktemp -d /tmp/spiralcore-runtime.XXXXXXXX)
ns_a="scp-$$-a"; ns_b="scp-$$-b"; ns_r="scp-$$-r"
owned_namespaces=0
owned_link=0
cap_pid=; cap_job=; a_job=; b_job=; server_job=
cleanup() {
  code=$?
  trap - EXIT INT TERM
  if [ "$owned_namespaces" = 1 ]; then
    # Only processes in our three freshly created namespaces may be signaled.
    for ns in "$ns_a" "$ns_b" "$ns_r"; do
      pids=$(sudo -n ip netns pids "$ns" 2>/dev/null || true)
      while IFS= read -r owned_pid; do
        [[ "$owned_pid" =~ ^[0-9]+$ ]] || continue
        sudo -n kill -TERM "$owned_pid" 2>/dev/null || true
      done <<< "$pids"
    done
    sleep 0.2
    for ns in "$ns_a" "$ns_b" "$ns_r"; do
      pids=$(sudo -n ip netns pids "$ns" 2>/dev/null || true)
      while IFS= read -r owned_pid; do
        [[ "$owned_pid" =~ ^[0-9]+$ ]] || continue
        sudo -n kill -KILL "$owned_pid" 2>/dev/null || true
      done <<< "$pids"
      sudo -n ip netns del "$ns" 2>/dev/null || true
    done
  fi
  if [ "$owned_link" = 1 ]; then
    sudo -n ip link del scga$$ 2>/dev/null || true
    sudo -n ip link del scgb$$ 2>/dev/null || true
    sudo -n ip link del scgrl$$ 2>/dev/null || true
    sudo -n ip link del scgrr$$ 2>/dev/null || true
  fi
  for pid in "$cap_job" "$a_job" "$b_job" "$server_job"; do
    [ -z "$pid" ] || wait "$pid" 2>/dev/null || true
  done
  # This directory was created by this invocation and contains only synthetic
  # fixtures/configuration/private test keys. Public failure evidence stays in out.
  rm -rf -- "$private"
  printf '%s\n' "$code" > "$out/run.exit"
  exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
disk_gate() { [ "$(df -Pk "$out" | awk 'NR==2{print $4}')" -ge 4194304 ]; }
disk_gate
cp "$registration" "$out/registration.json"
sha256sum "$service" "$probe" "$fixtures" "$registration" > "$out/frozen-inputs.sha256"
uname -a > "$out/platform.txt"
lscpu -J > "$out/cpu.json"
sudo -n ip -j route show table all > "$out/host-routes-before.json"
sudo -n ip -j -6 route show table all > "$out/host-routes-v6-before.json"
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -sha256 -days 1 \
  -subj /CN=SpiralCore-test-CA -keyout "$private/ca.key" -out "$private/ca.pem" \
  -addext basicConstraints=critical,CA:TRUE -addext keyUsage=critical,keyCertSign,cRLSign \
  > "$out/pki-generation.log" 2>&1
for side in a b; do
  openssl req -new -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes \
    -subj "/CN=peer-$side.test" -keyout "$private/$side.key" -out "$private/$side.csr" \
    >> "$out/pki-generation.log" 2>&1
  printf 'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth,clientAuth\nsubjectAltName=DNS:peer-%s.test\n' "$side" > "$private/$side.ext"
  openssl x509 -req -in "$private/$side.csr" -CA "$private/ca.pem" -CAkey "$private/ca.key" \
    -CAcreateserial -days 1 -sha256 -extfile "$private/$side.ext" -out "$private/$side.pem" \
    >> "$out/pki-generation.log" 2>&1
  openssl x509 -in "$private/$side.pem" -outform DER -out "$private/$side.der"
  openssl pkcs8 -topk8 -nocrypt -in "$private/$side.key" -outform DER -out "$private/$side-key.der"
done
openssl x509 -in "$private/ca.pem" -outform DER -out "$private/ca.der"
mkdir "$out/public-certificates"
cp "$private/ca.der" "$private/a.der" "$private/b.der" "$out/public-certificates/"
iperf3 --version > "$out/iperf3-version.txt" 2>&1
tc -V > "$out/tc-version.txt"
task_uid=$(id -u); task_gid=$(id -g)
for trial in 0 1 2; do
  disk_gate
  "$fixtures" "$private/fixtures-$trial" "$trial" > "$out/fixture-generation-$trial.json"
  mkdir "$out/fixtures-$trial"
  cp "$private/fixtures-$trial/"*.json "$private/fixtures-$trial/"*checkpoint.bin \
    "$private/fixtures-$trial/"*recipe.bin "$private/fixtures-$trial/schema.bin" "$out/fixtures-$trial/"
done
wait_file() {
  file=$1
  for _ in $(seq 1 200); do
    if [ -s "$file" ]; then return; fi
    sleep 0.1
  done
  echo "missing ready/receipt: $file" >&2; return 1
}
wait_job() {
  job=$1
  for _ in $(seq 1 300); do
    if ! kill -0 "$job" 2>/dev/null; then wait "$job"; return; fi
    sleep 0.1
  done
  echo "owned process shutdown timeout" >&2; return 1
}
disable_offloads() {
  ns=$1; link=$2; label=$3; dir=$4
  sudo -n ip netns exec "$ns" ethtool -K "$link" tso off gso off gro off > "$dir/offload-$label-change.txt" 2>&1
  sudo -n ip netns exec "$ns" ethtool -K "$link" tx-udp-segmentation off rx-udp-gro-forwarding off \
    >> "$dir/offload-$label-change.txt" 2>&1 || true
  sudo -n ip netns exec "$ns" ethtool -k "$link" > "$dir/offload-$label-state.txt"
  if grep -Eq '^[[:space:]]*(generic-segmentation-offload|generic-receive-offload|tx-udp-segmentation): on' "$dir/offload-$label-state.txt"; then
    echo "segmentation offload still enabled: $label" >&2; return 1
  fi
}
qdisc_record() {
  dir=$1; phase=$2
  for link in left right; do
    sudo -n ip netns exec "$ns_r" tc -s -j qdisc show dev "$link" > "$dir/tc-$link-$phase.json"
  done
}
qdisc_set() {
  profile=$1; dir=$2; phase=$3
  if [ "$profile" = wan80-10 ]; then
    for link in left right; do
      # Recreate, rather than replace, to reset calibration counters.
      if sudo -n ip netns exec "$ns_r" tc qdisc show dev "$link" | grep -q '^qdisc netem '; then
        sudo -n ip netns exec "$ns_r" tc qdisc del dev "$link" root
      fi
      sudo -n ip netns exec "$ns_r" tc qdisc add dev "$link" root handle 10: netem limit 256 delay 40ms rate 10mbit
    done
  fi
  qdisc_record "$dir" "$phase"
}
setup_topology() {
  dir=$1; profile=$2
  for ns in "$ns_a" "$ns_b" "$ns_r"; do
    if sudo -n ip netns list | awk '{print $1}' | grep -Fx "$ns"; then
      echo "namespace already exists: $ns" >&2; return 1
    fi
  done
  sudo -n ip netns add "$ns_a"; owned_namespaces=1
  sudo -n ip netns add "$ns_b"
  sudo -n ip netns add "$ns_r"
  sudo -n ip link add scga$$ type veth peer name scgrl$$; owned_link=1
  sudo -n ip link add scgb$$ type veth peer name scgrr$$
  sudo -n ip link set scga$$ netns "$ns_a"
  sudo -n ip link set scgb$$ netns "$ns_b"
  sudo -n ip link set scgrl$$ netns "$ns_r"
  sudo -n ip link set scgrr$$ netns "$ns_r"
  sudo -n ip -n "$ns_a" link set scga$$ name underlay
  sudo -n ip -n "$ns_b" link set scgb$$ name underlay
  sudo -n ip -n "$ns_r" link set scgrl$$ name left
  sudo -n ip -n "$ns_r" link set scgrr$$ name right
  sudo -n ip -n "$ns_a" addr add 10.203.0.1/30 dev underlay
  sudo -n ip -n "$ns_b" addr add 10.203.0.6/30 dev underlay
  sudo -n ip -n "$ns_r" addr add 10.203.0.2/30 dev left
  sudo -n ip -n "$ns_r" addr add 10.203.0.5/30 dev right
  for ns in "$ns_a" "$ns_b" "$ns_r"; do sudo -n ip -n "$ns" link set lo up; done
  sudo -n ip -n "$ns_a" link set underlay up
  sudo -n ip -n "$ns_b" link set underlay up
  sudo -n ip -n "$ns_r" link set left up
  sudo -n ip -n "$ns_r" link set right up
  sudo -n ip -n "$ns_a" route add 10.203.0.4/30 via 10.203.0.2
  sudo -n ip -n "$ns_b" route add 10.203.0.0/30 via 10.203.0.5
  sudo -n ip netns exec "$ns_r" sysctl -w net.ipv4.ip_forward=1 > "$dir/router-forwarding.txt"
  disable_offloads "$ns_a" underlay a "$dir"
  disable_offloads "$ns_b" underlay b "$dir"
  disable_offloads "$ns_r" left router-left "$dir"
  disable_offloads "$ns_r" right router-right "$dir"
  for side in a b; do
    ns=$ns_a; [ "$side" != b ] || ns=$ns_b
    sudo -n ip -n "$ns" -j addr > "$dir/addresses-$side.json"
    sudo -n ip -n "$ns" -j route show table all > "$dir/routes-$side.json"
  done
  sudo -n ip -n "$ns_r" -j addr > "$dir/addresses-router.json"
  sudo -n ip -n "$ns_r" -j route show table all > "$dir/routes-router.json"
  qdisc_set "$profile" "$dir" calibration-before
}
calibrate() {
  dir=$1; profile=$2
  for side in a b; do
    ns=$ns_a; peer=10.203.0.6
    if [ "$side" = b ]; then ns=$ns_b; peer=10.203.0.1; fi
    sudo -n ip netns exec "$ns" ping -n -c 20 -i 0.05 -W 2 "$peer" > "$dir/ping-$side.txt"
    awk '{for(i=1;i<=NF;i++)if($i~/^time=/){sub(/^time=/,"",$i);print $i}}' "$dir/ping-$side.txt" \
      | jq -s --arg profile "$profile" \
        '{profile:$profile,samples:.,count:length,retained:(.[4:]),mean_ms:((.[4:]|add)/16)}' > "$dir/ping-$side.json"
    jq -e --arg profile "$profile" \
      '.count==20 and (.retained|length)==16 and
       (if $profile=="wan80-10" then .mean_ms>=75 and .mean_ms<=90 else .mean_ms<20 end)' \
      "$dir/ping-$side.json" > "$dir/ping-$side-accepted.txt"
  done
  if [ "$profile" = wan80-10 ]; then
    for direction in forward reverse; do
      sudo -n ip netns exec "$ns_b" timeout --signal=TERM --kill-after=2 20 \
        iperf3 -s -1 -B 10.203.0.6 -p 5201 -J > "$dir/iperf-$direction-server.json" 2> "$dir/iperf-$direction-server.stderr" &
      server_job=$!
      for _ in $(seq 1 100); do
        sudo -n ip netns exec "$ns_b" ss -lnt | grep -q ':5201 ' && break
        sleep 0.05
      done
      sudo -n ip netns exec "$ns_b" ss -lnt | grep -q ':5201 '
      reverse=(); [ "$direction" != reverse ] || reverse=(-R)
      sudo -n ip netns exec "$ns_a" timeout --signal=TERM --kill-after=2 20 \
        iperf3 -c 10.203.0.6 -p 5201 -t 3 -O 1 -J --get-server-output "${reverse[@]}" \
        > "$dir/iperf-$direction-client.json" 2> "$dir/iperf-$direction-client.stderr"
      wait "$server_job"; server_job=
      jq -e '.error==null and .end.sum_received.bits_per_second>=8500000 and .end.sum_received.bits_per_second<=10200000' \
        "$dir/iperf-$direction-client.json" > "$dir/iperf-$direction-accepted.txt"
    done
  fi
  qdisc_record "$dir" calibration-after
  qdisc_set "$profile" "$dir" measurement-before
  jq -n --arg profile "$profile" \
    '{profile:$profile,accepted:true,scope:"underlay calibration before QUIC capture; calibration traffic excluded; fresh qdisc counters for measurement"}' \
    > "$dir/calibration.json"
}
config() {
  side=$1; arm=$2; trial=$3; dir=$4; scope=$5
  if [ "$side" = client ]; then role=connect; bind=10.203.0.1:4433; peer=10.203.0.6:4433; identity=a; other=b
  else role=listen; bind=10.203.0.6:4433; peer=10.203.0.1:4433; identity=b; other=a; fi
  jq -n --arg role "$role" --arg bind "$bind" --arg peer "$peer" --arg identity "$identity" --arg other "$other" \
    --arg side "$side" --arg private "$private" --arg dir "$dir" --arg scope "$scope" \
    --slurpfile groups "$private/fixtures-$trial/groups-$arm.json" --slurpfile verify "$private/fixtures-$trial/verify-files.json" \
    '{role:$role,bind:$bind,peer:$peer,server_name:("peer-"+$other+".test"),
      certificate:($private+"/"+$identity+".der"),private_key:($private+"/"+$identity+"-key.der"),
      trust_anchor:($private+"/ca.der"),peer_certificate:($private+"/"+$other+".der"),
      scope:$scope,policy_epoch:1,store_dir:($private+"/store-"+$scope+"-"+$side),
      stats_path:($dir+"/"+$side+".json"),ready_path:($dir+"/ready-"+$side+".json"),duration_secs:300,
      groups:(if $role=="connect" then $groups[0] else [] end),
      verify_files:(if $role=="listen" then $verify[0] else [] end)}' > "$private/$side-config.json"
}
check_session() {
  dir=$1; trial=$2; arm=$3
  jq -n -e --arg arm "$arm" --slurpfile c "$dir/client.json" --slurpfile s "$dir/server.json" \
    --slurpfile m "$out/fixtures-$trial/manifest.json" '
    $c[0] as $c | $s[0] as $s | $m[0] as $m |
    $c.status=="ok" and $s.status=="ok" and $c.role=="connect" and $s.role=="listen" and
    $c.namespace==$s.namespace and ($c.groups|length)==2 and
    all($c.groups[];.mode==$arm) and
    all($m.tiers[];
      .label as $label |
      [$c.operations[]|select(.group==$label)|.label]==
        ((if $arm=="warm" then ["schema"] elif $arm=="runtime" then ["checkpoint"] else [] end)+
         ["generated","residual","repeat","unrelated","opaque-edited","opaque-edited-repeat"])) and
    (if $arm=="runtime" then
       all($c.operations[]|select(.label=="generated" or .label=="residual");
         .selected_mode=="RuntimeV1" and .publication_path=="verified-transfer")
     elif $arm=="warm" then
       all($c.operations[]|select(.label=="opaque-edited");
         .selected_mode=="SparseV1" and .publication_path=="verified-transfer")
     else true end) and
    ($c.operations|length)==(if $arm=="warm" or $arm=="runtime" then 14 else 12 end) and
    all($c.operations[];
      . as $op | .success==true and .input_bytes>=0 and .cpu_us>=0 and .wall_us>=0 and
      (if .action=="schema" then .object_id==$m.schema.object_id and .input_bytes==$m.schema.bytes
       else ($m.tiers|map(select(.label==$op.group))|.[0]) as $tier |
         (if .action=="checkpoint" then $tier.checkpoint
          elif .label=="generated" then $tier.generated
          elif .label=="residual" or .label=="repeat" then $tier.residual
          elif .label=="unrelated" then $tier.opaque
          elif .label=="opaque-edited" or .label=="opaque-edited-repeat" then $tier.opaque_edited
          else null end) as $expected |
         $expected!=null and .object_id==$expected.object_id and .input_bytes==$expected.bytes end)) and
    $s.readback.reopened and $s.readback.expected_files_exact==8 and
    all($s.readback.objects[];.digest_verified) and all($s.server_batches[];.accepted) and
    ($s.readback.objects|map(.object_id)|sort)==($c.operations|map(.object_id)|unique|sort) and
    $c.channel.encoded_batch_bytes_sent==$s.channel.encoded_batch_bytes_received and
    $c.channel.encoded_batch_bytes_received==$s.channel.encoded_batch_bytes_sent and
    $c.channel.framing_bytes_sent==$s.channel.framing_bytes_received and
    $c.channel.framing_bytes_received==$s.channel.framing_bytes_sent
  ' > "$dir/exact-accepted.txt"
}
run_session() {
  profile=$1; trial=$2; arm=$3; topology_dir=$4
  disk_gate
  scope="runtime-$profile-t$trial-$arm"
  dir="$out/$profile-t$trial-$arm"; mkdir "$dir"
  session_start_ns=$(date +%s%N)
  date -u +%FT%TZ > "$dir/start.utc"
  jq -n --arg profile "$profile" --argjson trial "$trial" --arg arm "$arm" --arg calibration "$(basename "$topology_dir")" \
    '{profile:$profile,trial:$trial,arm:$arm,calibration_directory:$calibration,groups:2,object_deliveries:12}' > "$dir/session.json"
  qdisc_set "$profile" "$dir" measurement-before
  for side in client server; do
    mkdir "$private/store-$scope-$side"
    config "$side" "$arm" "$trial" "$dir" "$scope"
  done
  touch "$dir/capture.pid" "$dir/underlay.pcap"
  sudo -n ip netns exec "$ns_a" sh -c 'echo "$$" > "$1"; exec tcpdump --immediate-mode -Z root -i underlay -nn -s 96 -B 4096 -U -w "$2" "udp port 4433"' \
    capture "$dir/capture.pid" "$dir/underlay.pcap" > "$dir/capture.stdout" 2> "$dir/capture.stderr" &
  cap_job=$!
  for _ in $(seq 1 100); do grep -q 'listening on underlay' "$dir/capture.stderr" && break; sleep 0.1; done
  grep -q 'listening on underlay' "$dir/capture.stderr"
  cap_pid=$(cat "$dir/capture.pid")
  sudo -n ip netns exec "$ns_b" setpriv --reuid "$task_uid" --regid "$task_gid" --clear-groups -- \
    timeout --signal=TERM --kill-after=5 320 "$service" run "$private/server-config.json" \
    > "$dir/server.stdout" 2> "$dir/server.stderr" &
  b_job=$!
  wait_file "$dir/ready-server.json"
  jq -e '.listening' "$dir/ready-server.json" >/dev/null
  sudo -n ip netns exec "$ns_a" setpriv --reuid "$task_uid" --regid "$task_gid" --clear-groups -- \
    timeout --signal=TERM --kill-after=5 320 "$service" run "$private/client-config.json" \
    > "$dir/client.stdout" 2> "$dir/client.stderr" &
  a_job=$!
  wait "$a_job"; a_job=
  wait "$b_job"; b_job=
  wait_file "$dir/client.json"; wait_file "$dir/server.json"
  check_session "$dir" "$trial" "$arm"
  # Both service processes have exited. Drain capture delivery rather than
  # assuming a fixed delay flushed libpcap's kernel buffer. Retain every poll.
  capture_stable=0
  capture_previous=
  for poll in $(seq 1 100); do
    sudo -n kill -USR1 "$cap_pid"
    sleep 0.1
    if capture_drained "$dir/capture.stderr" > "$dir/capture-drain-$poll.json"; then
      capture_snapshot=$(cat "$dir/capture-drain-$poll.json")
      if [ "$capture_snapshot" = "$capture_previous" ]; then capture_stable=$((capture_stable+1)); else capture_stable=1; fi
      capture_previous=$capture_snapshot
      [ "$capture_stable" -lt 2 ] || break
    else
      capture_stable=0
      capture_previous=
    fi
  done
  [ "$capture_stable" -eq 2 ] || { echo "capture did not drain within bounded wait" >&2; return 1; }
  sudo -n kill -INT "$cap_pid"
  wait_job "$cap_job"; cap_job=
  "$probe" pcap-summary "$dir/underlay.pcap" "$dir/capture.json"
  capture_counters "$dir/capture.stderr" > "$dir/capture-counters.json"
  jq -e --slurpfile c "$dir/capture-counters.json" \
    '.other_packets==0 and .packets>0 and .packets==$c[0].captured and $c[0].captured==$c[0].received and $c[0].dropped==0' \
    "$dir/capture.json" > "$dir/capture-accepted.txt"
  qdisc_record "$dir" measurement-after
  if [ "$profile" = wan80-10 ]; then
    for link in left right; do
      jq -e '[.[]|select(.kind=="netem")]|length==1 and .[0].qlen==0 and .[0].drops>=0 and
        .[0].options.limit==256 and .[0].options.delay.delay==0.04 and .[0].options.rate.rate==1250000' \
        "$dir/tc-$link-measurement-after.json" > "$dir/tc-$link-accepted.txt"
    done
  fi
  du -sk "$private/store-$scope-client" "$private/store-$scope-server" > "$dir/store-allocated-kib.txt"
  # These exact paths were newly created above and are runner-owned after setpriv.
  rm -rf -- "$private/store-$scope-client" "$private/store-$scope-server"
  cap_pid=
  date -u +%FT%TZ > "$dir/end.utc"
  session_end_ns=$(date +%s%N)
  jq -n --slurpfile c "$dir/client.json" --argjson wall "$(((session_end_ns-session_start_ns)/1000))" \
    '{ok:true,stores_removed:true,object_deliveries:12,operations:($c[0].operations|length),harness_wall_us:$wall,
      wall_scope:"fresh stores, qdisc reset, capture startup, service handshake/preparation/input/publication/reopened readback, close/drain/parse and store cleanup; shared topology/calibration and fixture generation reported separately"}' > "$dir/complete.json"
  printf '%s exact reopened objects and complete capture accepted\n' "$(basename "$dir")"
}
for trial in 0 1 2; do
  profiles=$(jq -r --argjson trial "$trial" '.dose.profile_order_by_trial[$trial][]' "$registration")
  arms=$(jq -r --argjson trial "$trial" '.dose.mode_order_by_trial[$trial][]' "$registration")
  for profile in $profiles; do
    topology_dir="$out/topology-$profile-t$trial"; mkdir "$topology_dir"
    topology_start_ns=$(date +%s%N)
    setup_topology "$topology_dir" "$profile"
    calibrate "$topology_dir" "$profile"
    calibrated_ns=$(date +%s%N)
    for arm in $arms; do run_session "$profile" "$trial" "$arm" "$topology_dir"; done
    for ns in "$ns_a" "$ns_b" "$ns_r"; do sudo -n ip netns del "$ns"; done
    owned_namespaces=0; owned_link=0
    topology_end_ns=$(date +%s%N)
    jq -n --argjson setup "$(((calibrated_ns-topology_start_ns)/1000))" \
      --argjson total "$(((topology_end_ns-topology_start_ns)/1000))" \
      '{ok:true,namespace_cleanup:true,setup_calibration_wall_us:$setup,topology_all_four_sessions_wall_us:$total}' > "$topology_dir/complete.json"
  done
done
sudo -n ip -j route show table all > "$out/host-routes-after.json"
sudo -n ip -j -6 route show table all > "$out/host-routes-v6-after.json"
cmp "$out/host-routes-before.json" "$out/host-routes-after.json"
cmp "$out/host-routes-v6-before.json" "$out/host-routes-v6-after.json"
sha256sum -c "$out/frozen-inputs.sha256" > "$out/frozen-inputs-after.txt"
jq -s '{sessions:length,operations:(map(.operations)|add),object_deliveries:(map(.object_deliveries)|add),
  exact_reopened:all(.[];.ok),stores_removed:all(.[];.stores_removed),capture_complete:true,host_routes_unchanged:true}' \
  "$out"/lan-t*/complete.json "$out"/wan80-10-t*/complete.json > "$out/complete.json"
jq -e '.sessions==24 and .operations==312 and .object_deliveries==288 and .exact_reopened and .stores_removed' "$out/complete.json" >/dev/null
