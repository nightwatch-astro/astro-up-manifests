use astro_up_checker::providers::{self, CheckOutcome};
use astro_up_shared::manifest::{Checkver, Install, Manifest};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

fn test_client() -> reqwest_middleware::ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(2);
    let raw = reqwest::Client::builder()
        .user_agent("astro-up-checker-test")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap();
    ClientBuilder::new(raw)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

fn github_manifest(owner: &str, repo: &str) -> Manifest {
    Manifest {
        id: format!("{owner}-{repo}"),
        manifest_version: 1,
        name: repo.to_string(),
        description: None,
        publisher: None,
        homepage: None,
        category: "utility".into(),
        package_type: "application".into(),
        slug: repo.to_string(),
        tags: vec![],
        aliases: vec![],
        license: None,
        detection: None,
        install: Install {
            method: "zip_wrap".into(),
            scope: None,
            elevation: false,
            switches: Default::default(),
            exit_codes: vec![],
            success_codes: vec![],
        },
        checkver: Some(Checkver {
            provider: "github".into(),
            owner: Some(owner.into()),
            repo: Some(repo.into()),
            url: None,
            regex: None,
            version_format: Some("semver".into()),
            include_pre_release: false,
            css_selector: None,
            hash: None,
            autoupdate: None,
        }),
        hardware: None,
        backup: None,
        dependencies: None,
    }
}

/// Integration test against a real GitHub repo.
/// Ignored by default — run with `cargo test -- --ignored` or in CI with GITHUB_TOKEN.
#[tokio::test]
#[ignore]
async fn github_provider_finds_version() {
    let manifest = github_manifest("OpenPHDGuiding", "phd2");
    let client = test_client();

    let result = providers::check_manifest(&manifest, &client).await.unwrap();
    match result {
        CheckOutcome::Found(cr) => {
            assert!(!cr.version.is_empty(), "version should not be empty");
            assert!(cr.release_notes_url.is_some(), "should have release notes URL");
        }
        CheckOutcome::Skipped { reason } => {
            panic!("expected Found, got Skipped: {reason}");
        }
    }
}
