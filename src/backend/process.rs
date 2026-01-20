// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::{futures::SinkExt, Subscription};
use cosmic::iced_futures::stream;
use std::time::Duration;

/// Message returned from the process detection subscription
#[derive(Debug, Clone)]
pub struct ProcessUpdate {
    pub count: usize,
}

/// Subscription that polls for running claude processes
pub fn process_subscription() -> Subscription<ProcessUpdate> {
    Subscription::run_with_id(
        "claude-process-watcher",
        stream::channel(10, move |mut output| async move {
            loop {
                // Poll every 5 seconds (process count rarely changes rapidly)
                tokio::time::sleep(Duration::from_secs(5)).await;

                let count = count_claude_processes().await;
                let _ = output.send(ProcessUpdate { count }).await;
            }
        }),
    )
}

/// Count running claude processes by scanning /proc
async fn count_claude_processes() -> usize {
    tokio::task::spawn_blocking(count_claude_processes_sync)
        .await
        .unwrap_or(0)
}

fn count_claude_processes_sync() -> usize {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return 0;
    };

    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();

        // Check if it's a PID directory (numeric name)
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        // Read only cmdline (contains both process name and arguments)
        let cmdline_path = path.join("cmdline");
        let Ok(cmdline) = std::fs::read_to_string(&cmdline_path) else {
            continue;
        };

        // Iterate directly over split without Vec allocation
        let mut found = false;
        for arg in cmdline.split('\0') {
            if arg.is_empty() {
                continue;
            }
            // Check for claude binary or node running claude
            if arg.ends_with("/claude") || arg == "claude" {
                found = true;
                break;
            }
            // Check for node processes running claude CLI
            if arg.contains("@anthropic") && arg.contains("claude") {
                found = true;
                break;
            }
        }
        if found {
            count += 1;
        }
    }

    count
}
