use clap::Parser;

#[derive(Parser)]
#[command(
    name = "astro-up-checker",
    version,
    about = "Check for new versions of astrophotography software"
)]
struct Cli {
    /// Path to manifests directory
    #[arg(short, long, default_value = "manifests")]
    manifests: String,

    /// Check specific vendor only
    #[arg(short, long)]
    vendor: Option<String>,

    /// Output directory for version files
    #[arg(short, long, default_value = "versions")]
    output: String,

    /// Delay between checks in seconds
    #[arg(short, long, default_value = "0")]
    delay: u64,

    /// Maximum concurrent checks
    #[arg(long, default_value = "10")]
    concurrency: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let _cli = Cli::parse();
    println!("astro-up-checker {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
