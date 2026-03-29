use clap::Parser;

#[derive(Parser)]
#[command(
    name = "astro-up-compiler",
    version,
    about = "Compile TOML manifests into SQLite catalog"
)]
struct Cli {
    /// Path to manifests directory
    #[arg(short, long, default_value = "manifests")]
    manifests: String,

    /// Path to versions directory
    #[arg(short, long, default_value = "versions")]
    versions: String,

    /// Output SQLite database path
    #[arg(short, long, default_value = "catalog.db")]
    output: String,

    /// Validate manifests only (dry-run)
    #[arg(long)]
    validate: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let _cli = Cli::parse();
    println!("astro-up-compiler {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
