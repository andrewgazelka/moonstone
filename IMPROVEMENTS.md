# Moonstone Improvement Analysis

This document analyzes potential improvements to moonstone, categorized by the level of system access required.

---

## 🔴 Requires SIP Disable (Most Hardcore)

These improvements require disabling System Integrity Protection, making them extremely difficult to bypass.

### 1. Kernel Extension (KEXT) for Process Blocking

**Current**: Uses `lsappinfo` polling at 100ms intervals + SIGKILL
**Improvement**: Kernel extension that intercepts `execve()` syscall

```
Bypass difficulty: Recovery mode + SIP disable + KEXT removal
```

**Benefits**:
- Block apps BEFORE they launch (zero flicker)
- Cannot be bypassed by renaming binaries
- Works even if daemon is killed (kernel persists)

**Implementation**:
- Use `MAC` (Mandatory Access Control) framework
- Hook `proc_check_run_cs_invalid` or use Endpoint Security (requires SIP partial disable)

### 2. Network Extension at Kernel Level

**Current**: pf (packet filter) rules at IP level
**Improvement**: Network Kernel Extension (NKE) for deep packet inspection

```
Bypass difficulty: Recovery mode + SIP disable
```

**Benefits**:
- Block by hostname directly (no DNS resolution games)
- Block even if VPN is active (works at socket level)
- Block specific protocols/content types
- Cannot be bypassed by editing `/etc/hosts`

### 3. Protected System Daemon

**Current**: LaunchDaemon in `/Library/LaunchDaemons`
**Improvement**: Install daemon in SIP-protected location

```
Location: /System/Library/LaunchDaemons/com.moonstone.daemon.plist
Binary:   /System/Library/CoreServices/moonstone-daemon
```

**Benefits**:
- Cannot delete/modify daemon without SIP disable
- Survives even `sudo rm -rf /Library`
- Trusted by system as core service

### 4. Immutable Binary Protection

**Current**: Binary in `/usr/local/bin` (user-writable with sudo)
**Improvement**: Code-sign and add to AMFI trust cache

```
Bypass difficulty: SIP disable + Recovery mode
```

**Benefits**:
- macOS refuses to run modified binaries
- Cannot be replaced with fake moonstone

---

## 🟠 Requires Root / Recovery Mode

These improvements require root but not SIP disable.

### 5. Firmware-Level Persistence

**Current**: LaunchDaemon restarts on reboot
**Improvement**: NVRAM variable that triggers startup item

```bash
sudo nvram moonstone-enforce=true
```

**Benefits**:
- Survives even if `/Library` is wiped
- Checked before user processes start

**Limitation**: Can be cleared from Recovery mode with `nvram -d`

### 6. FileVault Integration

**Improvement**: Tie moonstone to FileVault unlock

```
On tamper: Lock FileVault, require recovery key
```

**Benefits**:
- Most extreme tamper response possible
- Requires recovery key (which you can hide from yourself)

### 7. Login Hook Protection

**Current**: Apps killed after they start
**Improvement**: Block at login with authorization plugin

```
Location: /Library/Security/SecurityAgentPlugins/moonstone.bundle
```

**Benefits**:
- Runs before ANY user code
- Can block launch of specific users entirely during block periods

### 8. Configuration in Protected Location

**Current**: `/etc/moonstone/config.toml` with `schg` flag
**Improvement**: Store encrypted config in System Keychain

```
Location: /Library/Keychains/System.keychain
Item: com.moonstone.config
```

**Benefits**:
- Protected by keychain ACLs
- Requires authentication to modify
- Cannot be edited by mounting disk from recovery

### 9. DNS Sinkhole

**Current**: Block by IP after DNS resolution
**Improvement**: Run local DNS resolver that sinkholes blocked domains

```
1. Install moonstone-dns on 127.0.0.1:53
2. Configure system to use 127.0.0.1 as DNS
3. Return 0.0.0.0 for blocked domains
```

**Benefits**:
- Works regardless of IP changes
- Can block wildcard domains (`*.twitter.com`)
- Faster than pf rules

---

## 🟡 Requires Root (Standard Improvements)

### 10. Blocklist Mode for Network ⭐ (Requested)

**Current**: Only allowlist mode implemented in `network.rs`
**Improvement**: Add blocklist mode

```toml
[websites]
mode = "blocklist"
blocked = [
  "twitter.com",
  "reddit.com",
  "youtube.com",
  "instagram.com",
  "tiktok.com",
  "facebook.com",
  "news.ycombinator.com",
]
```

