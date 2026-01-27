use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("ralphctl v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
