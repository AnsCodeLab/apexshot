# Codebase refactor status

Date: 2026-08-05  
Source plan: [`CODEBASE_REFACTOR_AUDIT.md`](./CODEBASE_REFACTOR_AUDIT.md)  
Scope: `src/` Rust refactor PRs from that audit.

---

## Summary

| Band | Status |
|------|--------|
| **PR 0** (clean merge gates) | **Done** |
| **PRs 1–9** (structural splits) | **Done** |
| **PR 10+** (EditorState / GTK coordinators) | **Started** — first safe slices landed; large coordinators remain |
| **`capture_overlay.rs` split** | **Deferred** (still ~2.4k lines) |

PR 10+ follows the audit rule: extract complete behavior slices with clear owners. Do **not** relocate whole multi-thousand-line functions only to improve line-count metrics.

---

## PR checklist

### PR 0–9 — done

See earlier sections in git history / audit. Short recap:

| PR | Result |
|----|--------|
| 0 | fmt, Clippy, default + Flatpak checks, contract tests |
| 1 | Dead code / orphan module deleted |
| 2–3 | Settings + editor CSS/`include_str!`; editor tests path |
| 4–6 | Render, overlay drawing, recording splits |
| 7–9 | CLI (`cli/`), hotkeys platforms, daemon handlers |

---

### PR 10+: editor state and GTK coordinators — in progress

#### Done (2026-08-05)

##### 1. `EditorState` module tree

`src/capture/editor/state.rs` → `src/capture/editor/state/`:

| File | Role | Approx. lines |
|------|------|--------------:|
| `mod.rs` | `EditorState` / `TextInputState` structs, shared free helpers, remaining impls | ~2,280 |
| `text_input.rs` | Text input / fit / commit path | ~580 |
| `history.rs` | Undo/redo/push/history availability | ~130 |
| `drag_draw.rs` | Begin/update/clear/draft/finalize drag + canvas expand | ~480 |
| `export.rs` | `to_rendered_image` / `to_final_image` / background compose | ~250 |

Struct stays in `state/mod.rs` so private field access remains inside the `state` module tree. No new public crate API.

##### 2. Editor events — first EventContext-owned families

`src/capture/editor/window/events.rs` → `src/capture/editor/window/events/`:

| File | Role | Approx. lines |
|------|------|--------------:|
| `mod.rs` | `EventContext`, `wire_editor_events` dispatcher, remaining families | ~3,490 |
| `zoom.rs` | `wire_zoom_controls` (zoom UI, ctrl+scroll, pan) | ~260 |
| `history.rs` | `wire_history_buttons` (undo/redo/delete) | ~65 |

##### 3. Editor setup — inspector shell

| File | Role | Approx. lines |
|------|------|--------------:|
| `window/inspectors.rs` | `build_tool_inspectors` → `InspectorParts` | ~280 |

Same pattern as `footer.rs` / `canvas.rs` / `toolbar.rs` (Parts struct + `pub(super)` builder).

##### 4. Overlay window peel

`src/overlay/window.rs` → `src/overlay/window/`:

| File | Role | Approx. lines |
|------|------|--------------:|
| `mod.rs` | `setup_window` + selection result + aspect helpers | ~2,060 |
| `audio.rs` | Local PW audio meters / daemon poll | ~310 |
| `platform.rs` | Overlay CSS + X11 compositor animation suppress | ~120 |

##### 5. Daemon coordinator thin-out

| File | Role | Approx. lines |
|------|------|--------------:|
| `daemon/dispatch.rs` | `dispatch_daemon_action` (full action `match`) | ~220 |
| `daemon/mod.rs` | `run_daemon_inner` setup + loop calling dispatch | ~980 total; **`run_daemon_inner` ~170 lines** |

`run_daemon_inner` is now a real coordinator (handlers already out in PR 9; action match out in PR 10).

Structure tests that `include_str!` production sources were updated to follow moved modules (`inspectors.rs`, `events/{mod,zoom,history}.rs`, `overlay/window/mod.rs`).

---

## Still large — tackle next (tomorrow)

These are the remaining high-coupling coordinators and oversized files. Prefer **behavior slices with clear owners**, not whole-function file moves.

### Logic-bearing coordinators still over ~1,000 lines

