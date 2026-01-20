// SPDX-License-Identifier: GPL-3.0-only

mod backend;
mod config;
mod localize;

use backend::{api, process, stats};
use tracing::debug;
use chrono::{DateTime, Utc};
use config::{ClaudeAppletConfig, IconDisplay};
use cosmic::{
    Element, Task, app,
    app::Core,
    applet::{menu_button, padded_control},
    cosmic_config::CosmicConfigEntry,
    cosmic_theme::Spacing,
    iced::{
        Alignment, Color, Length, Subscription,
        platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup},
        widget::svg,
        window::Id,
    },
    iced_widget::{column, row},
    theme,
    widget::{
        button, divider, horizontal_space, mouse_area, text, progress_bar, slider, toggler,
    },
};
use cosmic_time::Timeline;
use std::cell::RefCell;
use std::f32::consts::PI;
use std::time::Instant;

pub fn run() -> cosmic::iced::Result {
    localize::localize();
    cosmic::applet::run::<ClaudeApplet>(())
}

/// Colors for usage levels
const COLOR_LOW: Color = Color::from_rgb(0.29, 0.87, 0.50);      // #4ade80 green
const COLOR_MEDIUM: Color = Color::from_rgb(0.98, 0.80, 0.08);   // #facc15 yellow
const COLOR_HIGH: Color = Color::from_rgb(0.97, 0.44, 0.44);     // #f87171 red
const COLOR_INACTIVE: Color = Color::from_rgb(0.5, 0.5, 0.5);    // gray
const COLOR_CLAUDE: Color = Color::from_rgb(0.85, 0.47, 0.34);   // #da7756 Claude orange

/// Usage level derived from percentage and thresholds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UsageLevel {
    Low,
    Medium,
    High,
}

/// Cached SVG data to avoid regenerating on every render
#[derive(Default)]
struct SvgCacheInner {
    session_percent: f32,
    session_color: Option<Color>,
    session_svg: Option<String>,
    weekly_percent: f32,
    weekly_color: Option<Color>,
    weekly_svg: Option<String>,
    mascot_color: Option<Color>,
    mascot_svg: Option<String>,
}

type SvgCache = RefCell<SvgCacheInner>;

pub struct ClaudeApplet {
    core: Core,
    popup: Option<Id>,
    timeline: Timeline,

    // Configuration
    config: ClaudeAppletConfig,

    // UI state
    settings_expanded: bool,

    // Process status
    process_count: usize,

    // Stats from file
    today_messages: u32,
    today_sessions: u32,
    cost_usd: f64,

    // API usage data
    has_credentials: bool,
    subscription_type: String,
    session_usage_percent: f32,
    session_reset_time: Option<DateTime<Utc>>,
    weekly_usage_percent: f32,
    weekly_reset_time: Option<DateTime<Utc>>,
    opus_usage_percent: f32,
    sonnet_usage_percent: f32,
    api_error: Option<String>,

    // SVG cache for performance
    svg_cache: SvgCache,
}

#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    Frame(Instant),
    ProcessUpdate(process::ProcessUpdate),
    StatsUpdate(stats::StatsUpdate),
    ApiUpdate(api::UsageUpdate),
    ConfigChanged(ClaudeAppletConfig),
    OpenTerminal,
    OpenSettings,
    ToggleSettings,
    // Settings messages
    CycleIconDisplay,
    ToggleMascot(bool),
    SetWarningThreshold(u8),
    SetCriticalThreshold(u8),
    TogglePercentageText(bool),
    SetPollInterval(u32),
}

impl cosmic::Application for ClaudeApplet {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = config::APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, app::Task<Self::Message>) {
        // Load config from cosmic-config or use defaults
        let mut config = cosmic::cosmic_config::Config::new(Self::APP_ID, ClaudeAppletConfig::VERSION)
            .ok()
            .and_then(|c| ClaudeAppletConfig::get_entry(&c).ok())
            .unwrap_or_default();
        config.validate();

