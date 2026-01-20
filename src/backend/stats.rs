// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::{futures::SinkExt, Subscription};
use cosmic::iced_futures::stream;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;

/// Stats data from the Claude stats cache file
#[derive(Debug, Clone, Default)]
pub struct StatsUpdate {
    pub today_messages: u32,
    pub today_sessions: u32,
    pub total_messages: u32,
    pub total_sessions: u32,
    pub total_cost_usd: f64,
}

/// Raw JSON structure from stats-cache.json
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatsCache {
    #[serde(default)]
    total_messages: u32,
    #[serde(default)]
    total_sessions: u32,
    #[serde(default)]
    total_cost_usd: f64,
    #[serde(default)]
    daily_activity: Vec<DailyActivity>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DailyActivity {
    date: String,
    #[serde(default)]
    messages: u32,
    #[serde(default)]
    sessions: u32,
}

/// Subscription that monitors the stats-cache.json file
pub fn stats_subscription() -> Subscription<StatsUpdate> {
    Subscription::run_with_id(
        "claude-stats-watcher",
        stream::channel(10, move |mut output| async move {
            loop {
                // Poll every 30 seconds
                tokio::time::sleep(Duration::from_secs(30)).await;

                let stats = read_stats_file().await.unwrap_or_default();
                let _ = output.send(stats).await;
            }
        }),
    )
}

/// Get the path to the stats cache file
fn get_stats_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("stats-cache.json"))
}

/// Read and parse the stats cache file
async fn read_stats_file() -> Option<StatsUpdate> {
    let path = get_stats_path()?;

    let contents = tokio::fs::read_to_string(&path).await.ok()?;
    let cache: StatsCache = serde_json::from_str(&contents).ok()?;

    // Get today's date in YYYY-MM-DD format
    let today = get_today_string();

    // Find today's activity
    let today_activity = cache.daily_activity.iter().find(|a| a.date == today);

    let (today_messages, today_sessions) = match today_activity {
        Some(a) => (a.messages, a.sessions),
        None => (0, 0),
    };

    Some(StatsUpdate {
        today_messages,
        today_sessions,
        total_messages: cache.total_messages,
        total_sessions: cache.total_sessions,
        total_cost_usd: cache.total_cost_usd,
    })
}

/// Get today's date as YYYY-MM-DD string
fn get_today_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
