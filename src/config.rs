// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

pub const APP_ID: &str = "dev.m4ul3r.CosmicExtAppletClaude";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IconDisplay {
    /// Only show session (5-hour) ring
    Session,
    /// Only show weekly ring
    Weekly,
    /// Show dual rings (default)
    #[default]
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, CosmicConfigEntry)]
#[version = 1]
pub struct ClaudeAppletConfig {
    /// Which usage indicator(s) to display in the icon
    pub icon_display: IconDisplay,
    /// Show Claude mascot alongside usage rings
    pub show_mascot: bool,
    /// Threshold percentage for warning state (yellow)
    pub warning_threshold: u8,
    /// Threshold percentage for critical state (red)
    pub critical_threshold: u8,
    /// Show percentage text next to icon in panel
    pub show_percentage_text: bool,
    /// API poll interval in minutes
    pub poll_interval_minutes: u32,
}

impl Default for ClaudeAppletConfig {
    fn default() -> Self {
        Self {
            icon_display: IconDisplay::default(),
            show_mascot: true,
            warning_threshold: 50,
            critical_threshold: 80,
            show_percentage_text: false,
            poll_interval_minutes: 60,
        }
    }
}

impl ClaudeAppletConfig {
    /// Validate and clamp config values to sensible ranges.
    /// Ensures warning_threshold < critical_threshold and values are within bounds.
    pub fn validate(&mut self) {
        self.warning_threshold = self.warning_threshold.min(100);
        self.critical_threshold = self.critical_threshold.clamp(self.warning_threshold.saturating_add(1), 100);
        self.poll_interval_minutes = self.poll_interval_minutes.clamp(1, 1440);
    }
}
