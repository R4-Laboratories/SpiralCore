// Independent read-only SC-OPT-004 receipt analysis. No production/probe execution.
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs};
fn n(v: &Value, k: &str) -> u64 { v[k].as_u64().expect(k) }
fn close(a: f64, b: f64) { assert!((a-b).abs() <= 1e-12 * a.abs().max(1.0), "{a} != {b}"); }
fn main() {
    let root = std::env::args().nth(1).expect("absolute evidence root");
    let mut cells: BTreeMap<String, BTreeMap<String, Vec<f64>>> = BTreeMap::new();
    let mut counts = [0u64; 5]; // rows, frames, scans, metadata calls, modeled bytes
    let populations = [0u64, 64, 256, 768];
    let arms = ["raw-node-budget-262144", "sparse-node-budget-262144", "raw-node-budget-1", "sparse-node-budget-1"];
    for block in 0..2 {
        let text = fs::read_to_string(format!("{root}/measured/block{block}-diagnostic.jsonl")).unwrap();
        let rows: Vec<Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(rows.len(), 17);
        let meta = &rows[0];
        assert_eq!(meta["block"], block);
        assert_eq!(meta["library_revision"], "f0966ebedcb8994f0985b54e9a2b961eeb8be914");
        assert_eq!(meta["library_source_sha256"], "93e6cfb9ee5a88d5eb724d6c290296d22c444c6a9a50b3b0ce7bbff5dbba5539");
        assert_eq!(meta["executable_sha256"], "ee2e57c73d5ecd28d692877ff17edf89ff678b15eda66b41edabd7b303f06e6d");
        for (index, row) in rows[1..].iter().enumerate() {
            let population_index = (index / 4 + block) % 4;
            let population = populations[population_index];
            let arm_index = (index % 4 + block + population_index) % 4;
            let arm = arms[arm_index];
            let sparse = arm_index % 2 == 1;
            let retained_manifest = arm_index == 0;
            assert_eq!(row["population"], population);
            assert_eq!(row["arm_label"], arm);
            assert_eq!(row["block"], block);
            assert_eq!(row["status"], "exact"); assert!(row["error"].is_null());
            assert_eq!(row["error_stage"], "");
            assert_eq!(row["base_object"], "c7fceaf53550e0ab1393604c2bc88eabd267676766ae90ad52459ece381822c0");
            assert_eq!(row["target_object"], "227bbf10e522740c837d03da96838331a00fe3dac2ca77c095ba1044467b0ebf");
            assert_eq!(row["object_bytes"], 1048576);
            assert_eq!(row["target_manifest_present"], retained_manifest);
            assert_eq!(row["graph_cache_after"]["entries"], 0);
            assert_eq!(row["graph_cache_after"]["resident_bytes"], 0);
            let before = &row["before"]; let after = &row["after"];
            for (v, publications, active) in [(before, population+1, 1), (after, population+2, 2)] {
                assert_eq!(n(v,"publications"), publications);
                assert_eq!(n(v,"active_publications"), active);
                assert_eq!(n(v,"namespaces"), population/2+1);
            }
            assert_eq!(n(before,"active_bytes"), 1048657);
            let increment = 1048576 + 49 + if retained_manifest {32} else {0};
            assert_eq!(n(after,"publication_bytes")-n(before,"publication_bytes"), increment);
            assert_eq!(n(after,"active_bytes")-n(before,"active_bytes"), increment);
            assert_eq!(n(after,"manifests"), n(before,"manifests")+u64::from(retained_manifest));
            let frames = row["frame_samples"].as_array().unwrap();
            assert_eq!(frames.len(), if sparse {3} else {20});
            let mut stage = [[0u64;3];3]; let mut scan = [[0u64;2];3]; let mut bytes = 0;
            for (seq, frame) in frames.iter().enumerate() {
                let k = if seq == 0 {0} else if seq+1 == frames.len() {2} else {1};
                assert_eq!(n(frame,"kind"), k as u64+1); assert_eq!(n(frame,"sequence"), seq as u64);
                assert_eq!(n(frame,"reply_kind"), k as u64+101);
                let p=&frame["disk_usage"];
                for (clock, col) in [("cpu_ns",0),("wall_ns",1)] {
                    let cost=n(&frame["cost"],clock); let inner=n(p,clock); assert!(inner<=cost);
                    stage[k][col]+=cost; scan[k][col]+=inner;
                }
                stage[k][2]+=1;
                bytes+=n(frame,"request_bytes")+n(frame,"reply_bytes");
                if k==1 {
                    for value in p.as_object().unwrap().values() {
                        assert!(value==&json!(0) || value==&json!(false) || value.is_null());
                    }
                } else {
                    assert_eq!(p["overflow"],false); assert!(p["last_error"].is_null()); assert!(p["last_error_stage"].is_null());
                    for (key,want) in [("calls",1),("successes",1),("errors",0),("root_read_dir_calls",1),("root_entries",population/2+3),("root_file_type_calls",population/2+3),("namespace_read_dir_calls",population/2+1),("namespace_entries",population+1),("object_file_type_calls",population+1),("metadata_calls",population+1),("metadata_successes",population+1),("namespace_comparisons",population+1),("objects_counted",population+1),("returned_objects",population+1),("returned_all_bytes",n(before,"publication_bytes")),("returned_scoped_bytes",n(before,"active_bytes"))] { assert_eq!(n(p,key),want,"{key}"); }
                    counts[2]+=1; counts[3]+=population+1;
                }
            }
            assert_eq!(bytes,if sparse {536}else{1051138});
            let c = cells.entry(format!("N{population}/{arm}")).or_default();
            for (col,clock,label) in [(0,"cpu_ns","cpu"),(1,"wall_ns","wall")] {
                let total: u64=stage.iter().map(|s|s[col]).sum();
                let scans: u64=scan.iter().map(|s|s[col]).sum();
                for (s,name) in ["begin","data","finish"].iter().enumerate() {
                    assert_eq!(n(&row["handler_stages"][name],clock),stage[s][col]);
                    assert_eq!(n(&row["handler_stages"][name],"frames"),stage[s][2]);
                }
                assert_eq!(n(&row["handler_stages"]["total"],clock),total);
                let operation=n(&row["local_operation_total"],clock);
                assert!(operation>=n(&row["dispatch_including_instrumentation"],clock));
                assert!(n(&row["dispatch_including_instrumentation"],clock)>=total);
                assert!(total>0);
                for (name,value) in [(format!("scan_{label}_ns"),scans as f64),(format!("handler_{label}_ns"),total as f64),(format!("operation_{label}_ns"),operation as f64),(format!("scan_begin_{label}_ns"),scan[0][col] as f64),(format!("scan_finish_{label}_ns"),scan[2][col] as f64),(format!("scan_fraction_of_handler_{label}"),scans as f64/total as f64),(format!("scan_fraction_of_operation_{label}"),scans as f64/operation as f64)] { c.entry(name).or_default().push(value); }
            }
            c.entry("preparation_cpu_ns".into()).or_default().push(n(&row["frame_preparation"],"cpu_ns") as f64);
            c.entry("modeled_application_bytes".into()).or_default().push(bytes as f64);
            counts[0]+=1; counts[1]+=frames.len() as u64; counts[4]+=bytes;
        }
    }
    assert_eq!(counts,[32,368,64,17472,16826784]); assert_eq!(cells.len(),16);
    let author: Value=serde_json::from_slice(&fs::read(format!("{root}/profile-analysis.json")).unwrap()).unwrap();
    let mut checked=0; let mut summaries=BTreeMap::new();
    for (cell,metrics) in &cells {
        let mut result=BTreeMap::new();
        for (metric,values) in metrics {
            assert_eq!(values.len(),2);
            let expected=&author["cells"][cell][metric];
            for (i,value) in values.iter().enumerate(){close(*value,expected["values"][i].as_f64().unwrap());}
            let median=(values[0]+values[1])/2.0;
            close(median,expected["midpoint_median"].as_f64().unwrap());
            close(values[0].min(values[1]),expected["minimum"].as_f64().unwrap());
            close(values[0].max(values[1]),expected["maximum"].as_f64().unwrap());
            result.insert(metric,json!({"values":values,"midpoint_median":median})); checked+=1;
        }
        summaries.insert(cell,result);
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema":"spiralcore.independent-cache-profile-review.v1","result":"passed","observations":counts[0],"frames":counts[1],"scans":counts[2],"metadata_api_calls":counts[3],"modeled_application_bytes":counts[4],"metric_records_compared":checked,"independent_cells":summaries,"limitations":["Two descriptive observations per cell; no optimization or production speed inference","Counts describe Rust API calls, not syscall or physical disk I/O counts","Nested instrumented CPU and wall clocks include instrumentation overhead","Exact status and known object IDs checked; independent review does not rerun the preserved byte comparison","No WAN or energy inference"]})).unwrap());
}
