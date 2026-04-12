mod commands;
mod dispatch;
mod reporter;

pub const TICK: &str = "✓";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dispatch::run().await
}
