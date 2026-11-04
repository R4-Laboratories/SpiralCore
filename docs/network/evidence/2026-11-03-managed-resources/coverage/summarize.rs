use std::{collections::BTreeMap, fs};
fn main() {
    let text = fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let mut selected = false;
    let mut lines = BTreeMap::<u32, u64>::new();
    for line in text.lines() {
        if let Some(path) = line.strip_prefix("SF:") {
            selected = path.ends_with("/src/installed_resources.rs");
        }
        if selected && let Some(value) = line.strip_prefix("DA:") {
            let mut fields = value.split(',');
            let number: u32 = fields.next().unwrap().parse().unwrap();
            let hits: u64 = fields.next().unwrap().parse().unwrap();
            if number < 375 {
                lines.entry(number).and_modify(|n| *n = (*n).max(hits)).or_insert(hits);
            }
        }
    }
    assert!(!lines.is_empty());
    let hits = lines.values().filter(|n| **n > 0).count();
    let missing: Vec<_> = lines.iter().filter(|(_,n)| **n == 0).map(|(line,_)| line.to_string()).collect();
    println!("{{\"scope\":\"installed_resources.rs production lines 1-374; inline tests excluded\",\"mapped_lines\":{},\"covered_lines\":{},\"percent\":{:.4},\"uncovered_lines\":[{}],\"branch_coverage\":null}}", lines.len(), hits, 100.0*hits as f64/lines.len() as f64, missing.join(","));
}
