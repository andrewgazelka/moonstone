#!/bin/bash
set -e

# Moonstone Uninstaller
# Must be run with sudo

if [ "$EUID" -ne 0 ]; then
    echo "Please run with sudo: sudo ./uninstall.sh"
    exit 1
fi

ACTUAL_USER="${SUDO_USER:-$USER}"
ACTUAL_HOME=$(eval echo "~$ACTUAL_USER")

echo "=== Moonstone Uninstaller ==="
echo ""

# Confirmation
read -p "Are you sure you want to uninstall Moonstone? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

# Unlock config if locked
CONFIG_PATH="$ACTUAL_HOME/.config/moonstone/config.toml"
if [ -f "$CONFIG_PATH" ]; then
    echo "[1/5] Unlocking config file..."
    chflags noschg "$CONFIG_PATH" 2>/dev/null || true
fi

# Unload services
echo "[2/5] Stopping services..."
launchctl bootout system/com.moonstone.daemon 2>/dev/null || true
sudo -u "$ACTUAL_USER" launchctl bootout "gui/$(id -u $ACTUAL_USER)/com.moonstone.watchdog" 2>/dev/null || true

# Remove LaunchDaemon
echo "[3/5] Removing LaunchDaemon..."
rm -f /Library/LaunchDaemons/com.moonstone.daemon.plist

# Remove LaunchAgent
echo "[4/5] Removing LaunchAgent..."
rm -f "$ACTUAL_HOME/Library/LaunchAgents/com.moonstone.watchdog.plist"

# Remove binaries
echo "[5/5] Removing binaries..."
rm -f /usr/local/bin/moonstone-daemon
rm -f /usr/local/bin/moonstone-watchdog
rm -f /usr/local/bin/moonstone

# Remove socket
rm -f /tmp/moonstone.sock

echo ""
echo "=== Uninstall Complete ==="
echo ""
echo "Note: Config file preserved at $CONFIG_PATH"
echo "To remove: rm -rf $ACTUAL_HOME/.config/moonstone"
echo ""