        let applet = Self {
            core,
            popup: None,
            timeline: Timeline::default(),
            config,
            settings_expanded: false,
            process_count: 0,
            today_messages: 0,
            today_sessions: 0,
            cost_usd: 0.0,
            has_credentials: false,
            subscription_type: String::from("Unknown"),
            session_usage_percent: 0.0,
            session_reset_time: None,
            weekly_usage_percent: 0.0,
            weekly_reset_time: None,
            opus_usage_percent: 0.0,
            sonnet_usage_percent: 0.0,
            api_error: None,
            svg_cache: SvgCache::default(),
        };
        (applet, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let timeline = self
            .timeline
            .as_subscription()
            .map(|(_, now)| Message::Frame(now));

        let config_watcher = self.core.watch_config(Self::APP_ID).map(|u| {
            for err in u.errors {
                tracing::error!(?err, "Error watching config");
            }
            Message::ConfigChanged(u.config)
        });

        Subscription::batch([
            timeline,
            config_watcher,
            process::process_subscription().map(Message::ProcessUpdate),
            stats::stats_subscription().map(Message::StatsUpdate),
            api::api_subscription(self.config.poll_interval_minutes).map(Message::ApiUpdate),
        ])
    }

    fn update(&mut self, message: Self::Message) -> app::Task<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let Some(main_id) = self.core.main_window_id() else {
                        return Task::none();
                    };
                    self.timeline = Timeline::default();
                    let new_id = Id::unique();
                    self.popup = Some(new_id);
                    let popup_settings = self.core.applet.get_popup_settings(
                        main_id,
                        new_id,
                        Some((1, 1)),  // Required for complex popups to prevent Wayland crash
                        None,
                        None,
                    );
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::Frame(now) => self.timeline.now(now),
            Message::ProcessUpdate(update) => {
                self.process_count = update.count;
            }
            Message::StatsUpdate(update) => {
                self.today_messages = update.today_messages;
                self.today_sessions = update.today_sessions;
                self.cost_usd = update.total_cost_usd;
            }
            Message::ApiUpdate(update) => {
                debug!(
                    "ApiUpdate received: session={:.1}%, weekly={:.1}%, opus={:.1}%, sonnet={:.1}%",
                    update.session_usage_percent,
                    update.weekly_usage_percent,
                    update.opus_usage_percent,
                    update.sonnet_usage_percent
                );
                self.has_credentials = update.has_credentials;
                self.subscription_type = update.subscription_type;
                self.session_usage_percent = update.session_usage_percent;
                self.session_reset_time = update.session_reset_time;
                self.weekly_usage_percent = update.weekly_usage_percent;
                self.weekly_reset_time = update.weekly_reset_time;
                self.opus_usage_percent = update.opus_usage_percent;
                self.sonnet_usage_percent = update.sonnet_usage_percent;
                self.api_error = update.last_error;
            }
            Message::OpenTerminal => {
                let mut cmd = std::process::Command::new("cosmic-term");
                cmd.arg("-e").arg("claude");
                tokio::spawn(async {
                    if cosmic::process::spawn(cmd).await.is_none() {
                        tracing::error!("Failed to open terminal: cosmic-term process could not be spawned");
                    }
                });
            }
            Message::OpenSettings => {
                if let Some(home) = dirs::home_dir() {
                    let claude_dir = home.join(".claude");
                    let mut cmd = std::process::Command::new("cosmic-files");
                    cmd.arg(claude_dir);
                    tokio::spawn(async {
                        if cosmic::process::spawn(cmd).await.is_none() {
                            tracing::error!("Failed to open file manager: cosmic-files process could not be spawned");
                        }
                    });
                }
            }
            Message::ToggleSettings => {
                self.settings_expanded = !self.settings_expanded;
            }
            Message::ConfigChanged(mut config) => {
                config.validate();
                self.config = config;
            }
            Message::CycleIconDisplay => {
                self.config.icon_display = match self.config.icon_display {
                    IconDisplay::Session => IconDisplay::Weekly,
                    IconDisplay::Weekly => IconDisplay::Both,
                    IconDisplay::Both => IconDisplay::Session,
                };
                self.save_config();
            }
            Message::ToggleMascot(enabled) => {
                self.config.show_mascot = enabled;
                self.save_config();
            }
            Message::SetWarningThreshold(value) => {
                self.config.warning_threshold = value;
                self.save_config();
            }
            Message::SetCriticalThreshold(value) => {
                self.config.critical_threshold = value;
                self.save_config();
            }
            Message::TogglePercentageText(enabled) => {
                self.config.show_percentage_text = enabled;
                self.save_config();
            }
            Message::SetPollInterval(minutes) => {
                self.config.poll_interval_minutes = minutes;
                self.save_config();
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // Create custom colored indicator
        let indicator = self.create_usage_indicator();

        // Wrap in a button for click handling
        let indicator_button = button::custom(indicator)
            .padding(4)
            .class(cosmic::theme::Button::AppletIcon)
            .on_press(Message::TogglePopup);

        let content: Element<'_, Self::Message> = if self.config.show_percentage_text && self.has_credentials {
            let percent_text = match self.config.icon_display {
                IconDisplay::Session => format!("{:.0}%", self.session_usage_percent),
                IconDisplay::Weekly => format!("{:.0}%", self.weekly_usage_percent),
                IconDisplay::Both => format!("{:.0}%", self.session_usage_percent),
            };
            row![
                indicator_button,
                text::body(percent_text),
            ]
            .align_y(Alignment::Center)
            .spacing(4)
            .into()
        } else {
            indicator_button.into()
        };

