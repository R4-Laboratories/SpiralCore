def median: sort | .[length/2|floor];
def key: [.ipv6,.packet_bytes,.workload,.max_datagram,.mode,.trial] | tojson;
($before | map(select(.stage == "packet_codec"))) as $b |
($after | map(select(.stage == "packet_codec"))) as $a |
(reduce $b[] as $r ({}; .[($r|key)] = $r)) as $index |
[$a[] | select((.mode == "raw" or .mode == "zstd") and .workload != "guard_miss_shifted") |
 . as $r | $index[($r|key)] as $old |
 {mode, same_input:(.input_sha256 == $old.input_sha256),
  same_body:(.body_bytes == $old.body_bytes), same_framed:(.framed_bytes == $old.framed_bytes),
  encode_cpu_ratio:(.encode_cpu_ns / $old.encode_cpu_ns), decode_cpu_ratio:(.decode_cpu_ns / $old.decode_cpu_ns)}] as $paired |
($a | group_by([.ipv6,.packet_bytes,.workload,.max_datagram,.mode]) | map(. as $rows | {
 ipv6:.[0].ipv6, packet_bytes:.[0].packet_bytes, workload:.[0].workload,
 max_datagram:.[0].max_datagram, mode:.[0].mode, trials:length, packets_per_trial:.[0].packets,
 input_bytes:.[0].input_bytes, body_bytes:.[0].body_bytes, framed_bytes:.[0].framed_bytes,
 byte_counts_stable:all(.body_bytes == $rows[0].body_bytes and .framed_bytes == $rows[0].framed_bytes),
 median_encode_cpu_us_per_packet:([.[]|.encode_cpu_ns/.packets/1000]|median),
 median_decode_cpu_us_per_packet:([.[]|.decode_cpu_ns/.packets/1000]|median),
 median_encode_wall_us_per_packet:([.[]|.encode_wall_ns/.packets/1000]|median),
 median_decode_wall_us_per_packet:([.[]|.decode_wall_ns/.packets/1000]|median),
 median_first_encode_cpu_us:(([.[].first_encode_cpu_ns]|median)/1000),
 median_setup_cpu_us:(([.[].setup_cpu_ns]|median)/1000),
 encoder_counters:.[0].encoder_counters
})) as $groups |
{
 version:1, scope:"Local optimized codec diagnostic; process CPU, no network/authentication/durability. Three trials on shared host; timing drift must be considered.",
 baseline_measured_rows:($before|map(select(.stage))|length),
 after_measured_rows:($after|map(select(.stage))|length),
 after_all_exact:($after|map(select(.stage))|all(.exact_packets == .packets)),
 paired_legacy_rows:($paired|length), paired_inputs_identical:($paired|all(.same_input)),
 paired_body_and_framed_identical:($paired|all(.same_body and .same_framed)),
 unchanged_control_median_cpu_ratios:($paired|group_by(.mode)|map({mode:.[0].mode,
  encode_after_over_before:([.[].encode_cpu_ratio]|median),decode_after_over_before:([.[].decode_cpu_ratio]|median)})),
 packet_groups:$groups,
 adaptive_comparisons:($groups|group_by([.ipv6,.packet_bytes,.workload,.max_datagram])|map(
  .[0] as $first | (reduce .[] as $r ({}; .[$r.mode]=$r)) as $m |
  {ipv6:$first.ipv6,packet_bytes:$first.packet_bytes,workload:$first.workload,max_datagram:$first.max_datagram,
   historical_before_available:($first.workload != "guard_miss_shifted"),
   adaptive_body_excess_vs_fast:($m.adaptive.body_bytes-$m["fast-zstd"].body_bytes),
   adaptive_framed_excess_vs_fast:($m.adaptive.framed_bytes-$m["fast-zstd"].framed_bytes),
   adaptive_framed_excess_fraction_vs_fast:(($m.adaptive.framed_bytes-$m["fast-zstd"].framed_bytes)/$m["fast-zstd"].framed_bytes),
   adaptive_body_excess_fraction_vs_fast:(($m.adaptive.body_bytes-$m["fast-zstd"].body_bytes)/$m["fast-zstd"].body_bytes),
   fast_body_excess_vs_legacy:($m["fast-zstd"].body_bytes-$m.zstd.body_bytes),
   adaptive_encode_cpu_ratio_vs_fast:($m.adaptive.median_encode_cpu_us_per_packet/$m["fast-zstd"].median_encode_cpu_us_per_packet)
  }))
}
