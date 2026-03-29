use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "astro-up-checker",
    version,
    about = "Check for new versions of astrophotography software"
)]
struct Cli {
    /// Path to manifests directory
    #[arg(short, long, default_value = "manifests")]
    manifests: PathBuf,

    /// Path to versions directory
    #[arg(long, default_value = "versions")]
    versions: PathBuf,

    /// Path to checker state file
    #[arg(short, long, default_value = "checker-state.json")]
    state: PathBuf,

    /// Maximum concurrent checks
    #[arg(short, long, default_value = "10")]
    concurrency: usize,

    /// Filter manifests by substring match on ID, category, or provider
    #[arg(short, long)]
    filter: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(if cli.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("astro-up-checker {}", env!("CARGO_PKG_VERSION"));

    // TODO: implement checking pipeline
    // 1. Load manifests from cli.manifests
    // 2. Apply filter if cli.filter is set
    // 3. Load checker state from cli.state
    // 4. Run checks with buffer_unordered(cli.concurrency)
    // 5. Write version files to cli.versions
    // 6. Update and write checker state
    // 7. Auto-create/close issues for persistent failures
    // 8. Print summary

    Ok(())
}
