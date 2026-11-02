// Diagnostic-only wrapper: no successful completion exists in the failed run.
// Inject it solely to reach the real service receipts. The actual incomplete
// capture must still fail; analyze() must still reject the terminal exit.
fn read(path: &Path) -> Result<Value> {
    if path.ends_with("lan-t0-raw/complete.json") {
        return Ok(json!({"ok":true,"stores_removed":true,"object_deliveries":12,"operations":12,"harness_wall_us":0}));
    }
    read_original(path)
}
#[test]
fn failed_run_service_shape_reaches_capture_rejection() {
    let root=Path::new("/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore/docs/network/evidence/2026-11-02-runtime-gateway/failed-run-41373268385/campaign");
    let m=read(&root.join("fixtures-0/manifest.json")).unwrap();
    manifest_check(&m,0).unwrap();
    let mut rows=Rows::default();
    let error=session(root,"lan",0,"raw",&m,&mut rows).unwrap_err();
    assert_eq!(error.to_string(),"incomplete capture");
    assert_eq!((rows.operations.len(),rows.deliveries,rows.cumulative.len(),rows.prefix.len()),(12,12,14,14));
    assert_eq!(rows.sessions.len(),0);
    assert_eq!(analyze(root).unwrap_err().to_string(),"campaign terminal exit is not zero");
    println!("Diagnostic only: all12 service operations and exact reopen joins accepted; real incomplete capture and full failed campaign still rejected. No performance report emitted.");
}
