use std::{collections::{BTreeMap, BTreeSet}, fs, process::Command};
fn main() {
    let root = "/var/tmp/spiralcore-installed-supervisor-20261103/source/";
    let mut hits: BTreeMap<String, BTreeMap<u32, u64>> = BTreeMap::new();
    let mut file = None;
    for line in fs::read_to_string("/tmp/spiralcore-installed-supervisor-20261103/coverage-export/owned.lcov").unwrap().lines() {
        if let Some(p) = line.strip_prefix("SF:") { file = p.strip_prefix(root).map(str::to_owned); }
        if let (Some(p), Some(v)) = (&file, line.strip_prefix("DA:")) {
            let mut v = v.split(',');
            let n: u32 = v.next().unwrap().parse().unwrap();
            let count: u64 = v.next().unwrap().parse().unwrap();
            hits.entry(p.clone()).or_default().entry(n).and_modify(|old| *old = (*old).max(count)).or_insert(count);
        }
    }
    // Explicit source-qualified scopes exclude the entire separate test modules.
    // Main/report exposes all unmapped platform files instead of inventing zeros.
    let specs = [
        ("installed_supervisor.rs", 1, 512), ("installed_worker.rs", 1, 394),
        ("installed_worker/codec.rs", 1, 125), ("installed_worker/io_util.rs", 1, 167),
        ("host_budget.rs", 1, 422), ("orchestrator.rs", 1, 774),
        ("installed_owner.rs", 1, 10000), ("linux_host.rs", 1, 630),
        ("main.rs", 1, 10000), ("host.rs", 1, 10000),
    ];
    let mut total = (0, 0); let mut all = (0, 0);
    println!("file\tchanged_hit\tchanged_mapped\tpercent\twhole_scope_hit\twhole_scope_mapped\tunhit_changed");
    for (name, start, end) in specs {
        let p = format!("crates/spiralcore-mvm-live/src/{name}");
        let diff = Command::new("git").args(["diff", "--unified=0", "86cae0a509deb44043265c0dd16128fd43509cb1", "f1f30b8fb9890ea22af0b06c64be080a4c952128", "--", &p]).output().unwrap();
        assert!(diff.status.success());
        let mut added = BTreeSet::new(); let mut n = 0;
        for line in String::from_utf8(diff.stdout).unwrap().lines() {
            if line.starts_with("@@ ") {
                let right = line.split_whitespace().nth(2).unwrap().trim_start_matches('+');
                n = right.split(',').next().unwrap().parse().unwrap();
            } else if line.starts_with("+++") || line.starts_with("---") {} else if line.starts_with('+') { added.insert(n); n += 1; }
            else if !line.starts_with('-') && !line.starts_with('\\') { n += 1; }
        }
        let Some(lines) = hits.get(&p) else { println!("{p}\tUNMAPPED\tUNMAPPED\tNA\tNA\tNA\tNA"); continue; };
        let valid = |line: u32| {
            line >= start && line <= end &&
            !(name == "orchestrator.rs" && (415..=449).contains(&line)) &&
            !(name == "installed_owner.rs" && [30,31,32,458,459,473,474,476,477,494,495,504,505,507,508,572,573,574].contains(&line))
        };
        let whole: Vec<_> = lines.iter().filter(|(line,_)| valid(**line)).collect();
        let changed: Vec<_> = whole.iter().copied().filter(|(line,_)| added.contains(line)).collect();
        let h = changed.iter().filter(|(_,count)| **count>0).count();
        let wh = whole.iter().filter(|(_,count)| **count>0).count();
        let unhit: Vec<_> = changed.iter().filter(|(_,count)| **count==0).map(|(line,_)| line.to_string()).collect();
        println!("{p}\t{h}\t{}\t{:.2}\t{wh}\t{}\t{}",changed.len(),100.0*h as f64/changed.len().max(1) as f64,whole.len(),unhit.join(","));
        total.0+=h; total.1+=changed.len(); all.0+=wh; all.1+=whole.len();
    }
    println!("TOTAL_MAPPED\t{}\t{}\t{:.2}\t{}\t{}\tNA",total.0,total.1,100.0*total.0 as f64/total.1 as f64,all.0,all.1);
}
