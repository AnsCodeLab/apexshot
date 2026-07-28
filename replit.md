# ApexShot

Open-source Linux screenshot and screen recording tool with annotation, OCR, QR code detection, and ShareX-style workflows.

- **Repo:** https://github.com/apex-shot/apexshot
- **License:** GPL-3.0
- **Language:** Rust (2021 edition)
- **Version:** 0.2.33

## What it is

ApexShot is a native Linux desktop application targeting GNOME Wayland and other Linux desktops. It is **not a web app** and cannot run in Replit's preview pane — it requires a real Linux desktop with X11 or Wayland, GTK4, and system-level display access.

## Key features

- Full screen / area / window / crosshair screenshots
- Annotation: arrows, shapes, text, blur, pixelate, crop, highlighter
- Screen recording (MP4/GIF) with audio monitoring
- OCR and QR code detection from captured regions
- Browser scroll capture
- Global hotkeys, tray icon, daemon mode
- GNOME Wayland portal-backed capture paths

## Project layout

```
src/
  main.rs               — entry point
  daemon/               — background daemon logic
  capture/              — screenshot capture backends
  capture_overlay.rs    — area-selection overlay
  recording/            — screen recording engine
  pipewire_engine.rs    — PipeWire-based recording
  overlay/              — GTK4 overlay windows
  annotations/          — annotation tools
  ocr/                  — OCR integration
  qr/                   — QR detection
  hotkeys/              — global hotkey registration
  tray/                 — system tray
  cloud/                — upload/sharing backends
  settings/             — config management
  gnome_integration/    — GNOME Shell extension glue
  compositor/           — compositor detection
  distro/               — distro-specific paths
  backend/              — capture backend abstraction
  config.rs             — configuration structs
  lib.rs                — library root
capture-overlay/        — C/CMake overlay component
docs/                   — architecture and developer guides
assets/                 — icons and sounds
data/                   — bundled data files
```

## Building on Replit

All native dependencies (GTK4, GStreamer, PipeWire, Tesseract, Qt5, clang, cmake) are
installed as Nix packages via `replit.nix`. Keep `.cargo/config.toml` portable: do
not commit absolute `/nix/store/...` values for `rustc` or `LIBCLANG_PATH` there.
Those hashes change when Replit upgrades or garbage-collects Nix packages, which
makes local Cargo fail before it can build.

```bash
cargo build
cargo build --release
```

If Replit needs an environment-specific override, set it outside git-tracked files,
for example in Replit environment variables, the shell profile, or `~/.cargo/config.toml`.
If you see `"couldn't find any valid shared libraries matching: libclang.so"`, set
`LIBCLANG_PATH` in that local environment to the current Nix libclang directory rather
than editing `.cargo/config.toml`.

The C++ capture overlay (`build.rs` → `capture-overlay/`) is compiled by cmake at
build time and requires `cmake` and Qt5 headers (both installed via Nix).

## Running from source

```bash
cargo run -- daemon
```

Note: the binary requires a live X11/Wayland session, GTK4, and PipeWire at runtime
and cannot run inside Replit's sandbox preview.

## User preferences

- Imported to explore and read the code — no run workflow needed.
- Never commit pasted screenshots to the repo. Files under `attached_assets/` are
  chat references only — the directory is gitignored; do not `git add` or commit them.