        // Use autosize_window to properly size the panel button
        self.core.applet.autosize_window(content).into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let Spacing {
            space_xxs,
            space_s,
            ..
        } = theme::active().cosmic().spacing;

        // Header with subscription type
        let header = row![
            text::heading(fl!("claude-code")),
            horizontal_space(),
        ]
        .align_y(Alignment::Center)
        .padding([0, space_s]);

        let plan_text = if self.has_credentials {
            format!("{} {}", self.subscription_type, fl!("plan"))
        } else {
            fl!("not-logged-in")
        };

        let plan_section = padded_control(
            text::body(plan_text)
        );

        // 5-Hour Session Usage
        let session_section = padded_control(
            column![
                text::body(fl!("session-usage")),
                progress_bar(0.0..=100.0, self.session_usage_percent)
                    .width(Length::Fill),
                row![
                    text::caption(format!("{:.0}%", self.session_usage_percent)),
                    horizontal_space(),
                    text::caption(self.format_reset_time(self.session_reset_time)),
                ],
            ]
            .spacing(space_xxs)
        );

        // Weekly Usage
        let weekly_section = padded_control(
            column![
                text::body(fl!("weekly-usage")),
                progress_bar(0.0..=100.0, self.weekly_usage_percent)
                    .width(Length::Fill),
                row![
                    text::caption(format!("{:.0}%", self.weekly_usage_percent)),
                    horizontal_space(),
                    text::caption(self.format_reset_date(self.weekly_reset_time)),
                ],
            ]
            .spacing(space_xxs)
        );

        // Status section (process count)
        let status_text = if self.process_count > 0 {
            fl!("sessions-running", count = self.process_count)
        } else {
            fl!("no-sessions")
        };

        let status_section = padded_control(
            column![
                text::body(fl!("status")),
                text::caption(format!("● {}", status_text)),
            ]
            .spacing(space_xxs)
        );

        // Error display if any
        let error_section = self.api_error.as_ref().map(|error| padded_control(
            text::caption(format!("{}: {}", fl!("api-error"), error))
        ));

        // Settings section (collapsible)
        let settings_header = padded_control(
            mouse_area(
                row![
                    text::body(fl!("settings")),
                    horizontal_space(),
                    text::body(if self.settings_expanded { "▼" } else { "▶" }),
                ]
                .align_y(Alignment::Center)
            )
            .on_press(Message::ToggleSettings)
        );

        let icon_display_text = match self.config.icon_display {
            IconDisplay::Both => fl!("icon-display-both"),
            IconDisplay::Session => fl!("icon-display-session"),
            IconDisplay::Weekly => fl!("icon-display-weekly"),
        };

