#!/usr/bin/env bash
# Four-arm packet selector campaign. Shaped local network, not an Internet WAN.
# Public command output intentionally belongs to the unprivileged runner.
# shellcheck disable=SC2024
set -euo pipefail
validate_registration() {
  jq -e '
    .schema=="spiralcore.packet-selection.registration.v1" and .launch_ready==true and
    .modes==["raw","zstd","fast-zstd","adaptive"] and
    .profiles==[
      {"name":"lan","delay_ms_per_direction":0,"rate_bits_per_second_per_direction":null},
      {"name":"wan80-10","delay_ms_per_direction":40,"rate_bits_per_second_per_direction":10000000}] and
    .dose.trials==[0,1,2] and .dose.primary_ip_version==4 and .dose.primary_sessions==24 and
    .dose.smoke_ip_version==6 and .dose.smoke_sessions==8 and .dose.smoke_trial==0 and
    .dose.total_sessions==32 and .dose.total_transport_flows==64 and .dose.total_application_transfers==128 and
    .dose.total_application_input_bytes==1233256448 and
    .dose.primary_transfers==[
      {"name":"tcp-pattern-before","protocol":"tcp","fixture":"pattern","bytes":16777216},
      {"name":"tcp-opaque","protocol":"tcp","fixture":"opaque","bytes":16777216},
      {"name":"tcp-pattern-after","protocol":"tcp","fixture":"pattern","bytes":16777216},
      {"name":"udp-smoke","protocol":"udp","fixture":"opaque","bytes":4096}] and
    .dose.smoke_transfers==[
      {"name":"tcp-pattern-before","protocol":"tcp","fixture":"pattern","bytes":1048576},
      {"name":"tcp-opaque","protocol":"tcp","fixture":"opaque","bytes":1048576},
      {"name":"tcp-pattern-after","protocol":"tcp","fixture":"pattern","bytes":1048576},
      {"name":"udp-smoke","protocol":"udp","fixture":"opaque","bytes":4096}] and
    .dose.mode_order_by_trial==[
      ["raw","zstd","fast-zstd","adaptive"],
      ["zstd","fast-zstd","adaptive","raw"],
      ["fast-zstd","adaptive","raw","zstd"]] and
    .dose.profile_order_by_trial==[["lan","wan80-10"],["wan80-10","lan"],["lan","wan80-10"]] and
    .topology.tun_mtu==1280 and .topology.quic_port==4433 and
    .topology.queue_limit_packets==256 and
    .custody.automatic_retries==false and .custody.free_disk_floor_bytes==4294967296 and
    .custody.job_timeout_minutes==45 and .custody.campaign_timeout_minutes==25
  ' "$1" >/dev/null
}
validate_gateway_stats() {
  jq -e '.status=="ok" and .denied_packets==0 and .invalid_frames==0 and .capacity_drops==0 and
    (.expired_packets|type)=="number" and .expired_packets>=0 and (.expired_packets|floor)==.expired_packets' "$1"
}
validate_qdisc_stats() {
  jq -e '[.[]|select(.kind=="netem")]|length==1 and
    (.[0].drops|type)=="number" and .[0].drops>=0 and (.[0].drops|floor)==.[0].drops and .[0].qlen==0' "$1"
}
if [ "$#" = 3 ] && [ "$1" = check-statistics ]; then
  case "$2" in gateway) validate_gateway_stats "$3";; qdisc) validate_qdisc_stats "$3";; *) exit 2;; esac
  exit
