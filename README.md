# cosmic-applet-claude

A COSMIC desktop applet for monitoring your Claude Code usage.

---

> **WARNING: This project was entirely vibe-coded with Claude.**
>
> No humans were harmed in the making of this applet, but no humans really reviewed the code either. Use at your own risk. If it breaks, you get to keep both pieces.

---

## Features

- **Session Usage Tracking**: Monitor your 5-hour session usage with a visual progress ring
- **Weekly Usage Tracking**: Keep an eye on your weekly usage limits
- **Process Monitoring**: See how many Claude sessions are currently running
- **Configurable Thresholds**: Set your own warning (yellow) and critical (red) levels
- **Multiple Display Modes**: Show session, weekly, or both usage indicators
- **Optional Mascot**: Toggle the Claude mascot icon on/off
- **Quick Actions**: Launch Claude in terminal or open the `.claude` directory

## Installation

### Dependencies

- COSMIC Desktop Environment
- Rust toolchain
- [just](https://github.com/casey/just) command runner

### Building & Installing

```bash
# Build release binary
just build-release

# Install to system (requires sudo)
sudo just install
```

### Uninstalling

```bash
sudo just uninstall
```

### Development

```bash
# Build debug binary
just build-debug

# Clean build artifacts
just clean
```

After installation, add the applet to your COSMIC panel through the panel settings.

## Configuration

The applet can be configured through the popup settings panel:

- **Icon Display**: Choose to show Session, Weekly, or Both usage rings
- **Show Mascot**: Toggle the Claude mascot icon
- **Warning Threshold**: Set the percentage at which the indicator turns yellow (default: 50%)
- **Critical Threshold**: Set the percentage at which the indicator turns red (default: 80%)
- **Show Percentage**: Display the usage percentage as text next to the icon
- **Poll Interval**: How often to check the API for usage updates (5-120 minutes)

## How It Works

The applet reads your Claude credentials from `~/.claude/` and queries the API to get your current usage statistics. It displays this information as color-coded circular progress rings in your panel.

### Colors

- **Green**: Usage below warning threshold
- **Yellow**: Usage between warning and critical thresholds
- **Red**: Usage above critical threshold
- **Gray**: Not logged in or no credentials found

## License

GPL-3.0-only