        let settings_content: Option<Element<'_, Message>> = if self.settings_expanded {
            Some(padded_control(
                column![
                    row![
                        text::caption(fl!("icon-display")),
                        horizontal_space(),
                        menu_button(text::caption(icon_display_text))
                            .on_press(Message::CycleIconDisplay),
                    ]
                    .align_y(Alignment::Center),
                    row![
                        text::caption(fl!("show-mascot")),
                        horizontal_space(),
                        toggler(self.config.show_mascot)
                            .on_toggle(Message::ToggleMascot),
                    ]
                    .align_y(Alignment::Center),
                    row![
                        text::caption(format!("{}: {}%", fl!("warning-threshold"), self.config.warning_threshold)),
                        horizontal_space(),
                        slider(0..=100, self.config.warning_threshold, Message::SetWarningThreshold)
                            .width(Length::Fixed(120.0)),
                    ]
                    .align_y(Alignment::Center),
                    row![
                        text::caption(format!("{}: {}%", fl!("critical-threshold"), self.config.critical_threshold)),
                        horizontal_space(),
                        slider(0..=100, self.config.critical_threshold, Message::SetCriticalThreshold)
                            .width(Length::Fixed(120.0)),
                    ]
                    .align_y(Alignment::Center),
                    row![
                        text::caption(fl!("show-percentage")),
                        horizontal_space(),
                        toggler(self.config.show_percentage_text)
                            .on_toggle(Message::TogglePercentageText),
                    ]
                    .align_y(Alignment::Center),
                    row![
                        text::caption(format!("{}: {} min", fl!("poll-interval"), self.config.poll_interval_minutes)),
                        horizontal_space(),
                        slider(5..=120, self.config.poll_interval_minutes.min(120) as u8, |v| Message::SetPollInterval(v as u32))
                            .width(Length::Fixed(120.0)),
                    ]
                    .align_y(Alignment::Center),
                ]
                .spacing(space_xxs)
            ).into())
        } else {
            None
        };

        // Action buttons
        let actions = column![
            menu_button(text::body(fl!("open-terminal")))
                .on_press(Message::OpenTerminal),
            menu_button(text::body(fl!("open-claude-dir")))
                .on_press(Message::OpenSettings),
        ];

        let mut content_list = column![
            header,
            plan_section,
            padded_control(divider::horizontal::default()).padding([space_xxs, space_s]),
            session_section,
            padded_control(divider::horizontal::default()).padding([space_xxs, space_s]),
            weekly_section,
            padded_control(divider::horizontal::default()).padding([space_xxs, space_s]),
            status_section,
        ]
        .padding([8, 0]);

        if let Some(error_widget) = error_section {
            content_list = content_list.push(error_widget);
        }

        content_list = content_list
            .push(padded_control(divider::horizontal::default()).padding([space_xxs, space_s]))
            .push(settings_header);

        if let Some(settings_widget) = settings_content {
            content_list = content_list.push(settings_widget);
        }

        content_list = content_list
            .push(padded_control(divider::horizontal::default()).padding([space_xxs, space_s]))
            .push(actions);

        self.core.applet.popup_container(content_list).into()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}

impl ClaudeApplet {
    fn format_reset_time(&self, reset_time: Option<DateTime<Utc>>) -> String {
        match reset_time {
            Some(time) => {
                let now = Utc::now();
                let duration = time.signed_duration_since(now);

                if duration.num_seconds() <= 0 {
                    fl!("resetting")
                } else {
                    let hours = duration.num_hours();
                    let minutes = duration.num_minutes() % 60;

                    if hours > 0 {
                        fl!("resets-in-hours", hours = hours, minutes = minutes)
                    } else {
                        fl!("resets-in-minutes", minutes = minutes)
                    }
                }
            }
            None => fl!("unknown"),
        }
    }

    fn format_reset_date(&self, reset_time: Option<DateTime<Utc>>) -> String {
        match reset_time {
            Some(time) => {
                fl!("resets-on", date = time.format("%b %d").to_string())
            }
            None => fl!("unknown"),
        }
    }

