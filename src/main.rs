#[tokio::main]
async fn main() -> anyhow::Result<()> {
    charms::cli::run().await
}
