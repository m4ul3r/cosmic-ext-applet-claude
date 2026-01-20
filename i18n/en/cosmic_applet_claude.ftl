claude-code = Claude Code
plan = Plan
not-logged-in = Not logged in
status = Status
sessions-running = { $count ->
    [one] 1 session running
    *[other] { $count } sessions running
}
no-sessions = No sessions

# Usage sections
session-usage = 5-Hour Session
weekly-usage = Weekly Usage

# Reset times
resets-in-hours = Resets in { $hours }h { $minutes }m
resets-in-minutes = Resets in { $minutes }m
resets-on = Resets { $date }
resetting = Resetting...
unknown = Unknown

# Stats
today = Today
messages = { $count ->
    [one] 1 message
    *[other] { $count } messages
}
sessions = { $count ->
    [one] 1 session
    *[other] { $count } sessions
}
cost = Cost

# Actions
open-terminal = Open Claude Terminal
open-claude-dir = Open Claude Directory

# Settings
settings = Settings
settings-section = Settings
icon-display = Icon Display
icon-display-session = Session Only
icon-display-weekly = Weekly Only
icon-display-both = Both (Dual Rings)
show-mascot = Show Claude Mascot
warning-threshold = Warning
critical-threshold = Critical
show-percentage = Show Percentage
poll-interval = Poll Interval

# Errors
api-error = API Error