    /// Get usage level based on percentage and configured thresholds
    fn get_usage_level(&self, percent: f32) -> UsageLevel {
        if percent <= self.config.warning_threshold as f32 {
            UsageLevel::Low
        } else if percent <= self.config.critical_threshold as f32 {
            UsageLevel::Medium
        } else {
            UsageLevel::High
        }
    }

    /// Get color for a usage level
    fn get_level_color(&self, level: UsageLevel) -> Color {
        match level {
            UsageLevel::Low => COLOR_LOW,
            UsageLevel::Medium => COLOR_MEDIUM,
            UsageLevel::High => COLOR_HIGH,
        }
    }

    /// Generate SVG markup for a circular progress ring
    fn generate_progress_svg(percent: f32, color: Color, label: &str) -> String {
        let progress = (percent / 100.0).clamp(0.0, 1.0);
        let radius = 10.0;
        let circumference = 2.0 * PI * radius;
        let dash_offset = circumference * (1.0 - progress);

        // Convert Color to hex string
        let color_hex = format!(
            "#{:02x}{:02x}{:02x}",
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8
        );

        let track_color = "#4d4d4d";

        format!(
            r##"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <circle cx="12" cy="12" r="{radius}" fill="none" stroke="{track_color}" stroke-width="3"/>
                <circle cx="12" cy="12" r="{radius}" fill="none" stroke="{color_hex}" stroke-width="3"
                    stroke-dasharray="{circumference}" stroke-dashoffset="{dash_offset}"
                    stroke-linecap="round" transform="rotate(-90 12 12)"/>
                <text x="12" y="16" text-anchor="middle" fill="white" font-size="10">{label}</text>
            </svg>"##
        )
    }

    /// Generate SVG markup for the Claude mascot with color based on usage level
    /// Pixel-perfect match to the ASCII art:
    ///    ▐▛███▜▌
    ///   ▝▜█████▛▘
    ///     ▘▘ ▝▝
    fn generate_mascot_svg(color: Color) -> String {
        let color_hex = format!(
            "#{:02x}{:02x}{:02x}",
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8
        );

        // Using 9x8 grid for proper proportions
        format!(
            r##"<svg viewBox="0 0 9 8" xmlns="http://www.w3.org/2000/svg">
                <!-- Left ear -->
                <rect x="1" y="0" width="2" height="3" fill="{color_hex}"/>
                <!-- Right ear -->
                <rect x="6" y="0" width="2" height="3" fill="{color_hex}"/>
                <!-- Head/body connecting ears -->
                <rect x="3" y="1" width="3" height="2" fill="{color_hex}"/>
                <!-- Main body (wider) -->
                <rect x="0" y="3" width="9" height="3" fill="{color_hex}"/>
                <!-- Left foot -->
                <rect x="1" y="6" width="2" height="2" fill="{color_hex}"/>
                <!-- Right foot -->
                <rect x="6" y="6" width="2" height="2" fill="{color_hex}"/>
            </svg>"##
        )
    }

