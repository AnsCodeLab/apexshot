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

## Building on Replit (confirmed working)

All native dependencies (GTK4, GStreamer, PipeWire, Tesseract, Qt5, clang, cmake) are
installed as Nix packages and the required env vars are baked into `.cargo/config.toml`:

- `LIBCLANG_PATH` — points to `libclang.so` for the `bindgen`/`pipewire` crate build
- `rustc` — points to the nightly 1.90.0 binary; needed because `rten 0.24` uses
  AVX-512 intrinsics that are still behind a nightly feature gate in rustc 1.88/stable.

```bash
cargo build          # dev build (all env vars auto-loaded from .cargo/config.toml)
cargo build --release
```

If you see `"couldn't find any valid shared libraries matching: libclang.so"`, the
clang nix-store hash changed — update the `LIBCLANG_PATH` value in `.cargo/config.toml`.

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
