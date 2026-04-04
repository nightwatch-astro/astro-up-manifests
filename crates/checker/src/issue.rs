use astro_up_shared::manifest::Manifest;
use astro_up_shared::state::CheckerState;
use reqwest_middleware::ClientWithMiddleware;

const FAILURE_THRESHOLD: u32 = 8;

/// Process the checker state and auto-create/close GitHub issues for persistent failures.
/// Requires GITHUB_TOKEN env var and GITHUB_REPOSITORY (owner/repo) for authentication.
pub async fn process_issues(
    state: &mut CheckerState,
    client: &ClientWithMiddleware,
) -> anyhow::Result<IssueReport> {
    let token = match std::env::var("GITHUB_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            tracing::debug!("GITHUB_TOKEN not set, skipping issue management");
            return Ok(IssueReport::default());
        }
    };

    let repo = match std::env::var("GITHUB_REPOSITORY") {
        Ok(r) => r,
        Err(_) => {
            tracing::debug!("GITHUB_REPOSITORY not set, skipping issue management");
            return Ok(IssueReport::default());
        }
    };

    let mut report = IssueReport::default();

    // Collect IDs that need action (avoid borrowing state during iteration)
    let needs_create: Vec<String> = state
        .manifests
        .iter()
        .filter(|(_, ms)| ms.consecutive_failures >= FAILURE_THRESHOLD && ms.issue_number.is_none())
        .map(|(id, _)| id.clone())
        .collect();

    let needs_close: Vec<(String, u64)> = state
        .manifests
        .iter()
        .filter_map(|(id, ms)| {
            if ms.consecutive_failures == 0 {
                ms.issue_number.map(|n| (id.clone(), n))
            } else {
                None
            }
        })
        .collect();

    // Create issues for persistent failures
    for id in needs_create {
        let ms = &state.manifests[&id];
        let title = format!("Version check failing: {id}");
        let body = format!(
            "The version check for `{id}` has failed for {} consecutive pipeline runs.\n\nLast error: {}\n\nThis issue was auto-created by the checker pipeline.",
            ms.consecutive_failures,
            ms.last_error.as_deref().unwrap_or("unknown"),
        );

        match create_issue(&token, &repo, &title, &body, client).await {
            Ok(number) => {
                tracing::info!("created issue #{number} for {id}");
                state.manifests.get_mut(&id).unwrap().issue_number = Some(number);
                report.created.push((id, number));
            }
            Err(e) => {
                tracing::error!("failed to create issue for {id}: {e}");
            }
        }
    }

    // Close issues for resolved failures
    for (id, number) in needs_close {
        match close_issue(&token, &repo, number, client).await {
            Ok(()) => {
                tracing::info!("closed issue #{number} for {id}");
                state.manifests.get_mut(&id).unwrap().issue_number = None;
                report.closed.push((id, number));
            }
            Err(e) => {
                tracing::error!("failed to close issue #{number} for {id}: {e}");
            }
        }
    }

    Ok(report)
}

#[derive(Default)]
pub struct IssueReport {
    pub created: Vec<(String, u64)>,
    pub closed: Vec<(String, u64)>,
}

async fn create_issue(
    token: &str,
    repo: &str,
    title: &str,
    body: &str,
    client: &ClientWithMiddleware,
) -> anyhow::Result<u64> {
    let url = format!("https://api.github.com/repos/{repo}/issues");
    let payload = serde_json::to_vec(&serde_json::json!({
        "title": title,
        "body": body,
        "labels": ["checker-failure"]
    }))?;
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .body(payload)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub API returned {status}: {text}");
    }

    let json: serde_json::Value = resp.json().await?;
    let number = json["number"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("missing issue number in response"))?;

    Ok(number)
}

async fn close_issue(
    token: &str,
    repo: &str,
    number: u64,
    client: &ClientWithMiddleware,
) -> anyhow::Result<()> {
    let url = format!("https://api.github.com/repos/{repo}/issues/{number}");
    let payload = serde_json::to_vec(&serde_json::json!({
        "state": "closed",
        "state_reason": "completed"
    }))?;
    let resp = client
        .patch(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .body(payload)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub API returned {status}: {text}");
    }

    Ok(())
}

/// Create or update a GitHub issue that reminds maintainers to review manual-check packages.
/// Requires `GITHUB_TOKEN` and `GITHUB_REPOSITORY` env vars. Silently skips if either is unset.
pub async fn process_manual_reminders(
    state: &mut CheckerState,
    manifests: &[Manifest],
    client: &ClientWithMiddleware,
) -> anyhow::Result<()> {
    let token = match std::env::var("GITHUB_TOKEN") {
        Ok(t) => t,
        Err(_) => return Ok(()),
    };
    let repo = match std::env::var("GITHUB_REPOSITORY") {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };

    // Find all manual-check packages
    let manual_packages: Vec<&Manifest> = manifests
        .iter()
        .filter(|m| m.checkver.as_ref().is_some_and(|cv| cv.provider == "manual"))
        .collect();

    if manual_packages.is_empty() {
        return Ok(());
    }

    // Build markdown table
    let now = chrono::Utc::now();
    let mut body = String::from(
        "The following packages use manual version checking and may need review:\n\n\
         | Package | Last Updated | Days Since Update |\n\
         |---------|-------------|-------------------|\n",
    );

    for m in &manual_packages {
        let last_update = state
            .manifests
            .get(&m.id)
            .and_then(|ms| ms.last_manual_update);
        let days = last_update.map(|t| (now - t).num_days()).unwrap_or(-1);
        let days_str = if days < 0 {
            "never".to_string()
        } else {
            format!("{days}")
        };
        let date_str = last_update
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "never".to_string());
        body.push_str(&format!("| {} | {} | {} |\n", m.id, date_str, days_str));
    }

    body.push_str("\n*This issue is auto-updated by the checker pipeline.*");

    let title = "Manual version check reminder";

    if let Some(issue_num) = state.manual_reminder_issue {
        // Update existing issue
        update_issue(&token, &repo, issue_num, &body, client).await?;
        tracing::info!("updated manual reminder issue #{issue_num}");
    } else {
        // Create new issue
        let number = create_reminder_issue(&token, &repo, title, &body, client).await?;
        state.manual_reminder_issue = Some(number);
        tracing::info!("created manual reminder issue #{number}");
    }

    Ok(())
}

async fn create_reminder_issue(
    token: &str,
    repo: &str,
    title: &str,
    body: &str,
    client: &ClientWithMiddleware,
) -> anyhow::Result<u64> {
    let url = format!("https://api.github.com/repos/{repo}/issues");
    let payload = serde_json::to_vec(&serde_json::json!({
        "title": title,
        "body": body,
        "labels": ["manual-check"]
    }))?;
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .body(payload)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub API returned {status}: {text}");
    }

    let json: serde_json::Value = resp.json().await?;
    let number = json["number"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("missing issue number in response"))?;

    Ok(number)
}

async fn update_issue(
    token: &str,
    repo: &str,
    number: u64,
    body: &str,
    client: &ClientWithMiddleware,
) -> anyhow::Result<()> {
    let url = format!("https://api.github.com/repos/{repo}/issues/{number}");
    let payload = serde_json::to_vec(&serde_json::json!({
        "body": body
    }))?;
    let resp = client
        .patch(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .body(payload)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub API returned {status}: {text}");
    }

    Ok(())
}
