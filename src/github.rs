use crate::error::{GhExportError, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, warn};

const GITHUB_API_BASE: &str = "https://api.github.com";
const USER_AGENT_STRING: &str = "gh-export/0.1.0";

#[derive(Debug, Clone)]
pub struct GitHubClient {
    client: reqwest::Client,
    #[allow(dead_code)]
    token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: Owner,
    pub private: bool,
    pub html_url: String,
    pub description: Option<String>,
    pub fork: bool,
    pub created_at: String,
    pub updated_at: String,
    pub pushed_at: Option<String>,
    pub clone_url: String,
    pub ssh_url: String,
    pub size: u64,
    pub stargazers_count: u64,
    pub watchers_count: u64,
    pub language: Option<String>,
    pub archived: bool,
    pub disabled: bool,
    pub default_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub login: String,
    pub id: u64,
    #[serde(rename = "type")]
    pub owner_type: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RateLimitResponse {
    pub rate: RateLimit,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RateLimit {
    pub limit: u64,
    pub remaining: u64,
    pub reset: u64,
    pub used: u64,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
    #[allow(dead_code)]
    pub id: u64,
    #[allow(dead_code)]
    pub name: Option<String>,
    #[allow(dead_code)]
    pub public_repos: u64,
    #[allow(dead_code)]
    pub total_private_repos: Option<u64>,
}

impl GitHubClient {
    pub fn new(token: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| GhExportError::Auth("Invalid token format".to_string()))?,
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STRING));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self { client, token })
    }

    pub async fn get_authenticated_user(&self) -> Result<User> {
        let url = format!("{GITHUB_API_BASE}/user");
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(GhExportError::GitHubApi(format!(
                "Failed to get user info: {status} - {text}"
            )));
        }

        Ok(response.json().await?)
    }

    pub async fn list_user_repositories(&self, username: &str) -> Result<Vec<Repository>> {
        let mut repositories = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            debug!("Fetching repositories page {}", page);
            let url = format!(
                "{GITHUB_API_BASE}/users/{username}/repos?per_page={per_page}&page={page}"
            );

            let response = self.client.get(&url).send().await?;

            if response.status() == 404 {
                let url = format!(
                    "{GITHUB_API_BASE}/user/repos?per_page={per_page}&page={page}"
                );
                let response = self.client.get(&url).send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    return Err(GhExportError::GitHubApi(format!(
                        "Failed to list repositories: {status} - {text}"
                    )));
                }

                let repos: Vec<Repository> = response.json().await?;
                let is_last_page = repos.len() < per_page;
                repositories.extend(repos);

                if is_last_page {
                    break;
                }
            } else if response.status().is_success() {
                let repos: Vec<Repository> = response.json().await?;
                let is_last_page = repos.len() < per_page;
                repositories.extend(repos);

                if is_last_page {
                    break;
                }
            } else {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(GhExportError::GitHubApi(format!(
                    "Failed to list repositories: {status} - {text}"
                )));
            }

            page += 1;
        }

        Ok(repositories)
    }

    #[allow(dead_code)]
    pub async fn check_rate_limit(&self) -> Result<RateLimitResponse> {
        let url = format!("{GITHUB_API_BASE}/rate_limit");
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(GhExportError::GitHubApi(format!(
                "Failed to check rate limit: {status} - {text}"
            )));
        }

        Ok(response.json().await?)
    }

    #[allow(dead_code)]
    pub async fn wait_for_rate_limit(&self) -> Result<()> {
        let rate_limit = self.check_rate_limit().await?;

        if rate_limit.rate.remaining == 0 {
            let reset_time = chrono::DateTime::from_timestamp(rate_limit.rate.reset as i64, 0)
                .unwrap_or_else(chrono::Utc::now);
            let now = chrono::Utc::now();

            if reset_time > now {
                let wait_duration = reset_time - now;
                warn!(
                    "Rate limit exceeded. Waiting {} seconds until reset...",
                    wait_duration.num_seconds()
                );
                tokio::time::sleep(wait_duration.to_std().unwrap_or(Duration::from_secs(60))).await;
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_token(&self) -> &str {
        &self.token
    }
}
