#!/bin/bash
set -euo pipefail
umask 077
out=/var/tmp/spiralcore-tun-capture-session-debug-20261103
probe=$out/spiralcore-gateway-probe
cd "$out"
printf '%s  %s\n' 5da4bc45b4a0e2cefac96bfb16c1c65828c648bfc3d74962e64f11642555b340 "$probe" | sha256sum -c - > probe-verified.txt
tcpdump --version > tcpdump-version.txt 2>&1
ip -j route show table all | sha256sum > routes-before.sha256
ip -j -6 route show table all | sha256sum > routes6-before.sha256
"$probe" fixture "$out/fixture.bin" 32768 pattern 1729 > fixture.json
ns_a=; ns_b=; cap_pid=; cap_job=; server_job=
cleanup() {
  if [ -n "$cap_pid" ]; then sudo -n kill -INT "$cap_pid" 2>/dev/null || :; fi
  if [ -n "$server_job" ]; then kill -TERM "$server_job" 2>/dev/null || :; fi
  if [ -n "$cap_job" ]; then wait "$cap_job" 2>/dev/null || :; fi
  if [ -n "$server_job" ]; then wait "$server_job" 2>/dev/null || :; fi
  if [ -n "$ns_a" ]; then sudo -n ip netns del "$ns_a"; fi
  if [ -n "$ns_b" ]; then sudo -n ip netns del "$ns_b"; fi
}
trap cleanup EXIT
for mode in default immediate; do
  dir=$out/$mode
  mkdir -m 700 "$dir"
  ns_a=scdc${$}a; ns_b=scdc${$}b
  sudo -n ip netns add "$ns_a"
  sudo -n ip netns add "$ns_b"
  sudo -n ip -n "$ns_a" link add underlay type veth peer name peer
  sudo -n ip -n "$ns_a" link set peer netns "$ns_b"
  sudo -n ip -n "$ns_b" link set peer name underlay
  for side in a b; do
    ns=$ns_a; index=1
    if [ "$side" = b ]; then ns=$ns_b; index=2; fi
    sudo -n ip -n "$ns" addr add "10.231.0.$index/30" dev underlay
    sudo -n ip -n "$ns" link set underlay up
    sudo -n ip -n "$ns" link set lo up
  done
  touch "$dir/capture.pid" "$dir/capture.pcap"
  extra=; if [ "$mode" = immediate ]; then extra=--immediate-mode; fi
  sudo -n ip netns exec "$ns_a" sh -c 'echo "$$" > "$1"; shift; out=$1; shift; exec tcpdump -Z root -i underlay -nn -s 96 -B 4096 -U "$@" -w "$out" "udp port 4433"' \
    capture "$dir/capture.pid" "$dir/capture.pcap" ${extra:+"$extra"} > "$dir/capture.stdout" 2> "$dir/capture.stderr" &
  cap_job=$!
  for unused in $(seq 1 100); do grep -q 'listening on underlay' "$dir/capture.stderr" && break; sleep 0.05; done
  grep -q 'listening on underlay' "$dir/capture.stderr"
  cap_pid=$(cat "$dir/capture.pid")
  date -u +%FT%TZ > "$dir/start.utc"
  sudo -n ip netns exec "$ns_b" timeout --signal=TERM --kill-after=2 20 \
    "$probe" udp-server 10.231.0.2:4433 "$out/fixture.bin" "$dir/server.json" > "$dir/server.stdout" 2> "$dir/server.stderr" &
  server_job=$!
  for unused in $(seq 1 100); do sudo -n ip netns exec "$ns_b" ss -lnu | grep -q ':4433 ' && break; sleep 0.05; done
  sudo -n ip netns exec "$ns_b" ss -lnu | grep -q ':4433 '
  sudo -n ip netns exec "$ns_a" timeout --signal=TERM --kill-after=2 20 \
    "$probe" udp-client 10.231.0.2:4433 "$out/fixture.bin" "$dir/client.json" > "$dir/client.stdout" 2> "$dir/client.stderr"
  wait "$server_job"; server_job=
  for role in client server; do
    jq -e --slurpfile expected "$out/fixture.json" '.exact==true and .application_bytes==$expected[0].bytes and .sha256==$expected[0].sha256' "$dir/$role.json" > "$dir/$role-exact.txt"
  done
  sleep 0.3
  sudo -n kill -USR1 "$cap_pid"
  sleep 0.3
  sudo -n kill -INT "$cap_pid"
  wait "$cap_job"; cap_job=; cap_pid=
  "$probe" pcap-summary "$dir/capture.pcap" "$dir/capture.json"
  awk '/packets captured/{c=$1}/packets received by filter/{r=$1}/packets dropped by kernel/{d=$1} END {if(c==""||r==""||d=="")exit 1; printf "{\"captured\":%s,\"received\":%s,\"dropped\":%s}\n",c,r,d}' "$dir/capture.stderr" > "$dir/counters.json"
  if jq -e --slurpfile c "$dir/counters.json" '.other_packets==0 and .packets>0 and .packets==$c[0].captured and $c[0].captured==$c[0].received and $c[0].dropped==0' "$dir/capture.json" > "$dir/capture-accepted.txt"; then gate=0; else gate=$?; fi
  printf '%s\n' "$gate" > "$dir/gate.exit"
  date -u +%FT%TZ > "$dir/end.utc"
  sudo -n ip netns del "$ns_a"; ns_a=
  sudo -n ip netns del "$ns_b"; ns_b=
  printf '%s ' "$mode"; cat "$dir/counters.json"
done
ip -j route show table all | sha256sum > routes-after.sha256
ip -j -6 route show table all | sha256sum > routes6-after.sha256
cmp routes-before.sha256 routes-after.sha256
cmp routes6-before.sha256 routes6-after.sha256
printf '{"cells":2,"owned_namespace_cleanup":true,"host_routes_unchanged":true}\n' > complete.json