fi
if [ "$#" = 2 ] && [ "$1" = validate ]; then validate_registration "$2"; exit; fi
[ "$#" = 5 ] && [ "$1" = run ] || { echo "usage: $0 validate REGISTRATION | run NEW_OUT GATEWAY PROBE REGISTRATION" >&2; exit 2; }
out=$2; gateway=$3; probe=$4; registration=$5
validate_registration "$registration"
[ "$(uname -s)" = Linux ] || { echo "Linux namespaces required" >&2; exit 2; }
case "$out:$gateway:$probe" in /*:/*:/*) ;; *) echo "absolute paths required" >&2; exit 2;; esac
[ ! -e "$out" ] && [ -x "$gateway" ] && [ -x "$probe" ]
for tool in ip ss ethtool tcpdump openssl jq timeout sha256sum tc ping iperf3; do command -v "$tool" >/dev/null; done
sudo -n true
test -c /dev/net/tun
umask 077
mkdir "$out"
private=$(mktemp -d /tmp/spiralcore-packet.XXXXXXXX)
ns_a="scp-$$-a"; ns_b="scp-$$-b"; ns_r="scp-$$-r"
owned_namespaces=0
owned_link=0
cap_pid=; daemon_a=; daemon_b=; cap_job=; a_job=; b_job=; server_job=
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
sha256sum "$gateway" "$probe" "$registration" > "$out/frozen-inputs.sha256"
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
for trial in 0 1 2; do
  for kind in pattern opaque; do
    "$probe" fixture "$private/primary-$kind-$trial.bin" 16777216 "$kind" "$((1729+trial))" > "$out/fixture-primary-$kind-$trial.json"
  done
  "$probe" fixture "$private/udp-$trial.bin" 4096 opaque "$((1729+trial))" > "$out/fixture-udp-$trial.json"
done
for kind in pattern opaque; do
  "$probe" fixture "$private/smoke-$kind-0.bin" 1048576 "$kind" 1729 > "$out/fixture-smoke-$kind-0.json"
done
wait_file() {
  file=$1
  for _ in $(seq 1 200); do
    if [ -s "$file" ]; then sudo -n chmod 0644 "$file"; return; fi
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
config() {
  side=$1; mode=$2; dir=$3
  if [ "$side" = a ]; then role=connect; local4=10.204.0.1; remote4=10.204.0.2; local6=fd42:5343::1; remote6=fd42:5343::2; bind=10.203.0.1:4433; peer=10.203.0.6:4433; other=b
  else role=listen; local4=10.204.0.2; remote4=10.204.0.1; local6=fd42:5343::2; remote6=fd42:5343::1; bind=10.203.0.6:4433; peer=10.203.0.1:4433; other=a; fi
  jq -n --arg role "$role" --arg bind "$bind" --arg peer "$peer" --arg other "$other" --arg side "$side" \
    --arg private "$private" --arg dir "$dir" --arg mode "$mode" --arg l4 "$local4" --arg r4 "$remote4" --arg l6 "$local6" --arg r6 "$remote6" \
    '{role:$role,bind:$bind,peer:$peer,server_name:("peer-"+$other+".test"),
      certificate:($private+"/"+$side+".der"),private_key:($private+"/"+$side+"-key.der"),
      trust_anchor:($private+"/ca.der"),peer_certificate:($private+"/"+$other+".der"),tun:"sc0",mode:$mode,
      local_addresses:[$l4,$l6],peer_addresses:[$r4,$r6],stats_path:($dir+"/gateway-"+$side+".json"),
      ready_path:($dir+"/ready-"+$side+".json"),duration_secs:300}' > "$private/$side-config.json"
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
    ns=$ns_a; index=1; other=2
    if [ "$side" = b ]; then ns=$ns_b; index=2; other=1; fi
    sudo -n ip -n "$ns" tuntap add dev sc0 mode tun
    sudo -n ip -n "$ns" link set sc0 mtu 1280
    sudo -n ip -n "$ns" link set dev sc0 addrgenmode none
    sudo -n ip netns exec "$ns" sysctl -w net.ipv6.conf.sc0.autoconf=0 net.ipv6.conf.sc0.accept_ra=0 \
      net.ipv6.conf.sc0.router_solicitations=0 net.ipv6.conf.sc0.dad_transmits=0 > "$dir/tun-ipv6-$side.txt"
    sudo -n ip -n "$ns" addr add "10.204.0.$index/32" dev sc0
    sudo -n ip -n "$ns" -6 addr add "fd42:5343::$index/128" dev sc0 nodad
    sudo -n ip -n "$ns" link set sc0 up
    sudo -n ip -n "$ns" route add "10.204.0.$other/32" dev sc0
    sudo -n ip -n "$ns" -6 route add "fd42:5343::$other/128" dev sc0
    sudo -n ip -n "$ns" -j addr > "$dir/addresses-$side.json"
    sudo -n ip -n "$ns" -j route show table all > "$dir/routes-$side.json"
    sudo -n ip -n "$ns" -j -6 route show table all > "$dir/routes-v6-$side.json"
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
run_session() {
  scope=$1; profile=$2; version=$3; trial=$4; mode=$5
  disk_gate
  dir="$out/$scope-$profile-v$version-t$trial-$mode"; mkdir "$dir"
  session_start_ns=$(date +%s%N)
  date -u +%FT%TZ > "$dir/start.utc"
  jq -n --arg scope "$scope" --arg profile "$profile" --argjson ip "$version" --argjson trial "$trial" --arg mode "$mode" \
    '{scope:$scope,profile:$profile,ip_version:$ip,trial:$trial,mode:$mode}' > "$dir/session.json"
  setup_topology "$dir" "$profile"
  calibrate "$dir" "$profile"
  for side in a b; do config "$side" "$mode" "$dir"; done
  touch "$dir/capture.pid" "$dir/a.pid" "$dir/b.pid" "$dir/underlay.pcap"
  sudo -n ip netns exec "$ns_a" sh -c 'echo "$$" > "$1"; exec tcpdump -Z root -i underlay -nn -s 96 -B 4096 -U -w "$2" "udp port 4433"' \
    capture "$dir/capture.pid" "$dir/underlay.pcap" > "$dir/capture.stdout" 2> "$dir/capture.stderr" &
  cap_job=$!
  for _ in $(seq 1 100); do grep -q 'listening on underlay' "$dir/capture.stderr" && break; sleep 0.1; done
  grep -q 'listening on underlay' "$dir/capture.stderr"
  cap_pid=$(cat "$dir/capture.pid")
  for side in b a; do
    ns=$ns_a; [ "$side" != b ] || ns=$ns_b
    sudo -n ip netns exec "$ns" sh -c 'echo "$$" > "$1"; exec "$2" run "$3"' \
      gateway "$dir/$side.pid" "$gateway" "$private/$side-config.json" > "$dir/gateway-$side.stdout" 2> "$dir/gateway-$side.stderr" &
    if [ "$side" = a ]; then a_job=$!; else b_job=$!; fi
  done
  wait_file "$dir/ready-a.json"; wait_file "$dir/ready-b.json"
  for side in a b; do
    jq -e --arg mode "$mode" '.ready==true and .mtu==1280 and .mode==$mode' "$dir/ready-$side.json" >/dev/null
  done
  daemon_a=$(cat "$dir/a.pid"); daemon_b=$(cat "$dir/b.pid")
  # Three framed objects on one TCP connection preserve the selector flow key.
  addr=10.204.0.2:41000; [ "$version" != 6 ] || addr="[fd42:5343::2]:41000"
  pattern="$private/$scope-pattern-$trial.bin"
  opaque="$private/$scope-opaque-$trial.bin"
  sudo -n ip netns exec "$ns_b" timeout --signal=TERM --kill-after=5 240 \
    "$probe" tcp-sequence-server "$addr" "$pattern" "$opaque" "$dir/tcp-server" \
    > "$dir/tcp-sequence-server.stdout" 2> "$dir/tcp-sequence-server.stderr" &
  server_job=$!
  for _ in $(seq 1 100); do
    sudo -n ip netns exec "$ns_b" ss -lnt | grep -q ':41000 ' && break
    sleep 0.05
  done
  sudo -n ip netns exec "$ns_b" ss -lnt | grep -q ':41000 '
  sudo -n ip netns exec "$ns_a" timeout --signal=TERM --kill-after=5 240 \
    "$probe" tcp-sequence-client "$addr" "$pattern" "$opaque" "$dir/tcp-client" \
    > "$dir/tcp-sequence-client.stdout" 2> "$dir/tcp-sequence-client.stderr"
  wait "$server_job"; server_job=
  for item in pattern-before opaque pattern-after; do
    kind=pattern; [ "$item" != opaque ] || kind=opaque
    for side in client server; do
      wait_file "$dir/tcp-$side-$item.json"
      jq -e --slurpfile expected "$out/fixture-$scope-$kind-$trial.json" \
        '.exact==true and .application_bytes==$expected[0].bytes and .sha256==$expected[0].sha256' \
        "$dir/tcp-$side-$item.json" > "$dir/tcp-$side-$item-exact.txt"
    done
  done
  # Existing bounded UDP reliability probe is a small functional smoke only.
  addr=10.204.0.2:41003; [ "$version" != 6 ] || addr="[fd42:5343::2]:41003"
  sudo -n ip netns exec "$ns_b" timeout --signal=TERM --kill-after=5 30 \
    "$probe" udp-server "$addr" "$private/udp-$trial.bin" "$dir/udp-smoke-server.json" \
    > "$dir/udp-smoke-server.stdout" 2> "$dir/udp-smoke-server.stderr" &
  server_job=$!
  for _ in $(seq 1 100); do
    sudo -n ip netns exec "$ns_b" ss -lnu | grep -q ':41003 ' && break
    sleep 0.05
  done
  sudo -n ip netns exec "$ns_b" ss -lnu | grep -q ':41003 '
  sudo -n ip netns exec "$ns_a" timeout --signal=TERM --kill-after=5 30 \
    "$probe" udp-client "$addr" "$private/udp-$trial.bin" "$dir/udp-smoke-client.json" \
    > "$dir/udp-smoke-client.stdout" 2> "$dir/udp-smoke-client.stderr"
  wait "$server_job"; server_job=
  for side in client server; do
    wait_file "$dir/udp-smoke-$side.json"
    jq -e --slurpfile expected "$out/fixture-udp-$trial.json" \
      '.exact==true and .application_bytes==$expected[0].bytes and .sha256==$expected[0].sha256' \
      "$dir/udp-smoke-$side.json" > "$dir/udp-smoke-$side-exact.txt"
  done
  sudo -n kill -TERM "$daemon_a" "$daemon_b"
  wait_job "$a_job"; a_job=
  wait_job "$b_job"; b_job=
  for side in a b; do
    wait_file "$dir/gateway-$side.json"
    jq '{expired_packets,capacity_drops,replay_drops,invalid_frames,
      resource_limited:(.capacity_drops>0),
      interpretation:"Expiry/replay are retained loss or ordering costs; capacity exhaustion is a distinct resource-limit failure. invalid_frames also includes capacity Error::Limit in the current service."}' \
      "$dir/gateway-$side.json" > "$dir/gateway-$side-delivery-diagnostics.json"
    validate_gateway_stats "$dir/gateway-$side.json" > "$dir/gateway-$side-accepted.txt"
    jq -e --arg mode "$mode" '.encoder as $e |
      $e.packets==($e.compressed_packets+$e.raw_packets) and
      (if $mode=="raw" then $e.compression_attempts==0 and $e.adaptive_skips==0
       elif $mode=="adaptive" then ($e.compression_attempts+$e.adaptive_skips)==$e.packets
       else $e.compression_attempts==$e.packets and $e.adaptive_skips==0 end)' \
      "$dir/gateway-$side.json" > "$dir/gateway-$side-selector-accepted.txt"
  done
  sleep 0.3
  sudo -n kill -USR1 "$cap_pid"
  sleep 0.3
  sudo -n kill -INT "$cap_pid"
  wait_job "$cap_job"; cap_job=
  "$probe" pcap-summary "$dir/underlay.pcap" "$dir/capture.json"
  awk '/packets captured/{c=$1}/packets received by filter/{r=$1}/packets dropped by kernel/{d=$1}
    END {if(c==""||r==""||d=="")exit 1; printf "{\"captured\":%s,\"received\":%s,\"dropped\":%s}\n",c,r,d}' \
    "$dir/capture.stderr" > "$dir/capture-counters.json"
  jq -e --slurpfile c "$dir/capture-counters.json" \
    '.other_packets==0 and .packets>0 and .packets==$c[0].captured and $c[0].captured==$c[0].received and $c[0].dropped==0' \
    "$dir/capture.json" > "$dir/capture-accepted.txt"
  qdisc_record "$dir" measurement-after
  if [ "$profile" = wan80-10 ]; then
    for link in left right; do
      validate_qdisc_stats "$dir/tc-$link-measurement-after.json" > "$dir/tc-$link-accepted.txt"
      jq '[.[]|select(.kind=="netem")][0] | {drops,overlimits,requeues,backlog,qlen,
        interpretation:"Finite-queue congestion drops are measured costs, not configured random loss. Capture completeness and exact application recovery are checked independently."}' \
        "$dir/tc-$link-measurement-after.json" > "$dir/tc-$link-delivery-diagnostics.json"
    done
  fi
  for ns in "$ns_a" "$ns_b" "$ns_r"; do sudo -n ip netns del "$ns"; done
  owned_namespaces=0; owned_link=0
  cap_pid=; daemon_a=; daemon_b=
  date -u +%FT%TZ > "$dir/end.utc"
  session_end_ns=$(date +%s%N)
  jq -n --argjson wall_us "$(((session_end_ns-session_start_ns)/1000))" \
    '{ok:true,namespace_cleanup:true,application_transfers:4,transport_flows:2,harness_wall_us:$wall_us,
      wall_scope:"topology setup, calibration, capture startup, authentication, four application transfers on two transport flows, gateway shutdown, capture drain/parse and topology cleanup; excludes global build/PKI/fixture creation"}' \
    > "$dir/complete.json"
  printf '%s exact transfers, calibration and capture accepted\n' "$(basename "$dir")"
}
for trial in 0 1 2; do
  profiles=$(jq -r --argjson trial "$trial" '.dose.profile_order_by_trial[$trial][]' "$registration")
  modes=$(jq -r --argjson trial "$trial" '.dose.mode_order_by_trial[$trial][]' "$registration")
  for profile in $profiles; do
    for mode in $modes; do run_session primary "$profile" 4 "$trial" "$mode"; done
  done
done
for profile in lan wan80-10; do
  for mode in raw zstd fast-zstd adaptive; do run_session smoke "$profile" 6 0 "$mode"; done
done
sudo -n ip -j route show table all > "$out/host-routes-after.json"
sudo -n ip -j -6 route show table all > "$out/host-routes-v6-after.json"
cmp "$out/host-routes-before.json" "$out/host-routes-after.json"
cmp "$out/host-routes-v6-before.json" "$out/host-routes-v6-after.json"
sha256sum -c "$out/frozen-inputs.sha256" > "$out/frozen-inputs-after.txt"
printf '{"sessions":32,"primary_sessions":24,"smoke_sessions":8,"transport_flows":64,"application_transfers":128,"exact":true,"capture_complete":true,"calibration_complete":true,"host_routes_unchanged":true}\n' > "$out/complete.json"
