claude-code = Claude Code
plan = Planera
not-logged-in = Inte inloggad
status = Status
sessions-running = { $count ->
    [one] 1 session körs
    *[other] { $count } sessioner körs
}
no-sessions = Inga sessioner

# Användningsavsnitt
session-usage = 5-timmar Session
weekly-usage = Veckovis användning

# Återställnings tider
resets-in-hours = Återställer om { $hours }h { $minutes }m
resets-in-minutes = Återställer om { $minutes }m
resets-on = Återställer på { $date }
resetting = Återställer...
unknown = Okänd

# Statistik
today = Idag
messages = { $count ->
    [one] 1 meddelande
    *[other] { $count } meddelanden
}
sessions = { $count ->
    [one] 1 session
    *[other] { $count } sessioner
}
cost = Kostnad

# Åtgärder
open-terminal = Öppna Claude terminal
open-claude-dir = Öppna Claude katalog

# Inställningar
settings = inställningar
settings-section = inställningar
icon-display = Ikonvisning
icon-display-session = Session endast
icon-display-weekly = Veckovis endast
icon-display-both = Båda (dubbla ringar)
show-mascot = Visa Claude maskot
warning-threshold = Varning
critical-threshold = Kritisk
show-percentage = Visa procent
poll-interval = Avfråga intervall

# Fel
api-error = API-Fel
