#!/bin/bash
set -e

# Moonstone Installer
# Must be run with sudo

if [ "$EUID" -ne 0 ]; then
    echo "Please run with sudo: sudo ./install.sh"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACTUAL_USER="${SUDO_USER:-$USER}"
ACTUAL_HOME=$(eval echo "~$ACTUAL_USER")

echo "=== Moonstone Installer ==="
echo ""

# Build release binaries
echo "[1/7] Building release binaries..."
cd "$SCRIPT_DIR"
cargo build --release

# Create directories
echo "[2/7] Creating directories..."
mkdir -p /usr/local/bin
mkdir -p /var/log/moonstone
mkdir -p "$ACTUAL_HOME/.config/moonstone"
chown "$ACTUAL_USER" "$ACTUAL_HOME/.config/moonstone"

# Install binaries
echo "[3/7] Installing binaries..."
cp target/release/moonstone-daemon /usr/local/bin/
cp target/release/moonstone-watchdog /usr/local/bin/
cp target/release/moonstone /usr/local/bin/
chmod +x /usr/local/bin/moonstone-daemon
chmod +x /usr/local/bin/moonstone-watchdog
chmod +x /usr/local/bin/moonstone

# Install default config if not exists
echo "[4/7] Setting up configuration..."
mkdir -p /etc/moonstone
CONFIG_PATH="/etc/moonstone/config.toml"
if [ ! -f "$CONFIG_PATH" ]; then
    cat > "$CONFIG_PATH" << 'EOF'
# Moonstone Configuration
# https://github.com/andrewgazelka/moonstone

[schedule]
# Block periods (24-hour format)
# Block from 4am-5pm and 5:10pm-3:59am (only 10 min break at 5pm)
blocks = [
  { start = "04:00", end = "17:00" },
  { start = "17:10", end = "03:59" },
]

[apps]
# "allowlist" = block everything except listed apps
# "blocklist" = allow everything except listed apps
mode = "allowlist"

# Apps allowed during block periods (bundle IDs)
allowed = [
  "com.apple.facetime",
  "com.mitchellh.ghostty",
  "net.sourceforge.skim-app.skim",
  "com.beeper.beeper-desktop",
  "com.apple.Music",
  "dev.orbstack.OrbStack",
  "com.flexibits.fantastical2.mac",
  "com.apple.Terminal",
  "com.apple.finder",
  "com.apple.systempreferences",
]

[websites]
mode = "allowlist"
allowed = ["github.com", "docs.rs", "crates.io", "localhost"]

[hardcore]
# What to do if tampering detected: "sleep", "shutdown", or "lock"
on_tamper = "sleep"

# Seconds of continuous typing required to emergency disable
emergency_disable_challenge = 300

# Lock config file with chflags (requires recovery mode to edit)
lock_config = true

# "instant" = no warning, "notify" = brief notification before kill
kill_behavior = "instant"
EOF
    chmod 644 "$CONFIG_PATH"
    echo "   Created default config at $CONFIG_PATH"
else
    echo "   Config already exists, skipping"
fi

# Install LaunchDaemon (runs as root)
echo "[5/7] Installing LaunchDaemon..."
cp "$SCRIPT_DIR/launchd/com.moonstone.daemon.plist" /Library/LaunchDaemons/
chmod 644 /Library/LaunchDaemons/com.moonstone.daemon.plist
chown root:wheel /Library/LaunchDaemons/com.moonstone.daemon.plist

# Install LaunchAgent (runs as user)
echo "[6/7] Installing LaunchAgent..."
LAUNCH_AGENTS_DIR="$ACTUAL_HOME/Library/LaunchAgents"
mkdir -p "$LAUNCH_AGENTS_DIR"
cp "$SCRIPT_DIR/launchd/com.moonstone.watchdog.plist" "$LAUNCH_AGENTS_DIR/"
chown "$ACTUAL_USER" "$LAUNCH_AGENTS_DIR/com.moonstone.watchdog.plist"

# Load services
echo "[7/7] Loading services..."
launchctl load /Library/LaunchDaemons/com.moonstone.daemon.plist 2>/dev/null || true
sudo -u "$ACTUAL_USER" launchctl load "$LAUNCH_AGENTS_DIR/com.moonstone.watchdog.plist" 2>/dev/null || true

echo ""
echo "=== Installation Complete ==="
echo ""
echo "Moonstone is now running!"
echo ""
echo "Commands:"
echo "  moonstone status           - Show current status"
echo "  moonstone config           - Show configuration"
echo "  moonstone is-blocked       - Check if currently blocked"
echo "  moonstone emergency-disable - Disable (requires challenge)"
echo ""
echo "Config file: $CONFIG_PATH"
echo ""
echo "To lock config (prevents editing without recovery mode):"
echo "  sudo chflags schg $CONFIG_PATH"
echo ""
