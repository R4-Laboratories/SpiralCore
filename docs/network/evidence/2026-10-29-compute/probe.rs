use cpu_time::ProcessTime;
use spiralcore_transfer::{dag::{self, Graph, Node}, hex, object_id};
use std::{hint::black_box, mem::size_of, time::Instant};
use uor_r4_object_store::{Limits, Store};

fn measure<T>(name: &str, samples: usize, batch: usize, mut f: impl FnMut() -> T) {
    drop(black_box(f()));
    let mut walls = Vec::new();
    let mut cpus = Vec::new();
    for _ in 0..samples {
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        for _ in 0..batch { drop(black_box(f())); }
        walls.push(wall.elapsed().as_nanos() as f64 / batch as f64 / 1000.0);
        cpus.push(cpu.elapsed().as_nanos() as f64 / batch as f64 / 1000.0);
    }
    walls.sort_by(f64::total_cmp);
    cpus.sort_by(f64::total_cmp);
    println!("{{\"operation\":\"{name}\",\"samples\":{samples},\"batch\":{batch},\"wall_micros_min\":{},\"wall_micros_median\":{},\"wall_micros_max\":{},\"cpu_micros_min\":{},\"cpu_micros_median\":{},\"cpu_micros_max\":{}}}", walls[0], walls[samples/2], walls[samples-1], cpus[0], cpus[samples/2], cpus[samples-1]);
}
fn limits() -> Limits {
    Limits { max_object_bytes: 64<<20, max_records: 262144, max_record_bytes: 262144*49, max_returned_bytes: 64<<20, max_visits: dag::work_bound(1<<20), max_occurrences: dag::work_bound(1<<20) }
}
fn describe(name: &str, graph: &Graph, base: Option<&Graph>) {
    let leaves = graph.nodes.values().filter(|n| matches!(n, Node::Leaf(_))).count();
    let encoded = graph.nodes.values().map(|n| n.encode().len()).sum::<usize>();
    let delta = base.map(|b| graph.delta(b));
    println!("{{\"fixture\":\"{name}\",\"bytes\":{},\"object\":\"{}\",\"unique_leaves\":{leaves},\"unique_parents\":{},\"unique_nodes\":{},\"roots\":{},\"node_record_bytes\":{encoded},\"delta_nodes\":{},\"delta_record_bytes\":{}}}", graph.manifest.len,hex(&graph.manifest.object),graph.nodes.len()-leaves,graph.nodes.len(),graph.manifest.roots.len(),delta.as_ref().map_or(0,Vec::len),delta.as_ref().map_or(0,|d|d.iter().map(|n|n.encode().len()).sum::<usize>()));
}
fn main() {
    let mut state = 0x9e3779b97f4a7c15u64;
    let bytes: Vec<u8> = (0..1<<20).map(|_| {state^=state<<13;state^=state>>7;state^=state<<17;state as u8}).collect();
    let graph = Graph::fold(&bytes,262144).unwrap();
    assert_eq!(graph.rebuild().unwrap(),bytes);
    let mut changed = bytes.clone();
    for b in &mut changed[524288..524320] { *b ^= 0xa5; }
    let changed_graph = Graph::fold(&changed,262144).unwrap();
    assert_eq!(changed_graph.rebuild().unwrap(),changed);
    let mut scattered = bytes.clone();
    for i in 0..32 { for b in &mut scattered[i*32768..i*32768+32] { *b ^= 0xa5; } }
    let scattered_graph = Graph::fold(&scattered,262144).unwrap();
    assert_eq!(scattered_graph.rebuild().unwrap(),scattered);
    let mut prefixed = vec![0xa5]; prefixed.extend_from_slice(&bytes);
    let prefix_graph = Graph::fold(&prefixed,262144).unwrap();
    assert_eq!(prefix_graph.rebuild().unwrap(),prefixed);
    let repeated:Vec<u8>=(0..1<<20).map(|n|(n%64) as u8).collect();
    let repeated_graph=Graph::fold(&repeated,262144).unwrap();
    assert_eq!(repeated_graph.rebuild().unwrap(),repeated);
    let mut core=Store::new(limits());
    let folded=core.fold(&bytes).unwrap();
    assert_eq!(core.rebuild(&folded.manifest).unwrap().bytes(),bytes);
    println!("{{\"method\":\"one warmup; sequential local process; optimized release library; allocation/drop included; page caches uncontrolled; no network, disk publication or energy measurement\",\"node_rust_value_bytes\":{},\"legacy_record_bytes\":{}}}",size_of::<Node>(),core.retained_record_bytes());
    describe("random_1MiB",&graph,None);
    describe("localized_32",&changed_graph,Some(&graph));
    describe("scattered_32x32",&scattered_graph,Some(&graph));
    describe("prefix_1",&prefix_graph,Some(&graph));
    describe("periodic_64",&repeated_graph,None);
    measure("object_id_random_1MiB",7,100,||object_id(black_box(&bytes)));
    measure("copy_vec_random_1MiB",7,100,||black_box(&bytes).clone());
    measure("legacy_store_fold_random_1MiB",7,1,||{let mut s=Store::new(limits());let folded=s.fold(black_box(&bytes)).unwrap();(s,folded)});
    measure("legacy_store_rebuild_random_1MiB",7,1,||core.rebuild(black_box(&folded.manifest)).unwrap());
    measure("graph_fold_random_1MiB",7,1,||Graph::fold(black_box(&bytes),262144).unwrap());
    measure("graph_fold_localized_32",7,1,||Graph::fold(black_box(&changed),262144).unwrap());
    measure("graph_clone_random_1MiB",7,1,||black_box(&graph).clone());
    measure("graph_rebuild_random_1MiB",7,1,||black_box(&graph).rebuild().unwrap());
    measure("graph_missing_random_1MiB",7,1,||dag::missing(black_box(&graph.manifest),black_box(&graph.nodes)).unwrap());
    measure("all_node_ids_random_1MiB",7,1,||{for n in graph.nodes.values() {black_box(n.id());}});
    measure("all_node_encodes_random_1MiB",7,1,||{let mut total=0;for n in graph.nodes.values() {total+=black_box(n.encode()).len();}total});
    measure("delta_exact_same_base",7,1,||black_box(&graph).delta(black_box(&graph)));
    measure("delta_localized_against_reused_base",7,1,||black_box(&changed_graph).delta(black_box(&graph)));
    measure("graph_fold_periodic_64_1MiB",7,1,||Graph::fold(black_box(&repeated),262144).unwrap());
    measure("graph_rebuild_periodic_64_1MiB",7,1,||black_box(&repeated_graph).rebuild().unwrap());
}
