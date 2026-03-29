use astro_up_checker::hash;
use astro_up_checker::providers::{self, CheckOutcome};
use astro_up_checker::version_writer::DiscoveredVersion;
use astro_up_shared::manifest::Manifest;
use astro_up_shared::state::CheckerState;
use astro_up_shared::template;
use clap::Parser;
use futures::stream::{self, StreamExt};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

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

struct Summary {
    checked: u32,
    new_versions: Vec<String>,
    failed: Vec<String>,
    skipped: Vec<String>,
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

    // 1. Load manifests
    let all_manifests = load_manifests(&cli.manifests)?;

    // 2. Apply filter
    let manifests: Vec<&Manifest> = if let Some(ref filter) = cli.filter {
        all_manifests
            .iter()
            .filter(|m| matches_filter(m, filter))
            .collect()
    } else {
        all_manifests.iter().collect()
    };

    tracing::info!("checking {} manifests ({} concurrent)", manifests.len(), cli.concurrency);

    // 3. Load state
    let state = Arc::new(Mutex::new(CheckerState::read(&cli.state)?));

    // 4. Build HTTP client with retry middleware
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let raw_client = reqwest::Client::builder()
        .user_agent("astro-up-checker/0.1")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let client = ClientBuilder::new(raw_client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    // 5. Run checks with bounded concurrency
    let versions_dir = cli.versions.clone();
    let summary = Arc::new(Mutex::new(Summary {
        checked: 0,
        new_versions: Vec::new(),
        failed: Vec::new(),
        skipped: Vec::new(),
    }));

    let results: Vec<_> = stream::iter(manifests)
        .map(|manifest| {
            let client = client.clone();
            let state = state.clone();
            let summary = summary.clone();
            let versions_dir = versions_dir.clone();
            async move {
                process_manifest(manifest, &client, &state, &summary, &versions_dir).await;
            }
        })
        .buffer_unordered(cli.concurrency)
        .collect()
        .await;

    drop(results);

    // 6. Write updated state
    let state = state.lock().await;
    state.write(&cli.state)?;

    // 7. Print summary
    let summary = summary.lock().await;
    println!(
        "Checked {} manifests ({} concurrent)",
        summary.checked, cli.concurrency
    );
    if !summary.new_versions.is_empty() {
        println!("  New versions: {} ({})", summary.new_versions.len(), summary.new_versions.join(", "));
    }
    if !summary.failed.is_empty() {
        println!("  Failed: {} ({})", summary.failed.len(), summary.failed.join(", "));
    }
    if !summary.skipped.is_empty() {
        println!("  Skipped: {} (manual)", summary.skipped.len());
    }

    // Report persistent failures
    for (id, ms) in state.manifests.iter() {
        if ms.consecutive_failures >= 8 {
            if let Some(issue) = ms.issue_number {
                println!("  Persistent failure: {id} — {} consecutive, issue #{issue}", ms.consecutive_failures);
            }
        }
    }

    Ok(())
}

async fn process_manifest(
    manifest: &Manifest,
    client: &reqwest_middleware::ClientWithMiddleware,
    state: &Arc<Mutex<CheckerState>>,
    summary: &Arc<Mutex<Summary>>,
    versions_dir: &PathBuf,
) {
    let mut sum = summary.lock().await;
    sum.checked += 1;
    drop(sum);

    match providers::check_manifest(manifest, client).await {
        Ok(CheckOutcome::Found(result)) => {
            // Resolve autoupdate URL template if available
            let url = manifest
                .checkver
                .as_ref()
                .and_then(|cv| cv.autoupdate.as_ref())
                .and_then(|au| au.url.as_ref())
                .map(|tmpl| template::substitute(tmpl, &result.version))
                .or(result.url.clone());

            // Discover hash
            let sha256 = hash::discover_hash(
                manifest.checkver.as_ref().and_then(|cv| cv.hash.as_ref()),
                url.as_deref().unwrap_or(""),
                &result.version,
                client,
            )
            .await;

            let discovered = DiscoveredVersion {
                package_id: manifest.id.clone(),
                version: result.version.clone(),
                url: url.unwrap_or_default(),
                sha256,
                release_notes_url: result.release_notes_url,
                pre_release: result.pre_release,
            };

            match discovered.write(versions_dir) {
                Ok(Some(_path)) => {
                    let mut sum = summary.lock().await;
                    sum.new_versions.push(format!("{} {}", manifest.id, result.version));
                }
                Ok(None) => {
                    // Version already exists
                    tracing::debug!("{}: {} already exists", manifest.id, result.version);
                }
                Err(e) => {
                    tracing::error!("{}: failed to write version file: {e}", manifest.id);
                }
            }

            state.lock().await.record_success(&manifest.id);
        }
        Ok(CheckOutcome::Skipped { reason }) => {
            tracing::debug!("{}: skipped — {reason}", manifest.id);
            let mut sum = summary.lock().await;
            sum.skipped.push(manifest.id.clone());
        }
        Err(e) => {
            tracing::warn!("{}: {e}", manifest.id);
            state.lock().await.record_failure(&manifest.id, &e.to_string());
            let mut sum = summary.lock().await;
            sum.failed.push(manifest.id.clone());
        }
    }
}

fn load_manifests(dir: &PathBuf) -> anyhow::Result<Vec<Manifest>> {
    let mut manifests = Vec::new();
    for entry in walkdir::WalkDir::new(dir).max_depth(1) {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let content = std::fs::read_to_string(path)?;
        match toml::from_str::<Manifest>(&content) {
            Ok(m) => manifests.push(m),
            Err(e) => tracing::warn!("{}: {e}", path.display()),
        }
    }
    Ok(manifests)
}

fn matches_filter(manifest: &Manifest, filter: &str) -> bool {
    let filter_lower = filter.to_lowercase();
    manifest.id.to_lowercase().contains(&filter_lower)
        || manifest.category.to_lowercase().contains(&filter_lower)
        || manifest
            .checkver
            .as_ref()
            .map_or(false, |cv| cv.provider.to_lowercase().contains(&filter_lower))
}
