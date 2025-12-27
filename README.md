# Moonstone

Hardcore macOS focus blocker. Blocks apps and websites with extreme prejudice.

## Philosophy

Make bypassing so annoying that you'll just wait out the timer.

## Features

- **App Blocking**: Instant SIGKILL of blocked apps (100ms polling)
- **Website Blocking**: IP-level blocking via pf firewall (bypasses DNS tricks)
- **Dual-Process Watchdog**: If you kill one process, the other sleeps your machine
- **Config Locking**: `chflags schg` prevents editing without recovery mode
- **Emergency Disable**: 5-minute typing challenge to temporarily disable

## Installation

```bash
git clone https://github.com/andrewgazelka/moonstone
cd moonstone
sudo ./install.sh
```

## Configuration

Edit `~/.config/moonstone/config.toml`:

```toml
[schedule]
blocks = [
  { start = "04:00", end = "17:00" },
  { start = "17:10", end = "03:59" },
]

[apps]
mode = "allowlist"
allowed = [
  "com.apple.facetime",
  "com.mitchellh.ghostty",
  "com.apple.Terminal",
  "com.apple.finder",
]

[websites]
mode = "allowlist"
allowed = ["github.com", "docs.rs", "localhost"]

[hardcore]
on_tamper = "sleep"
emergency_disable_challenge = 300
lock_config = true
```

## Commands

```bash
moonstone status           # Show current status
moonstone config           # Show configuration
moonstone is-blocked       # Check if currently blocked
moonstone emergency-disable # Disable (requires 5-min challenge)
```

## Bypass Difficulty

| Method | Result |
|--------|--------|
| Kill daemon | Watchdog sleeps machine |
| Kill watchdog | Daemon restarts it |
| Kill both | Need kernel-level timing |
| `launchctl bootout` | Restarts in 1 second |
| Edit config | Locked with schg |
| Boot to recovery | Works (10+ min process) |

## Uninstall

```bash
sudo ./uninstall.sh
```

## License

MIT
