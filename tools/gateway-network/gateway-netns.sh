#!/usr/bin/env bash
# Bounded Linux-only ordinary-socket/TUN/QUIC witness. No host routes or NAT.
set -euo pipefail
validate_registration() {
  jq -e '
    .schema=="spiralcore.tun.registration.v1" and .launch_ready==true and
    .dose.trials==[0,1,2] and .dose.application_ip_versions==[4,6] and
    .dose.packet_modes==["raw","zstd"] and .dose.persistent_peer_sessions==12 and
    .dose.application_flows==48 and .topology.tun_mtu==1280 and .topology.quic_port==4433 and
    .dose.flows_per_session==[
      {"protocol":"TCP","fixture":"pattern","bytes":16777216},
      {"protocol":"TCP","fixture":"opaque","bytes":16777216},
      {"protocol":"UDP","fixture":"pattern","bytes":1048576},
      {"protocol":"UDP","fixture":"opaque","bytes":1048576}] and
    .dose.mode_order_by_trial==[["raw","zstd"],["zstd","raw"],["raw","zstd"]] and
    .custody.automatic_retries==false and .custody.free_disk_floor_bytes==4294967296
  ' "$1" >/dev/null
}
if [ "$#" = 2 ] && [ "$1" = validate ]; then validate_registration "$2"; exit; fi
[ "$#" = 5 ] && [ "$1" = run ] || { echo "usage: $0 validate REGISTRATION | run NEW_OUT GATEWAY PROBE REGISTRATION" >&2; exit 2; }
out=$2; gateway=$3; probe=$4; registration=$5
validate_registration "$registration"
[ "$(uname -s)" = Linux ] || { echo "Linux namespaces required" >&2; exit 2; }
case "$out:$gateway:$probe" in /*:/*:/*) ;; *) echo "absolute paths required" >&2; exit 2;; esac
[ ! -e "$out" ] && [ -x "$gateway" ] && [ -x "$probe" ]
for tool in ip ss ethtool tcpdump openssl jq timeout sha256sum; do command -v "$tool" >/dev/null; done
sudo -n true
test -c /dev/net/tun
umask 077
mkdir "$out"
private=$(mktemp -d /tmp/spiralcore-gateway.XXXXXXXX)
ns_a="scg-$$-a"; ns_b="scg-$$-b"
owned_namespaces=0
owned_link=0
cap_pid=; daemon_a=; daemon_b=; cap_job=; a_job=; b_job=; server_job=
cleanup() {
  code=$?
  trap - EXIT INT TERM
  if [ "$owned_namespaces" = 1 ]; then
    # Only processes in our two freshly created namespaces may be signaled.
    for ns in "$ns_a" "$ns_b"; do
      pids=$(sudo -n ip netns pids "$ns" 2>/dev/null || true)
      if [ -n "$pids" ]; then sudo -n kill -TERM $pids 2>/dev/null || true; fi
    done
    sleep 0.2
    for ns in "$ns_a" "$ns_b"; do
      pids=$(sudo -n ip netns pids "$ns" 2>/dev/null || true)
      if [ -n "$pids" ]; then sudo -n kill -KILL $pids 2>/dev/null || true; fi
      sudo -n ip netns del "$ns" 2>/dev/null || true
    done
  fi
  if [ "$owned_link" = 1 ]; then
    sudo -n ip link del scga$$ 2>/dev/null || true
    sudo -n ip link del scgb$$ 2>/dev/null || true
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
for trial in 0 1 2; do
  for protocol in tcp udp; do
    size=16777216; [ "$protocol" != udp ] || size=1048576
    for kind in pattern opaque; do
      "$probe" fixture "$private/$protocol-$kind-$trial.bin" "$size" "$kind" "$((1729+trial))" \
        > "$out/fixture-$protocol-$kind-$trial.json"
    done
  done
done
wait_file() {
  file=$1
  for unused in $(seq 1 200); do
    if [ -s "$file" ]; then sudo -n chmod 0644 "$file"; return; fi
    sleep 0.1
  done
  echo "missing ready/receipt: $file" >&2; return 1
}
wait_job() {
  job=$1
  for unused in $(seq 1 300); do
    if ! kill -0 "$job" 2>/dev/null; then wait "$job"; return; fi
    sleep 0.1
  done
  echo "owned process shutdown timeout" >&2; return 1
}
config() {
  side=$1; mode=$2; dir=$3
  if [ "$side" = a ]; then role=connect; local4=10.204.0.1; remote4=10.204.0.2; local6=fd42:5343::1; remote6=fd42:5343::2; bind=10.203.0.1:4433; peer=10.203.0.2:4433; other=b
  else role=listen; local4=10.204.0.2; remote4=10.204.0.1; local6=fd42:5343::2; remote6=fd42:5343::1; bind=10.203.0.2:4433; peer=10.203.0.1:4433; other=a; fi
  jq -n --arg role "$role" --arg bind "$bind" --arg peer "$peer" --arg other "$other" --arg side "$side" \
    --arg private "$private" --arg dir "$dir" --arg mode "$mode" --arg l4 "$local4" --arg r4 "$remote4" --arg l6 "$local6" --arg r6 "$remote6" \
    '{role:$role,bind:$bind,peer:$peer,server_name:("peer-"+$other+".test"),
      certificate:($private+"/"+$side+".der"),private_key:($private+"/"+$side+"-key.der"),
      trust_anchor:($private+"/ca.der"),peer_certificate:($private+"/"+$other+".der"),tun:"sc0",mode:$mode,
      local_addresses:[$l4,$l6],peer_addresses:[$r4,$r6],stats_path:($dir+"/gateway-"+$side+".json"),
      ready_path:($dir+"/ready-"+$side+".json"),duration_secs:300}' > "$private/$side-config.json"
}
for trial in 0 1 2; do
  for version in 4 6; do
    modes="raw zstd"; [ "$trial" != 1 ] || modes="zstd raw"
    for mode in $modes; do
      disk_gate
      dir="$out/v$version-t$trial-$mode"; mkdir "$dir"
      date -u +%FT%TZ > "$dir/start.utc"
      # Reject existing names instead of assuming ownership of any namespace.
      ! sudo -n ip netns list | awk '{print $1}' | grep -Fx "$ns_a"
      ! sudo -n ip netns list | awk '{print $1}' | grep -Fx "$ns_b"
      sudo -n ip netns add "$ns_a"
      owned_namespaces=1
      sudo -n ip netns add "$ns_b"
      sudo -n ip link add scga$$ type veth peer name scgb$$
      owned_link=1
      sudo -n ip link set scga$$ netns "$ns_a"
      sudo -n ip link set scgb$$ netns "$ns_b"
      for side in a b; do
        if [ "$side" = a ]; then ns=$ns_a; link=scga$$; index=1; other=2; else ns=$ns_b; link=scgb$$; index=2; other=1; fi
        sudo -n ip -n "$ns" link set "$link" name underlay
        sudo -n ip -n "$ns" addr add "10.203.0.$index/30" dev underlay
        sudo -n ip -n "$ns" link set lo up
        sudo -n ip -n "$ns" link set underlay up
        sudo -n ip netns exec "$ns" ethtool -K underlay tso off gso off gro off \
          > "$dir/offload-$side-change.txt" 2>&1
        # Kernel feature names vary; record optional feature requests and the
        # complete final state. Base gso/gro disable is mandatory above.
        sudo -n ip netns exec "$ns" ethtool -K underlay tx-udp-segmentation off rx-udp-gro-forwarding off \
          >> "$dir/offload-$side-change.txt" 2>&1 || true
        sudo -n ip netns exec "$ns" ethtool -k underlay > "$dir/offload-$side-state.txt"
        ! grep -Eq '^[[:space:]]*(generic-segmentation-offload|generic-receive-offload|tx-udp-segmentation): on' "$dir/offload-$side-state.txt"
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
        config "$side" "$mode" "$dir"
      done
      touch "$dir/capture.pid" "$dir/a.pid" "$dir/b.pid" "$dir/underlay.pcap"
      sudo -n ip netns exec "$ns_a" sh -c 'echo "$$" > "$1"; exec tcpdump -Z root -i underlay -nn -s 96 -B 4096 -U -w "$2" "udp port 4433"' \
        capture "$dir/capture.pid" "$dir/underlay.pcap" > "$dir/capture.stdout" 2> "$dir/capture.stderr" &
      cap_job=$!
      for unused in $(seq 1 100); do grep -q 'listening on underlay' "$dir/capture.stderr" && break; sleep 0.1; done
      grep -q 'listening on underlay' "$dir/capture.stderr"
      cap_pid=$(cat "$dir/capture.pid")
      for side in b a; do
        ns=$ns_a; [ "$side" != b ] || ns=$ns_b
        sudo -n ip netns exec "$ns" sh -c 'echo "$$" > "$1"; exec "$2" run "$3"' \
          gateway "$dir/$side.pid" "$gateway" "$private/$side-config.json" > "$dir/gateway-$side.stdout" 2> "$dir/gateway-$side.stderr" &
        if [ "$side" = a ]; then a_job=$!; else b_job=$!; fi
      done
      wait_file "$dir/ready-a.json"; wait_file "$dir/ready-b.json"
      jq -e --arg mode "$mode" '.ready==true and .mtu==1280 and .mode==$mode' "$dir/ready-a.json" >/dev/null
      jq -e --arg mode "$mode" '.ready==true and .mtu==1280 and .mode==$mode' "$dir/ready-b.json" >/dev/null
      daemon_a=$(cat "$dir/a.pid"); daemon_b=$(cat "$dir/b.pid")
      for protocol in tcp udp; do
        for kind in pattern opaque; do
          name="$protocol-$kind"
          port=41000; [ "$protocol" != udp ] || port=41002
          [ "$kind" != opaque ] || port=$((port+1))
          addr="10.204.0.2:$port"; [ "$version" != 6 ] || addr="[fd42:5343::2]:$port"
          fixture="$private/$protocol-$kind-$trial.bin"
          sudo -n ip netns exec "$ns_b" timeout --signal=TERM --kill-after=5 90 \
            "$probe" "$protocol-server" "$addr" "$fixture" "$dir/$name-server.json" \
            > "$dir/$name-server.stdout" 2> "$dir/$name-server.stderr" &
          server_job=$!
          # Observe a bound socket without making an extra TCP connection.
          flag=-lnt; [ "$protocol" != udp ] || flag=-lnu
          for unused in $(seq 1 100); do
            sudo -n ip netns exec "$ns_b" ss "$flag" | grep -q ":$port " && break
            sleep 0.05
          done
          sudo -n ip netns exec "$ns_b" ss "$flag" | grep -q ":$port "
          sudo -n ip netns exec "$ns_a" timeout --signal=TERM --kill-after=5 90 \
            "$probe" "$protocol-client" "$addr" "$fixture" "$dir/$name-client.json" \
            > "$dir/$name-client.stdout" 2> "$dir/$name-client.stderr"
          wait "$server_job"; server_job=
          wait_file "$dir/$name-client.json"; wait_file "$dir/$name-server.json"
          jq -e --slurpfile expected "$out/fixture-$protocol-$kind-$trial.json" \
            '.exact==true and .application_bytes==$expected[0].bytes and .sha256==$expected[0].sha256' \
            "$dir/$name-client.json" > "$dir/$name-client-exact.txt"
          jq -e --slurpfile expected "$out/fixture-$protocol-$kind-$trial.json" \
            '.exact==true and .application_bytes==$expected[0].bytes and .sha256==$expected[0].sha256' \
            "$dir/$name-server.json" > "$dir/$name-server-exact.txt"
        done
      done
      sudo -n kill -TERM "$daemon_a" "$daemon_b"
      wait_job "$a_job"; a_job=
      wait_job "$b_job"; b_job=
      wait_file "$dir/gateway-a.json"; wait_file "$dir/gateway-b.json"
      for side in a b; do
        jq -e '.status=="ok" and .denied_packets==0 and .invalid_frames==0 and .capacity_drops==0 and .expired_packets==0' \
          "$dir/gateway-$side.json" > "$dir/gateway-$side-accepted.txt"
      done
      # Bounded post-close quiet observation; not a claim about traffic outside
      # this interval. Interrupt tcpdump cleanly to emit final capture counters.
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
      date -u +%FT%TZ > "$dir/end.utc"
      sudo -n ip netns del "$ns_a"; sudo -n ip netns del "$ns_b"; owned_namespaces=0
      owned_link=0
      cap_pid=; daemon_a=; daemon_b=
      printf '{"ok":true,"namespace_cleanup":true,"flows":4}\n' > "$dir/complete.json"
      printf '%s exact flows and capture accepted\n' "$(basename "$dir")"
    done
  done
done
sudo -n ip -j route show table all > "$out/host-routes-after.json"
sudo -n ip -j -6 route show table all > "$out/host-routes-v6-after.json"
cmp "$out/host-routes-before.json" "$out/host-routes-after.json"
cmp "$out/host-routes-v6-before.json" "$out/host-routes-v6-after.json"
sha256sum -c "$out/frozen-inputs.sha256" > "$out/frozen-inputs-after.txt"
printf '{"sessions":12,"flows":48,"exact":true,"capture_complete":true,"host_routes_unchanged":true}\n' > "$out/complete.json"
