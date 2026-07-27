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

## Building (requires Linux desktop with GTK4/X11 dev libs)

```bash
cargo build --release
```

## Running from source

```bash
cargo run -- daemon
```

## User preferences

- Imported to explore and read the code — no run workflow needed.
