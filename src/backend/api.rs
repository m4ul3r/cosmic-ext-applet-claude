// SPDX-License-Identifier: GPL-3.0-only

use chrono::{DateTime, Utc};
use cosmic::iced::futures::SinkExt;
use cosmic::iced::{stream, Subscription};
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, error, warn};

const DEFAULT_POLL_INTERVAL_MINUTES: u32 = 60;
const USAGE_API_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Debug, Clone, Default)]
pub struct UsageUpdate {
    pub has_credentials: bool,
    pub subscription_type: String,
    pub session_usage_percent: f32,
    pub session_reset_time: Option<DateTime<Utc>>,
    pub weekly_usage_percent: f32,
    pub weekly_reset_time: Option<DateTime<Utc>>,
    pub opus_usage_percent: f32,
    pub sonnet_usage_percent: f32,
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Credentials {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OAuthCredentials>,
}

#[derive(Debug, Deserialize)]
struct OAuthCredentials {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: Option<i64>, // Unix timestamp in milliseconds
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
    seven_day_opus: Option<ModelUsage>,
    seven_day_sonnet: Option<ModelUsage>,
}

#[derive(Debug, Deserialize)]
struct UsageWindow {
    utilization: f32,
    resets_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelUsage {
    utilization: f32,
}

fn get_credentials_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join(".credentials.json"))
}

fn read_credentials() -> Option<(String, String)> {
    let path = get_credentials_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let creds: Credentials = serde_json::from_str(&content).ok()?;
    let oauth = creds.claude_ai_oauth?;

    // Check if token is expired (expires_at is Unix timestamp in milliseconds)
    // Add 5-minute buffer to prevent mid-request expiration
    if let Some(expires_at_ms) = oauth.expires_at {
        let expires_at_secs = expires_at_ms / 1000;
        if let Some(expiry) = DateTime::from_timestamp(expires_at_secs, 0) {
            let buffer = chrono::Duration::minutes(5);
            if expiry < Utc::now() + buffer {
                warn!("OAuth token has expired or is about to expire");
                return None;
            }
        }
    }

    let subscription = oauth.subscription_type.unwrap_or_else(|| "Unknown".to_string());
    Some((oauth.access_token, subscription))
}

async fn fetch_usage(client: &reqwest::Client, access_token: &str) -> Result<UsageResponse, String> {
    let response = client
        .get(USAGE_API_URL)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("anthropic-beta", "oauth-2025-04-20")
        .send()
        .await
        .map_err(|_| "Network request failed".to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        // Log status only, don't expose raw API response body
        return Err(format!("API error: HTTP {}", status.as_u16()));
    }

    let text = response.text().await.map_err(|_| "Failed to read response".to_string())?;
    serde_json::from_str::<UsageResponse>(&text).map_err(|_| "Failed to parse response".to_string())
}

pub fn api_subscription(poll_interval_minutes: u32) -> Subscription<UsageUpdate> {
    let interval = if poll_interval_minutes > 0 {
        poll_interval_minutes
    } else {
        DEFAULT_POLL_INTERVAL_MINUTES
    };

    Subscription::run_with_id(
        std::sync::Arc::new(("claude-api-usage", interval)),
        stream::channel(1, move |mut sender| async move {
            let poll_duration = Duration::from_secs(interval as u64 * 60);
            // Create client once and reuse for connection pooling
            let client = reqwest::Client::new();

            // Initial delay to let the UI settle
            tokio::time::sleep(Duration::from_secs(2)).await;

            loop {
                let update = match read_credentials() {
                    Some((token, subscription_type)) => {
                        debug!("Fetching Claude API usage data");
                        match fetch_usage(&client, &token).await {
                            Ok(usage) => {
                                let session_reset = usage
                                    .five_hour
                                    .as_ref()
                                    .and_then(|w| w.resets_at.as_ref())
                                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                                    .map(|dt| dt.with_timezone(&Utc));

                                let weekly_reset = usage
                                    .seven_day
                                    .as_ref()
                                    .and_then(|w| w.resets_at.as_ref())
                                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                                    .map(|dt| dt.with_timezone(&Utc));

                                let session_pct = usage.five_hour.as_ref().map(|w| w.utilization).unwrap_or(0.0);
                                let weekly_pct = usage.seven_day.as_ref().map(|w| w.utilization).unwrap_or(0.0);
                                let opus_pct = usage.seven_day_opus.as_ref().map(|m| m.utilization).unwrap_or(0.0);
                                let sonnet_pct = usage.seven_day_sonnet.as_ref().map(|m| m.utilization).unwrap_or(0.0);

                                debug!(
                                    "Parsed usage: session={:.1}%, weekly={:.1}%, opus={:.1}%, sonnet={:.1}%",
                                    session_pct, weekly_pct, opus_pct, sonnet_pct
                                );

                                UsageUpdate {
                                    has_credentials: true,
                                    subscription_type,
                                    session_usage_percent: session_pct,
                                    session_reset_time: session_reset,
                                    weekly_usage_percent: weekly_pct,
                                    weekly_reset_time: weekly_reset,
                                    opus_usage_percent: opus_pct,
                                    sonnet_usage_percent: sonnet_pct,
                                    last_error: None,
                                }
                            }
                            Err(e) => {
                                error!("Failed to fetch usage: {}", e);
                                UsageUpdate {
                                    has_credentials: true,
                                    subscription_type,
                                    last_error: Some(e),
                                    ..Default::default()
                                }
                            }
                        }
                    }
                    None => {
                        debug!("No valid credentials found");
                        UsageUpdate {
                            has_credentials: false,
                            subscription_type: "Not logged in".to_string(),
                            ..Default::default()
                        }
                    }
                };

                let _ = sender.send(update).await;
                tokio::time::sleep(poll_duration).await;
            }
        }),
    )
}
