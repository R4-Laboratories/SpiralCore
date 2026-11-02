# Input: an array of {path, data} records for campaign/{primary,smoke}-*/*.json.
# Example collection: jq -n '[inputs|{path:input_filename,data:.}]' "$campaign"/primary-*/*.json "$campaign"/smoke-*/*.json
def check($ok; $why): if $ok then . else error($why) end;
def integer: type == "number" and . >= 0 and floor == .;
def median: sort | .[length/2|floor];
def required($f; $name): if $f|has($name) then $f[$name] else error("missing " + $name) end;
def app_pair($name; $client; $server; $bytes; $index):
  null | check($client.exact == true and $server.exact == true and
    $client.application_bytes == $bytes and $server.application_bytes == $bytes and
    $client.sha256 == $server.sha256 and ($client.sha256|test("^[0-9a-f]{64}$")); "application exactness/length/hash " + $name) |
  check(all([$client.wall_us,$server.wall_us,$client.process_cpu_us,$server.process_cpu_us][]; integer) and
    $client.wall_us > 0 and $server.wall_us > 0; "application timing " + $name) |
  check(if $index == null then $client.protocol == "udp" and $server.protocol == "udp"
    else $client.protocol == "tcp" and $server.protocol == "tcp" and
    $client.role == "tcp-sequence-client" and $server.role == "tcp-sequence-server" and
    $client.detail.same_connection == true and $server.detail.same_connection == true and
    $client.detail.sequence_index == $index and $server.detail.sequence_index == $index and
    all([$client.detail.local_address,$client.detail.peer_address,$server.detail.local_address,$server.detail.peer_address][];
      type == "string" and length > 0) and
    $client.detail.connect_included == ($index == 0) and $server.detail.connect_included == false and
    $client.detail.reports_emitted_after_sequence == true and $server.detail.reports_emitted_after_sequence == true
    end; "application sequence contract " + $name) |
  {transfer:$name, application_bytes:$bytes, sha256:$client.sha256,
   client_wall_us:$client.wall_us, server_wall_us:$server.wall_us,
   client_interval_cpu_us:$client.process_cpu_us, server_interval_cpu_us:$server.process_cpu_us,
   client_detail:$client.detail, server_detail:$server.detail};
def gateway_valid($g; $mode):
  $g.status == "ok" and $g.mode == $mode and $g.denied_packets == 0 and
  $g.invalid_frames == 0 and $g.capacity_drops == 0 and
  all([$g.process_cpu_us,$g.encode_cpu_us,$g.decode_cpu_us,$g.expired_packets,$g.replay_drops,
    $g.setup_cpu_us,$g.wall_us,$g.framed_sent_bytes,$g.udp_sent_bytes,
    $g.encoder.packets,$g.encoder.compression_attempts,$g.encoder.compressed_packets,$g.encoder.raw_packets,
    $g.encoder.adaptive_skips,$g.encoder.reprobes,$g.encoder.guard_probes,$g.encoder.flow_evictions][]; integer) and
  $g.process_cpu_us >= ($g.encode_cpu_us + $g.decode_cpu_us) and
  $g.encoder.packets == ($g.encoder.raw_packets + $g.encoder.compressed_packets) and
  (if $mode == "raw" then $g.encoder.compression_attempts == 0 and $g.encoder.adaptive_skips == 0
   elif $mode == "adaptive" then $g.encoder.packets == ($g.encoder.compression_attempts + $g.encoder.adaptive_skips)
   else $g.encoder.compression_attempts == $g.encoder.packets and $g.encoder.adaptive_skips == 0 end);
