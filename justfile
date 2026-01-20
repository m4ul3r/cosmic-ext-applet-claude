# Variables matching parent justfile conventions
rootdir := ''
prefix := '/usr'
bindir := rootdir + prefix + '/bin'
sharedir := rootdir + prefix + '/share'
iconsdir := sharedir + '/icons/hicolor'

# Binary name
name := 'cosmic-applet-claude'
appid := 'dev.m4ul3r.CosmicExtAppletClaude'

# Default target
default: build-release

# Build in release mode
build-release:
    cargo build --release

# Build in debug mode
build-debug:
    cargo build

# Install the applet
install:
    # Install binary
    install -Dm0755 target/release/{{name}} {{bindir}}/{{name}}
    # Install desktop file
    install -Dm0644 data/{{appid}}.desktop {{sharedir}}/applications/{{appid}}.desktop
    # Install icons
    for icon in data/icons/scalable/apps/*.svg; do \
        install -Dm0644 "$icon" {{iconsdir}}/scalable/apps/$(basename "$icon"); \
    done

# Uninstall the applet
uninstall:
    rm -f {{bindir}}/{{name}}
    rm -f {{sharedir}}/applications/{{appid}}.desktop
    rm -f {{iconsdir}}/scalable/apps/{{appid}}*.svg

# Clean build artifacts
clean:
    cargo clean