**Implementation**:
```rust
// In network.rs generate_rules()
match mode {
    BlockMode::Allowlist => {
        // Current: pass allowed, block rest
    }
    BlockMode::Blocklist => {
        // New: block specific IPs, pass rest
        for ip in blocked_ips {
            rules.push_str(&format!(
                "block drop out quick proto {{ tcp, udp }} to {}\n", ip
            ));
        }
        rules.push_str("pass out quick all\n");
    }
}
```

**Benefits**:
- Much easier to configure for casual use
- Only blocks distracting sites, allows everything else
- No need to maintain exhaustive allowlist

### 11. App Signature Verification

**Current**: Checks bundle ID only
**Improvement**: Verify code signature matches expected

```rust
// Verify app hasn't been tampered with
let signature = codesign::verify(&app_path)?;
if signature.team_id != expected_team_id {
    kill_app();
}
```

**Benefits**:
- Cannot bypass by creating fake app with allowed bundle ID

### 12. Browser Extension Blocking

**Current**: Can only block entire browser
**Improvement**: Detect and block specific browser extensions

```rust
// Check browser extension directories
// ~/Library/Application Support/Google/Chrome/Default/Extensions/
// Block if distracting extension detected
```

### 13. Screen Recording Detection

**Improvement**: Detect if user is viewing blocked content via:
- Screen recording/sharing
- AirPlay mirroring to view blocked device
- Universal Clipboard from blocked device

### 14. Multi-User Coordination

**Current**: Per-user watchdog
**Improvement**: System-wide enforcement that blocks creating new users

```
- Block System Preferences > Users & Groups during block period
- Prevent `sysadminctl` user creation
```

---

## 🟢 No Special Permissions Needed

### 15. Time Server Verification

**Current**: Trusts system clock
**Improvement**: Verify time via HTTPS to trusted servers

```rust
// Fetch time from multiple sources
let times = [
    fetch_time("https://worldtimeapi.org/api/ip"),
    fetch_time("https://timeapi.io/api/Time/current/zone"),
];
// If system clock differs by >5min, assume tampering
```

**Benefits**:
- Prevents bypass by changing system time

### 16. Hardware Token Integration

**Improvement**: Require YubiKey for emergency disable

```rust
// Instead of typing challenge:
if !yubikey::verify_otp(user_input) {
    return Err("Invalid YubiKey OTP");
}
```

**Benefits**:
- Can physically lock YubiKey away from yourself

### 17. Social Accountability

**Improvement**: Notify accountability partner on:
- Emergency disable attempt
- Tamper detection
- Schedule modification

```rust
// Send webhook/email to partner
notify_partner(&config.accountability_email, "Emergency disable activated");
```

### 18. Graduated Blocking

**Current**: Binary allow/block
**Improvement**: Time-limited access

```toml
[apps.graduated]
"com.twitter.twitter" = { daily_limit = "30m" }
"com.apple.Safari" = { hourly_limit = "15m" }
```

### 19. Content-Aware Blocking

**Improvement**: Allow site but block specific content

```toml
[websites.content_rules]
"youtube.com" = { allow = ["/results", "/@productivity"], block = ["shorts", "gaming"] }
```

---

## Implementation Priority

| Priority | Improvement | Effort | Impact |
|----------|-------------|--------|--------|
| 1 | ⭐ Blocklist mode | Low | High |
| 2 | Time verification | Low | Medium |
| 3 | DNS sinkhole | Medium | High |
| 4 | App signature verification | Medium | Medium |
| 5 | Kernel process blocking | High | Very High |
| 6 | Network kernel extension | High | Very High |

---

## Bypass Difficulty Matrix

| Attack Vector | Current | With SIP Improvements |
|--------------|---------|----------------------|
| Kill daemon | Sleep/shutdown | Kernel blocks |
| Edit config | Recovery mode | Keychain protected |
| Change time | Not protected | HTTPS verified |
| Rename app | Works | Signature check fails |
| Use VPN | Bypasses pf | NKE blocks |
| Create new user | Works | Blocked |
| Boot recovery | 10min bypass | SIP disable required |

---

## Quick Wins to Implement Now

1. **Blocklist mode for websites** - Flip the logic in `network.rs`
2. **Config field renaming** - Use `blocked` instead of `allowed` when mode is blocklist
3. **CIDR support for blocklist** - Block common CDN ranges for social media
4. **Time verification** - Simple HTTPS check at startup