def session($id; $f):
  required($f; "session.json") as $m |
  required($f; "complete.json") as $done |
  required($f; "capture.json") as $cap |
  required($f; "capture-counters.json") as $cc |
  required($f; "gateway-a.json") as $a |
  required($f; "gateway-b.json") as $b |
  null |
  check($id == "\($m.scope)-\($m.profile)-v\($m.ip_version)-t\($m.trial)-\($m.mode)";
    "session metadata/path mismatch " + $id) |
  check(($m.profile == "lan" or $m.profile == "wan80-10") and
    (["raw","zstd","fast-zstd","adaptive"]|index($m.mode)) != null and
    (if $m.scope == "primary" then $m.ip_version == 4 and ([0,1,2]|index($m.trial)) != null
     else $m.scope == "smoke" and $m.ip_version == 6 and $m.trial == 0 end); "session shape " + $id) |
  check($done.ok == true and $done.namespace_cleanup == true and $done.application_transfers == 4 and $done.transport_flows == 2;
    "incomplete session " + $id) |
  check($cap.other_packets == 0 and $cap.packets > 0 and $cap.packets == $cc.captured and
    $cc.captured == $cc.received and $cc.dropped == 0 and
    all([$cap.packets,$cap.ip_bytes,$cap.udp_payload_bytes,$cap.ethernet_bytes][]; integer);
    "capture completeness " + $id) |
  check(gateway_valid($a;$m.mode) and gateway_valid($b;$m.mode); "gateway failure/counter inconsistency " + $id) |
  (if $m.scope == "primary" then 16777216 else 1048576 end) as $size |
  (["pattern-before","opaque","pattern-after"] | to_entries | map(. as $item |
    app_pair("tcp-"+$item.value; required($f; "tcp-client-"+$item.value+".json");
      required($f; "tcp-server-"+$item.value+".json"); $size; $item.key))) as $tcp |
  app_pair("udp-smoke"; required($f; "udp-smoke-client.json"); required($f; "udp-smoke-server.json"); 4096; null) as $udp |
  check($tcp[0].sha256 == $tcp[2].sha256 and
    ($tcp|map(.client_detail.local_address)|unique|length) == 1 and
    ($tcp|map(.client_detail.peer_address)|unique|length) == 1 and
    ($tcp|map(.server_detail.local_address)|unique|length) == 1 and
    ($tcp|map(.server_detail.peer_address)|unique|length) == 1 and
    $tcp[0].client_detail.local_address == $tcp[0].server_detail.peer_address and
    $tcp[0].client_detail.peer_address == $tcp[0].server_detail.local_address;
    "persistent TCP endpoint or pattern identity " + $id) |
  required($f; "calibration.json") as $cal |
  [required($f; "ping-a.json"), required($f; "ping-b.json")] as $ping |
  check($cal.accepted == true and $cal.profile == $m.profile and
    all($ping[]; .count == 20 and (.samples|length) == 20 and .retained == .samples[4:] and
      ((.mean_ms - ((.retained|add)/16))|fabs) < 0.000001 and
      (if $m.profile == "wan80-10" then .mean_ms >= 75 and .mean_ms <= 90 else .mean_ms < 20 end));
    "RTT calibration " + $id) |
  (if $m.profile == "wan80-10" then
    [required($f; "iperf-forward-client.json"), required($f; "iperf-reverse-client.json")]
   else [] end) as $rate |
  check(all($rate[]; .error == null and .end.sum_received.bits_per_second >= 8500000 and
    .end.sum_received.bits_per_second <= 10200000); "rate calibration " + $id) |
  [required($f; "tc-left-measurement-after.json"), required($f; "tc-right-measurement-after.json")] as $tc |
  check(if $m.profile == "wan80-10" then all($tc[];
      map(select(.kind == "netem")) | length == 1 and .[0].qlen == 0 and (.[0].drops|integer))
    else true end; "undrained or malformed shaped qdisc " + $id) |
  ($tcp + [$udp]) as $apps |
  $m + {session:$id, application_input_bytes:($apps|map(.application_bytes)|add),
    capture_ip_bytes:$cap.ip_bytes,capture_udp_payload_bytes:$cap.udp_payload_bytes,
    capture_ethernet_bytes:$cap.ethernet_bytes,capture_packets:$cap.packets,
    gateway_process_cpu_us:($a.process_cpu_us+$b.process_cpu_us),
    gateway_a_process_cpu_us:$a.process_cpu_us,gateway_b_process_cpu_us:$b.process_cpu_us,
    nested_encode_cpu_us:($a.encode_cpu_us+$b.encode_cpu_us),nested_decode_cpu_us:($a.decode_cpu_us+$b.decode_cpu_us),
    application_interval_cpu_us:($apps|map(.client_interval_cpu_us+.server_interval_cpu_us)|add),
    gateway_setup_cpu_us:($a.setup_cpu_us+$b.setup_cpu_us),
    gateway_a_wall_us:$a.wall_us,gateway_b_wall_us:$b.wall_us,harness_wall_us:$done.harness_wall_us,
    framed_sent_bytes:($a.framed_sent_bytes+$b.framed_sent_bytes),
    quinn_sent_udp_payload_bytes:($a.udp_sent_bytes+$b.udp_sent_bytes),
    congestion_queue_drops:($tc|flatten|map(.drops//0)|add//0),
    expired_packets:($a.expired_packets+$b.expired_packets),replay_drops:($a.replay_drops+$b.replay_drops),
    rtt_mean_ms_each_direction:($ping|map(.mean_ms)),rate_bits_per_second_each_direction:($rate|map(.end.sum_received.bits_per_second)),
    encoder_a:$a.encoder,encoder_b:$b.encoder,transfers:$apps};

map(. as $record | (.path|capture("(?<session>(?:primary|smoke)-(?:lan|wan80-10)-v[46]-t[0-2]-(?:raw|zstd|fast-zstd|adaptive))/(?<file>[^/]+\\.json)$")) as $path |
    {session:$path.session,file:$path.file,data:$record.data}) |
group_by(.session) | map(.[0].session as $id |
  (reduce .[] as $record ({}; check(has($record.file)|not; "duplicate receipt " + $id + "/" + $record.file) |
    .[$record.file]=$record.data)) as $files | session($id;$files)) as $sessions |
check(($sessions|length) == 32 and ($sessions|map(select(.scope == "primary"))|length) == 24 and
  ($sessions|map(select(.scope == "smoke"))|length) == 8 and
  ($sessions|map(.application_input_bytes)|add) == 1233256448; "campaign count or application-byte mismatch") |
[$sessions[] | . as $s | .transfers[] | . + ($s|{session,scope,profile,ip_version,trial,mode})] as $transfers |
check(all($transfers|group_by([.scope,.trial,.transfer])[];
  (map(.sha256)|unique|length) == 1 and (map(.application_bytes)|unique|length) == 1);
  "fixture identity differs between arms/profiles") |
[$sessions[]|select(.scope == "primary")] as $primary |
[$transfers[]|select(.scope == "primary")] as $primary_transfers |
[$primary|group_by([.profile,.trial])[] | . as $group |
  $group[] as $candidate | ["raw","zstd","fast-zstd"][] as $base |
  select($candidate.mode != $base) | ($group|map(select(.mode == $base))|.[0]) as $baseline |
  {profile:$candidate.profile,trial:$candidate.trial,mode:$candidate.mode,baseline:$base,
   capture_ip_byte_ratio:($candidate.capture_ip_bytes/$baseline.capture_ip_bytes),
   capture_udp_payload_byte_ratio:($candidate.capture_udp_payload_bytes/$baseline.capture_udp_payload_bytes),
   gateway_process_cpu_ratio:($candidate.gateway_process_cpu_us/$baseline.gateway_process_cpu_us),
   application_interval_cpu_ratio:($candidate.application_interval_cpu_us/$baseline.application_interval_cpu_us)}] as $session_ratios |
[$primary_transfers|group_by([.profile,.trial,.transfer])[] | . as $group |
  $group[] as $candidate | ["raw","zstd","fast-zstd"][] as $base |
  select($candidate.mode != $base) | ($group|map(select(.mode == $base))|.[0]) as $baseline |
  {profile:$candidate.profile,trial:$candidate.trial,transfer:$candidate.transfer,mode:$candidate.mode,baseline:$base,
   client_wall_ratio:($candidate.client_wall_us/$baseline.client_wall_us),
   server_wall_ratio:($candidate.server_wall_us/$baseline.server_wall_us)}] as $time_ratios |
{
 schema:"spiralcore.packet-selection.independent-network-analysis.v1",
 scope:"Shaped local Linux network on one runner; not Internet WAN or separate physical hosts. Medians are descriptive n3, no p95, confidence or energy claim.",
 accounting:"Capture is one underlay interface in both directions, includes authentication and drain. Mixed-session bytes/CPU cannot be attributed to individual objects. Gateway process CPU includes codec CPU; nested codec times are never added to it. Application CPU covers recorded intervals, not complete process lifetime. Per-object client wall includes its exact receipt; first TCP includes connect.",
 exact_endpoint_receipts:(($transfers|length)*2),
 logical_application_transfers:($transfers|length),sessions:$sessions,
 primary_session_medians:($primary|group_by([.profile,.mode])|map({profile:.[0].profile,mode:.[0].mode,trials:length,
  capture_ip_bytes:([.[].capture_ip_bytes]|median),capture_udp_payload_bytes:([.[].capture_udp_payload_bytes]|median),
  capture_ethernet_bytes:([.[].capture_ethernet_bytes]|median),gateway_process_cpu_us:([.[].gateway_process_cpu_us]|median),
  nested_encode_cpu_us:([.[].nested_encode_cpu_us]|median),nested_decode_cpu_us:([.[].nested_decode_cpu_us]|median),
  application_interval_cpu_us:([.[].application_interval_cpu_us]|median),
  congestion_queue_drops_by_trial:(sort_by(.trial)|map(.congestion_queue_drops)),
  expired_packets_by_trial:(sort_by(.trial)|map(.expired_packets)),replay_drops_by_trial:(sort_by(.trial)|map(.replay_drops))})),
 primary_transfer_medians:($primary_transfers|group_by([.profile,.mode,.transfer])|map({profile:.[0].profile,mode:.[0].mode,transfer:.[0].transfer,trials:length,
  application_bytes:.[0].application_bytes,client_wall_us:([.[].client_wall_us]|median),server_wall_us:([.[].server_wall_us]|median)})),
 paired_session_ratios:$session_ratios,
 median_paired_session_ratios:($session_ratios|group_by([.profile,.mode,.baseline])|map({profile:.[0].profile,mode:.[0].mode,baseline:.[0].baseline,trials:length,
  capture_ip_byte_ratio:([.[].capture_ip_byte_ratio]|median),capture_udp_payload_byte_ratio:([.[].capture_udp_payload_byte_ratio]|median),
  gateway_process_cpu_ratio:([.[].gateway_process_cpu_ratio]|median),application_interval_cpu_ratio:([.[].application_interval_cpu_ratio]|median)})),
 paired_transfer_wall_ratios:$time_ratios,
 median_paired_transfer_wall_ratios:($time_ratios|group_by([.profile,.mode,.baseline,.transfer])|map({profile:.[0].profile,mode:.[0].mode,baseline:.[0].baseline,transfer:.[0].transfer,trials:length,
  client_wall_ratio:([.[].client_wall_ratio]|median),server_wall_ratio:([.[].server_wall_ratio]|median)})),
 ipv6_interpretation:"Eight n1 sessions retained individually for functional exactness; excluded from primary performance medians and paired performance ratios."
}
