#[tokio::main(flavor = "current_thread")]
async fn main() -> spiralcore_gateway::measurement::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 || args[1] != "run" {
        return Err(spiralcore_gateway::measurement::invalid(
            "usage: spiralcore-gateway run CONFIG.json",
        )
        .into());
    }
    let bytes = std::fs::read(&args[2])?;
    if bytes.len() > 65536 {
        return Err(spiralcore_gateway::measurement::invalid("config too large").into());
    }
    let config: spiralcore_gateway::service::Config = serde_json::from_slice(&bytes)?;
    config.validate()?;
    #[cfg(target_os = "linux")]
    {
        spiralcore_gateway::service::run(config).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(spiralcore_gateway::measurement::invalid("TUN service requires Linux").into())
    }
}