    /// Create a session progress ring using SVG (with caching)
    fn create_session_ring(&self, percent: f32, color: Color) -> Element<'_, Message> {
        let svg_data = {
            let mut cache = self.svg_cache.borrow_mut();
            // Check if cached value is still valid
            if cache.session_svg.is_some()
                && (cache.session_percent - percent).abs() < 0.1
                && cache.session_color == Some(color)
            {
                cache.session_svg.clone().unwrap()
            } else {
                // Generate new SVG and cache it
                let svg = Self::generate_progress_svg(percent, color, "S");
                cache.session_percent = percent;
                cache.session_color = Some(color);
                cache.session_svg = Some(svg);
                cache.session_svg.clone().unwrap()
            }
        };
        let handle = svg::Handle::from_memory(svg_data.into_bytes());
        cosmic::iced_widget::Svg::new(handle)
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .into()
    }

    /// Create a weekly progress ring using SVG (with caching)
    fn create_weekly_ring(&self, percent: f32, color: Color) -> Element<'_, Message> {
        let svg_data = {
            let mut cache = self.svg_cache.borrow_mut();
            // Check if cached value is still valid
            if cache.weekly_svg.is_some()
                && (cache.weekly_percent - percent).abs() < 0.1
                && cache.weekly_color == Some(color)
            {
                cache.weekly_svg.clone().unwrap()
            } else {
                // Generate new SVG and cache it
                let svg = Self::generate_progress_svg(percent, color, "W");
                cache.weekly_percent = percent;
                cache.weekly_color = Some(color);
                cache.weekly_svg = Some(svg);
                cache.weekly_svg.clone().unwrap()
            }
        };
        let handle = svg::Handle::from_memory(svg_data.into_bytes());
        cosmic::iced_widget::Svg::new(handle)
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .into()
    }

    /// Create the Claude mascot icon using SVG (with caching)
    fn create_mascot(&self, color: Color) -> Element<'_, Message> {
        let svg_data = {
            let mut cache = self.svg_cache.borrow_mut();
            // Check if cached value is still valid
            if cache.mascot_svg.is_some() && cache.mascot_color == Some(color) {
                cache.mascot_svg.clone().unwrap()
            } else {
                // Generate new SVG and cache it
                let svg = Self::generate_mascot_svg(color);
                cache.mascot_color = Some(color);
                cache.mascot_svg = Some(svg);
                cache.mascot_svg.clone().unwrap()
            }
        };
        let handle = svg::Handle::from_memory(svg_data.into_bytes());
        cosmic::iced_widget::Svg::new(handle)
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .into()
    }

    /// Create the visual usage indicator widget - SVG circular progress rings with optional mascot
    fn create_usage_indicator(&self) -> Element<'_, Message> {
        let spacing = 4.0;

        debug!(
            "create_usage_indicator: session={:.1}%, weekly={:.1}%, has_creds={}, show_mascot={}",
            self.session_usage_percent, self.weekly_usage_percent, self.has_credentials, self.config.show_mascot
        );

        if !self.has_credentials {
            // Inactive state - show appropriate icon in gray
            let rings: Element<'_, Message> = match self.config.icon_display {
                IconDisplay::Session => self.create_session_ring(0.0, COLOR_INACTIVE),
                IconDisplay::Weekly => self.create_weekly_ring(0.0, COLOR_INACTIVE),
                IconDisplay::Both => row![
                    self.create_session_ring(0.0, COLOR_INACTIVE),
                    self.create_weekly_ring(0.0, COLOR_INACTIVE),
                ]
                .spacing(spacing)
                .align_y(Alignment::Center)
                .into(),
            };

            return if self.config.show_mascot {
                row![
                    self.create_mascot(COLOR_INACTIVE),
                    rings,
                ]
                .spacing(spacing)
                .align_y(Alignment::Center)
                .into()
            } else {
                rings
            };
        }

        let session_color = self.get_level_color(self.get_usage_level(self.session_usage_percent));
        let weekly_color = self.get_level_color(self.get_usage_level(self.weekly_usage_percent));

        let rings: Element<'_, Message> = match self.config.icon_display {
            IconDisplay::Session => {
                self.create_session_ring(self.session_usage_percent, session_color)
            }
            IconDisplay::Weekly => {
                self.create_weekly_ring(self.weekly_usage_percent, weekly_color)
            }
            IconDisplay::Both => {
                row![
                    self.create_session_ring(self.session_usage_percent, session_color),
                    self.create_weekly_ring(self.weekly_usage_percent, weekly_color),
                ]
                .spacing(spacing)
                .align_y(Alignment::Center)
                .into()
            }
        };

        if self.config.show_mascot {
            row![
                self.create_mascot(COLOR_CLAUDE),
                rings,
            ]
            .spacing(spacing)
            .align_y(Alignment::Center)
            .into()
        } else {
            rings
        }
    }

    /// Save current config to cosmic-config
    fn save_config(&self) {
        if let Ok(config_helper) =
            cosmic::cosmic_config::Config::new(config::APP_ID, ClaudeAppletConfig::VERSION)
        {
            if let Err(err) = self.config.write_entry(&config_helper) {
                tracing::error!(?err, "Error writing config");
            }
        }
    }
}
