---
name: ApexShot build quirks
description: Keep Cargo config portable across Replit/Nix and local machines
---

## Rule
Keep `.cargo/config.toml` environment-neutral. Do not commit absolute `/nix/store/...`
paths for `build.rustc`, `LIBCLANG_PATH`, or other machine-specific settings there.
Replit/Nix store hashes change when packages are upgraded or garbage-collected, and a
stale `build.rustc` path makes Cargo fail before it can run anything.

Use the active toolchain from `PATH` unless there is a current, local reason to override it.
If Replit needs an override, put it outside tracked repo files:

- Replit environment variables or shell profile
- User-level Cargo config at `~/.cargo/config.toml`
- One-off command environment, e.g. `LIBCLANG_PATH=... cargo build`

If `bindgen` fails with "couldn't find any valid shared libraries matching libclang.so",
set `LIBCLANG_PATH` in the local environment to the current Nix libclang directory. Do
not bake that path into `.cargo/config.toml`.

If a future dependency again requires nightly-only Rust features, prefer a real
toolchain selection (`rust-toolchain.toml`, `rustup override`, or Replit module/package
selection) over a committed absolute Nix store `build.rustc` path.

## Other notes
- The C++ capture overlay (`capture-overlay/`) is built by cmake during `cargo build`;
  cmake and Qt5 headers are installed as Nix packages.
