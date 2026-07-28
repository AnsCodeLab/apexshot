---
name: Running clippy on ApexShot
description: Clippy notes for the current portable Cargo config
---
Use the active Rust toolchain from `PATH`. Do not add `/nix/store/...` `build.rustc`
overrides to `.cargo/config.toml`; Replit changes those hashes and stale paths break
Cargo before clippy can run.

**How to apply:**

```
cargo clippy
```
