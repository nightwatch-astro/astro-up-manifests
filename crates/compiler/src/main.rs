use astro_up_compiler::{compile, manifest, schema, version_file};
use clap::Parser;
use rusqlite::Connection;
use std::path::PathBuf;
use std::process::ExitCode;

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

fn main() -> ExitCode {
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

    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            tracing::error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    // 1. Load and validate manifests
    let result = manifest::load_manifests(&cli.manifests)?;

    // 2. Validate-only mode
    if cli.validate {
        if result.errors.is_empty() {
            println!("All {} manifests valid.", result.manifests.len());
            return Ok(ExitCode::SUCCESS);
        } else {
            println!("{} errors found:", result.errors.len());
            for err in &result.errors {
                println!("  {err}");
            }
            return Ok(ExitCode::from(2));
        }
    }

    // 3. Create SQLite database
    if cli.output.exists() {
        std::fs::remove_file(&cli.output)?;
    }
    let conn = Connection::open(&cli.output)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

    // 4. Create schema
    schema::create_schema(&conn)?;

    // 5. Compile manifests into tables
    compile::compile_manifests(&conn, &result.manifests)?;

    // 6. Aggregate version files
    let version_count = version_file::aggregate_versions(&conn, &cli.versions)?;

    // 7. Summary
    println!(
        "Compiled {} manifests, {} versions into {}",
        result.manifests.len(),
        version_count,
        cli.output.display()
    );
    if !result.errors.is_empty() {
        println!(
            "  ({} manifests skipped due to errors)",
            result.errors.len()
        );
    }

    Ok(ExitCode::SUCCESS)
}
