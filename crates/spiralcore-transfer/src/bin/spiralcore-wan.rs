//! Short-lived loopback origin for an authenticated synthetic WAN campaign.
use spiralcore_transfer::{
    endpoint::{ColdGraphPolicy, Endpoint},
    wan::{self, BenchAuthority},
};
use std::{collections::BTreeSet, io::Write, net::TcpListener, path::Path, time::Duration};
fn main() {
    if let Err(e) = run() {
        eprintln!("WAN_ERROR {e}");
        std::process::exit(2);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("serve") {
        return Err("usage: spiralcore-wan serve --cache PATH --plan FILE [--bind 127.0.0.1:PORT] [--seconds 3600]".into());
    }
    let mut cache = None;
    let mut plan = None;
    let mut bind = "127.0.0.1:0".to_string();
    let mut seconds = 3600u64;
    let mut seen = BTreeSet::new();
    while let Some(k) = args.next() {
        if !seen.insert(k.clone()) {
            return Err("duplicate option".into());
        }
        let v = args.next().ok_or("missing value")?;
        match k.as_str() {
            "--cache" => cache = Some(v),
            "--plan" => plan = Some(v),
            "--bind" => bind = v,
            "--seconds" => seconds = v.parse()?,
            _ => return Err("unknown option".into()),
        }
    }
    let addr: std::net::SocketAddr = bind.parse()?;
    if !addr.ip().is_loopback() || seconds == 0 || seconds > 3600 {
        return Err("loopback and 1..3600 seconds required".into());
    }
    let token =
        std::env::var("SPIRALCORE_WAN_TOKEN").map_err(|_| "SPIRALCORE_WAN_TOKEN is required")?;
    wan::parse_id(&token).map_err(|_| "capability must be 64 lowercase hexadecimal characters")?;
    let plan: serde_json::Value =
        serde_json::from_slice(&std::fs::read(plan.ok_or("--plan required")?)?)?;
    let policy = match (
        plan.get("schema").and_then(|v| v.as_str()),
        plan.get("cold_graph_warming"),
    ) {
        (Some("spiralcore.wan.plan.v4"), Some(serde_json::Value::Bool(false))) => {
            ColdGraphPolicy::Disabled
        }
        (Some("spiralcore.wan.plan.v4"), _) | (_, Some(_)) => {
            return Err("unsupported or missing warming policy".into())
        }
        _ => ColdGraphPolicy::EagerBounded,
    };
    let codec_cache_enabled = match plan.get("codec_cache_enabled") {
        None => false,
        Some(serde_json::Value::Bool(enabled))
            if plan.get("schema").and_then(|v| v.as_str()) == Some("spiralcore.wan.plan.v4") =>
        {
            *enabled
        }
        _ => return Err("invalid codec cache configuration; v4 boolean required".into()),
    };
    let scopes = plan
        .get("allowed_scopes")
        .and_then(|x| x.as_array())
        .ok_or("plan allowed_scopes array required")?
        .iter()
        .map(|s| wan::parse_id(s.as_str().unwrap_or("")))
        .collect::<spiralcore_transfer::Result<BTreeSet<_>>>()?;
    if scopes.is_empty() || scopes.len() > 4096 {
        return Err("invalid scope count".into());
    }
    let cache = cache.ok_or("--cache required")?;
    let cache = Path::new(&cache);
    if cache.exists() && std::fs::read_dir(cache)?.next().is_some() {
        return Err(
            "each campaign requires a fresh cache; retain old evidence and select a new path"
                .into(),
        );
    }
    let mut endpoint = Endpoint::open_with_policy(cache, wan::limits(), BenchAuthority, policy)?;
    endpoint.set_codec_cache_enabled(codec_cache_enabled)?;
    let listener = TcpListener::bind(addr)?;
    eprintln!("POLICY cold_graph_policy={policy:?} codec_cache_enabled={codec_cache_enabled}");
    println!("READY addr={}", listener.local_addr()?);
    std::io::stdout().flush()?;
    if codec_cache_enabled {
        wan::serve_observed(
            listener,
            endpoint,
            &token,
            &scopes,
            Duration::from_secs(seconds),
            16384,
            |observation| {
                let stats = observation.codec_cache;
                let row = serde_json::json!({"event":"wan_request", "request_ordinal":observation.request_ordinal,
                "scope":spiralcore_transfer::hex(&observation.scope), "http_status":observation.http_status,
                "codec_cache":{"hits":stats.hits,"misses":stats.misses,"preparations":stats.preparations,
                    "evictions":stats.evictions,"entries":stats.entries,"resident_bytes":stats.resident_bytes}});
                let mut output = std::io::stderr().lock();
                serde_json::to_writer(&mut output, &row)
                    .map_err(|_| spiralcore_transfer::Error::Storage)?;
                writeln!(output)?;
                output.flush()?;
                Ok(())
            },
        )?;
    } else {
        wan::serve(
            listener,
            endpoint,
            &token,
            &scopes,
            Duration::from_secs(seconds),
            16384,
        )?;
    }
    Ok(())
}
