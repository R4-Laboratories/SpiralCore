//! Cooperating file/object interface to the SpiralCore gateway.
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> spiralcore_gateway::measurement::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 || args[1] != "run" {
        return Err(spiralcore_gateway::measurement::invalid(
            "usage: spiralcore-gateway-object run CONFIG.json",
        )
        .into());
    }
    let bytes =
        spiralcore_gateway::object_service::read_bounded(std::path::Path::new(&args[2]), 65536)?;
    let config = serde_json::from_slice(&bytes)?;
    spiralcore_gateway::object_service::run(config).await
}
