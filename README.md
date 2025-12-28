# Moonstone

Modular Apple MDM server and Focus enforcement agent.

## Architecture

```
moonstone/
├── crates/
│   ├── mdm/              # Generic MDM (reusable)
│   │   ├── core/         # Core types & protocols
│   │   ├── storage/      # Diesel-based storage (SQLite/PostgreSQL)
│   │   ├── crypto/       # Certificate & signature utilities
│   │   ├── service/      # Service layer & DDM
│   │   ├── push/         # APNs push notifications
│   │   └── http/         # HTTP handlers & API
│   │
│   └── focus/            # Focus-specific
│       ├── agent/        # macOS enforcement daemon
│       └── server/       # MDM server with focus policies
```

## Crates

| Crate | Description |
|-------|-------------|
| `mdm-core` | Core MDM types, check-in messages, commands |
| `mdm-storage` | Diesel ORM storage with SQLite/PostgreSQL support |
| `mdm-crypto` | Certificate parsing, PKCS7 signature verification |
| `mdm-service` | Business logic, NanoMdm service implementation |
| `mdm-push` | APNs push notification delivery |
| `mdm-http` | Axum HTTP handlers and REST API |
| `focus-agent` | macOS daemon for app/website blocking |
| `focus-server` | MDM server with focus policy management |

## Features

- **MDM-Driven**: All policies pushed from server, no local config
- **App Blocking**: Instant SIGKILL of blocked apps
- **Website Blocking**: IP-level blocking via pf firewall
- **Allowlist/Blocklist**: Flexible policy modes
- **Scheduling**: Time-based blocking periods

## Building

```bash
cargo build --release
```

## License

MIT

## Acknowledgments

Architecture inspired by [nanomdm](https://github.com/micromdm/nanomdm) by MicroMDM - a minimalist, composable MDM server written in Go.
