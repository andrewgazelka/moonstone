# Moonstone Development Notes

## pf (Packet Filter) Rules

macOS pf requires strict ordering in `/etc/pf.conf`:

1. Options
2. Normalization (scrub)
3. Queueing (dummynet)
4. Translation (nat/rdr)
5. Filtering (anchor/pass/block)

The moonstone anchor must be added in the **filtering** section, not before scrub-anchor lines:

```
# Normalization
scrub-anchor "com.apple/*"

# NAT/RDR
nat-anchor "com.apple/*"
rdr-anchor "com.apple/*"

# Queueing
dummynet-anchor "com.apple/*"

# Filtering - moonstone first for blocking priority
anchor "com.moonstone"
load anchor "com.moonstone" from "/etc/pf.anchors/com.moonstone"
anchor "com.apple/*"
load anchor "com.apple" from "/etc/pf.anchors/com.apple"
```

## Bundle IDs

Bundle IDs are **case-sensitive** on macOS. Use `defaults read` to get the correct ID:

```bash
defaults read "/Applications/AppName.app/Contents/Info" CFBundleIdentifier
```

Known quirks:
- FaceTime: `com.apple.FaceTime` (capital F and T)
- OrbStack: `dev.kdrag0n.MacVirt` (not dev.orbstack.OrbStack)
- Beeper: `com.automattic.beeper.desktop` (not com.beeper.beeper-desktop)
- AeroSpace: `bobko.aerospace`
- Arc: `company.thebrowser.Browser`

## VPN Bypass

Cloudflare WARP and other VPNs tunnel traffic, bypassing local pf firewall rules. Network blocking won't work if a VPN is active.

## Daemon Detection

The daemon runs as root. Use `lsappinfo` instead of AppleScript for app detection - AppleScript requires Accessibility permissions which root doesn't have.
