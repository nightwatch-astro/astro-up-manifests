use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "astro-up-compiler",
    version,
    about = "Compile TOML manifests into SQLite catalog"
)]
struct Cli {
    /// Path to manifests directory
    #[arg(short, long, default_value = "manifests")]
    manifests: PathBuf,

    /// Path to versions directory
    #[arg(long, default_value = "versions")]
    versions: PathBuf,

    /// Output SQLite database path
    #[arg(short, long, default_value = "catalog.db")]
    output: PathBuf,

    /// Validate manifests only (dry-run)
    #[arg(long)]
    validate: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(if cli.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("astro-up-compiler {}", env!("CARGO_PKG_VERSION"));

    // TODO: implement compilation pipeline
    // 1. Load manifests from cli.manifests
    // 2. Validate all manifests
    // 3. If cli.validate: report results and exit
    // 4. Create SQLite schema at cli.output
    // 5. Compile manifests into tables
    // 6. Aggregate version files from cli.versions

    Ok(())
}
