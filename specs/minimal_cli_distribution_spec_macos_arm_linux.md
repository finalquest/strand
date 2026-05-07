# Minimal CLI Distribution Spec

## Goal

Distribute the internal Rust CLI tool using:

- precompiled binaries
- Pages hosting
- a simple install.sh bootstrap script

The initial scope intentionally stays minimal.

---

# Supported Platforms

Initial supported targets:

```text
macos-arm64
linux-x86_64
```

Unsupported platforms:

```text
macos-x86_64
windows
```

---

# Build Commands

## macOS ARM64

```bash
cargo build --release
```

Generated binary:

```text
target/release/mytool
```

---

## Linux x86_64

Uses cross-rs.

Install:

```bash
cargo install cross
```

Build:

```bash
cross build --release --target x86_64-unknown-linux-gnu
```

Generated binary:

```text
target/x86_64-unknown-linux-gnu/release/mytool
```

---

# Release Structure

```text
releases/
  v1.0.0/
    mytool-macos-arm64
    mytool-linux-x86_64
```

---

# latest.json

```json
{
  "version": "1.0.0"
}
```

---

# Installer Command

```bash
curl -fsSL https://tools.company.dev/install.sh | bash
```

---

# Installer Responsibilities

The installer script must:

1. Detect operating system
2. Detect CPU architecture
3. Resolve latest version
4. Download the correct binary
5. Install into ~/.local/bin
6. Mark the binary as executable

---

# Platform Resolution

## macOS ARM64

```text
Darwin + arm64
→ mytool-macos-arm64
```

---

## Linux x86_64

```text
Linux + x86_64
→ mytool-linux-x86_64
```

---

# Unsupported Platforms

The installer should explicitly reject unsupported platforms.

Example:

```bash
echo "Intel macOS is not supported"
exit 1
```

---

# Installation Path

```text
~/.local/bin/mytool
```

---

# Update Strategy

Updates are performed by rerunning the installer:

```bash
curl -fsSL https://tools.company.dev/install.sh | bash
```

---

# Non-Goals

This specification does not define:

- Homebrew
- apt/dnf packages
- Windows support
- Intel macOS support
- automatic updates
- package managers
- telemetry
- binary signing

