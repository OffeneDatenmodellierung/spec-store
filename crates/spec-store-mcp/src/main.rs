mod params;
mod tools;

use anyhow::Result;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("spec-store-mcp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // All logging goes to stderr — stdout is the JSON-RPC channel
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("spec-store MCP server starting");

    let server = tools::SpecStoreServer::new();
    let transport = rmcp::transport::io::stdio();

    server.serve(transport).await?;

    Ok(())
}
