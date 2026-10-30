use spiralcore_transfer::{
    client,
    dag::Graph,
    endpoint::{Authority, ColdGraphPolicy, Context, Endpoint, Limits},
    protocol::Mode,
    Result,
};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-warm-policy-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Allow;
impl Authority for Allow {
    fn authorize(&self, _: &Context) -> Result<()> {
        Ok(())
    }
}
fn context() -> Context {
    Context {
        tenant: "t".into(),
        domain: "d".into(),
        sender: "s".into(),
        recipient: "r".into(),
        mailbox: "m".into(),
        capability: "c".into(),
        authorization_scope: "objects".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}

#[test]
fn disabled_skips_only_optional_raw_and_zstd_graph_warming() {
    let c = context();
    for mode in [Mode::Raw, Mode::Zstd, Mode::Dag] {
        let t = Temp::new();
        let bytes = vec![mode as u8 + 7; 4096];
        let mut ep =
            Endpoint::open_with_policy(&t.0, Limits::default(), Allow, ColdGraphPolicy::Disabled)
                .unwrap();
        client::transfer(&bytes, mode, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
        assert_eq!(
            ep.read_object(&c, spiralcore_transfer::object_id(&bytes))
                .unwrap(),
            bytes
        );
        assert_eq!(
            ep.graph_cache_stats().entries,
            usize::from(mode == Mode::Dag)
        );
        let file =
            t.0.join(spiralcore_transfer::hex(&c.namespace().unwrap()))
                .join(format!(
                    "{}.object",
                    spiralcore_transfer::hex(&spiralcore_transfer::object_id(&bytes))
                ));
        let published = std::fs::read(file).unwrap();
        assert_eq!(published[48], u8::from(mode == Mode::Dag));
    }
}

#[test]
fn default_policy_preserves_graph_readiness_and_disabled_reduces_admission_only() {
    let c = context();
    let bytes = vec![19; 32768];
    let t = Temp::new();
    let mut ep = Endpoint::open(&t.0, Limits::default(), Allow).unwrap();
    client::transfer(&bytes, Mode::Raw, None, [2; 32], |f| ep.handle(&c, f, 0)).unwrap();
    assert_eq!(ep.graph_cache_stats().entries, 1);
    let graph = Graph::fold(&bytes, 262144).unwrap();
    let mut target = bytes.clone();
    target[100] = 20;
    client::transfer(&target, Mode::Dag, Some(&graph), [3; 32], |f| {
        ep.handle(&c, f, 1)
    })
    .unwrap();
    let limited = Limits {
        resident_bytes: (2 << 20) + 2 * bytes.len() as u64 + 65536,
        ..Limits::default()
    };
    let t2 = Temp::new();
    let mut disabled =
        Endpoint::open_with_policy(&t2.0, limited, Allow, ColdGraphPolicy::Disabled).unwrap();
    client::transfer(&bytes, Mode::Raw, None, [4; 32], |f| {
        disabled.handle(&c, f, 0)
    })
    .unwrap();
}