| Function | Location | Approx. span | Why it is hard |
|----------|----------|-------------:|----------------|
| **`wire_editor_events`** | `capture/editor/window/events/mod.rs` | **~3,160 lines** | GTK callbacks share `EventContext`, widgets, and sync closures. Zoom + history are out; canvas drag/click/motion and keyboard remain. |
| **`setup_editor_window_full`** | `capture/editor/window/mod.rs` | **~2,990 lines** | Construction order, widget lifetimes, async effects, `set_draw_func`. Inspector shell is out; draw_func + layout chrome remain. |
| **`overlay::window::setup_window`** | `overlay/window/mod.rs` | **~1,850 lines** | Motion hit-test, toolbar clicks, drag gestures, keyboard. Audio/CSS/X11 helpers are out; input surface remains. |

### Other large files (navigation / deferred)

| File | Approx. lines | Notes |
|------|--------------:|-------|
| `capture/editor/window/mod.rs` | ~4,050 | Dominated by `setup_editor_window_full` + tests |
| `capture/editor/window/events/mod.rs` | ~3,490 | Dominated by `wire_editor_events` |
| `capture_overlay.rs` | ~2,430 | **Deferred** process-boundary split (audit: after daemon/capture) |
| `capture/editor/state/mod.rs` | ~2,280 | More `EditorState` groups still inline (selection, crop, arrow, tool style) |
| `overlay/window/mod.rs` | ~2,060 | Dominated by `setup_window` |
| `capture/editor/tests.rs` | ~2,050 | Test payload only |
| `recording/backend.rs` | ~1,930 | Already split out of recording facade; further internal split optional |
| `recording/stop_overlay.rs` | ~1,710 | Separate from main overlay window |

### Recommended next extractions (ordered by safety)

1. **Editor events**
   - Save / Done / copy / upload button families (all deps on `EventContext`)
   - Crop apply/reset buttons
   - Tool-button mode switches (repetitive)
   - **Defer** canvas drag (~800 lines), canvas click/motion (~770), full keyboard until shared helpers exist

2. **Editor setup**
   - Crop dim fields + option-list construction (returns widgets)
   - Async effects pipeline only if closures already have a clear owner
   - **Defer** `set_draw_func` body and large layout/chrome until Parts ownership is clear

3. **More `EditorState` child modules**
   - Selection / crop / arrow / tool-style groups still in `state/mod.rs`
   - Keep struct in `state/mod.rs`; child `impl EditorState` only inside the `state` tree

4. **Overlay `setup_window`**
   - Window shell construction (window + drawing area + geometry) → `OverlayWindowParts`
   - **Defer** toolbar-click and motion blocks without shared hit-test ownership

5. **`capture_overlay.rs`**
   - Worker lifecycle, output parsing, wlroots routing, recording request spawn — separate PR after coordinator work settles

6. **Manual smoke** (still required before calling PR 10 “done”)
   - Capture → editor annotate/undo/redo/crop/save  
   - Settings open/save/close  
   - Daemon hotkey + record start/pause/resume/stop  
   - Overlay on X11 and Wayland as applicable  

---

## Success metrics — progress

| Metric | Audit baseline | Target | Now |
|--------|---------------:|-------:|-----|
| Unreachable Rust modules | 1 | 0 | **0** |
| Confirmed caller-free internal functions | 17 | 0 | **Addressed (PR 1)** |
| Flatpak dead-code warnings | 10 | 0 | **0** |
| Logic-bearing coordinators over 1,000 lines | 3 | 0 after GTK work | **3 remain** (`wire_editor_events`, `setup_editor_window_full`, `setup_window`) |
| `run_daemon_inner` as thin coordinator | large match inline | handlers + dispatch out | **Done** (~170 lines) |
| New public API from splits | 0 | 0 | **Held** |

A 3,000-line function moved into a 3,000-line file is **not** success. Line count is secondary to ownership.

---

## Guardrails (still in force)

- One behavior domain per change set  
- First commit moves code; follow-ups may simplify  
- No new dependencies for module splitting  
- No public re-export changes unless reviewed as API  
- No whole-function relocation solely for file-size targets  
- No mass removal of `#[allow(dead_code)]`  
- Keep `cargo check --all-targets --no-default-features --features flatpak` in every gate  

---

## Verification

Re-run before merge / after tomorrow’s work:

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo check --all-targets --no-default-features --features flatpak
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

PR 10 slice automated coverage already exercised:

- `cargo test --lib -- capture::editor`  
- `cargo test --lib -- daemon::`  
- `cargo test --test window_picker_ui_contract`  
