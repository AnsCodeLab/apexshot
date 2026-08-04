# Codebase refactor implementation validation

Date: 2026-08-04  
Corrections applied: 2026-08-05  
Validated documents: [`CODEBASE_REFACTOR_AUDIT.md`](./CODEBASE_REFACTOR_AUDIT.md) and [`CODEBASE_REFACTOR_STATUS.md`](./CODEBASE_REFACTOR_STATUS.md)  
Scope: Current working-tree implementation of the `src/` refactor described as PRs 0-9.

---

## Executive verdict

The mechanical cleanup and structural splitting through PR 9 now match the audit, preserve their intended facades, and pass the core Rust test suites. The large GTK coordinators were correctly left in place rather than moved wholesale.

**Post-correction (2026-08-05):** the blocking validation findings are fixed:

- [x] PR 9 daemon IPC string regressions (`open_file`, preview D-Bus member, hotkey name match) restored with protocol unit tests.
- [x] `find_physical_input_device` narrowed back to `pub(crate)`.
- [x] Desktop-identity and package-metadata contract tests updated for `src/cli/install.rs` and current packaging reality.
- [x] PR 0 Clippy `needless_return` and Flatpak Tesseract dead-code warnings cleared.
- [x] PR 6 implementation tests moved to `backend.rs` and `controls.rs`; test-driven sibling visibility removed.
- [x] Status-document ownership/API descriptions and `EDITOR_CSS` visibility corrected.
- [ ] Manual GTK/D-Bus/portal/capture smoke matrix still unverified.
- [ ] PR guardrails still cannot be proven from commit history (single dirty working tree).

No source implementation was changed during the original validation pass. Corrections above were applied afterward.

---

## Status by planned PR

| Work item | Validation result | Assessment | Correction |
|-----------|-------------------|------------|------------|
| PR 0: clean gates | **Done** | Format, check, Clippy `-D warnings`, Flatpak check, and contract tests pass. | [x] |
| PR 1: dead code | Confirmed | The orphan and listed internal dead code were removed without deleting the live editor text paths. | n/a |
| PR 2: settings CSS | Confirmed | CSS was moved byte-for-byte and loaded through `include_str!`. | n/a |
| PR 3: editor CSS/tests | Confirmed | CSS and tests were moved correctly; targeted editor tests pass. | n/a |
| PR 4: render split | Confirmed | Module ownership, function bodies, and internal facade are preserved. | n/a |
| PR 5: overlay drawing split | Confirmed | Split is sound; status ownership wording corrected. | [x] docs |
| PR 6: recording split | **Confirmed** | Runtime and test ownership splits are complete; internal test helpers use minimum visibility. | [x] |
| PR 7: CLI split | **Confirmed** | Binary-only module split is correct. Contract tests now inspect `src/cli/install.rs`. | [x] |
| PR 8: hotkeys split | Confirmed | Platform/config ownership and facade are preserved. | n/a |
| PR 9: daemon split | **Confirmed after fix** | Physical split is coherent; IPC strings and crate-private API restored. | [x] |
| PR 10+ | Not started | The state and GTK coordinator work remains untouched as documented. | deferred |
| `capture_overlay.rs` | Deferred | It remains a single 2,431-line file with no child module tree. | deferred |

---

## Findings

### High: `open-file` daemon dispatch is broken — **FIXED**

The CLI sends the stable action name `"open_file"`:

- `src/main.rs:204-208`

**Was broken:** the daemon matched only `"capture_handlers::open_file"`.

**Fix:** restored stable protocol parsing via `parse_trigger_action` in `src/daemon/mod.rs`, matching `"open_file"`. Added `parse_trigger_action_accepts_stable_open_file_protocol_name` so Rust module paths cannot become IPC names.

### High: editor preview D-Bus calls use an invalid moved-function path — **FIXED**

**Was broken:** `show_preview_via_daemon` called `"capture_handlers::show_preview_for_path"`.

**Fix:** client now calls the stable member through `SHOW_PREVIEW_FOR_PATH_MEMBER` (`"show_preview_for_path"`), matching the zbus server method. Covered by `preview_dbus_member_is_stable_method_name`.

### Medium: hotkey name matching was corrupted during extraction — **FIXED**

Configured shortcuts use the binding name `"open_file"`:

- `src/hotkeys/config.rs:210-215`
- Confirmed by `src/hotkeys/mod.rs:718-739`

**Was broken:** listener recognized `"super::capture_handlers::open_file"` / `"open-file"` but not `"open_file"`.

**Fix:** restored `"open_file" | "open-file"` in `src/daemon/hotkey_listener.rs`. Added `binding_to_daemon_action_maps_open_file_by_name_without_args` so name-only bindings resolve without arg fallback.

### Medium: PR 9 introduced new downstream public API — **FIXED**

**Was broken:** `find_physical_input_device` was public and publicly re-exported.

**Fix:** function and re-export are `pub(crate)` again (`src/daemon/audio.rs`, `src/daemon/mod.rs`). Downstream path `apexshot::daemon::find_physical_input_device` is no longer public API.

### Medium: PR 6 did not complete test ownership — **FIXED**

The audit says recording tests should move when their target functions move. The initial split left them centralized in the parent and imported child internals wholesale.

- **Fix:** moved 10 backend tests to `src/recording/backend.rs:1740-1930` and 10 request/control tests to `src/recording/controls.rs:589-1030`.
- The parent now keeps only four coordinator/output-path tests in `src/recording/mod.rs:403-483`.
- Removed the parent module's broad `backend::*`, `controls::*`, and capture-request imports.

The implementation helpers that were sibling-visible mainly for parent tests are now private to their owning modules, including:

- `src/recording/backend.rs:10` - `CropMargins`
- `src/recording/backend.rs:122` - `compute_wayland_crop`
- `src/recording/backend.rs:234` / `242` - `EncoderProfile` / `PROFILES`
- `src/recording/backend.rs:351` - `ffmpeg_available_encoders`
- `src/recording/backend.rs:984` - `video_encoder_props`
- `src/recording/backend.rs:1017` - `normalize_recording_config_for_profile`
- `src/recording/controls.rs:24` - `should_use_shell_mask_for_request`

The public recording facade and required sibling backend entry points remain unchanged. Targeted recording tests pass: 60 passed.

### Medium: the documented full-test baseline is stale — **FIXED**

| Test | Was | Now |
|------|-----|-----|
| `packaged_desktop_identity_matches_primary_ui_application_id` | searched `src/main.rs` | inspects `src/cli/install.rs` |
| `opensuse_rpm_spec_matches_project_packaging_contract` | searched `src/main.rs` | inspects `src/cli/install.rs` |
| `deb_package_includes_capture_helper_binary` | expected Ubuntu 25.10 | expects Ubuntu 24.04 (matches workflow) |
| `opensuse_installer_contains_reported_dependency_set` | expected `opensuse-install.sh` dispatch | matches current zypper detect + local RPM build messaging |

All four contract tests pass.

### Medium: PR guardrails cannot be validated from commit history — **OPEN**

The refactor exists as a single dirty working tree on top of commit `63be852`. The new split files are untracked while the old monolith files appear deleted or modified. Packaging, workflow, installer, documentation, editor, recording, hotkey, and daemon changes are present together.

As a result, these process claims cannot be proven from repository history:

- One behavior domain per PR.
- First commit moves code without changing behavior.
- Packaging and editor/recording changes are isolated.
- Every conceptual PR ran its Flatpak gate.

The combined snapshot can be validated, but the proposed per-PR review sequence cannot.

### Low: PR 0 formatting / Clippy / Flatpak — **FIXED**

- [x] `cargo fmt --all -- --check` passes (was already green at validation time).
- [x] Three Clippy `needless_return` findings in `src/ocr/mod.rs` fixed.
- [x] Tesseract-only helpers gated with `cfg(feature = "tesseract-ocr")` / `cfg(any(test, feature = "tesseract-ocr"))`; Flatpak feature build is warning-clean aside from the intentional Qt skip note.
- [x] Contract tests green (see above).

### Low: some status descriptions are imprecise — **FIXED**

- The status now identifies the render `pub use` facade as crate-internal because `render` is private in `src/capture/editor.rs:10`.
- The status now assigns countdown, scroll, crosshair, and window-picker drawing to `mode_overlays.rs`, while general selection remains in `drawing/mod.rs`.
- `EDITOR_CSS` is private at `src/capture/editor/ui_support.rs:136`; its nested tests retain access without crate-wide visibility.

---

## Confirmed correct work

### PR 1: dead-code cleanup

- `src/settings/storage.rs` is deleted and no module declaration references it.
- All confirmed internal deletion candidates from the audit are absent.
- The five definition-only `EditorState` methods were removed.
- `cancel_text_edit` was correctly retained and remains called by the Escape handler.
- `get_text_bounds`, `update_text_action`, and `end_select_drag` are gated with `#[cfg(test)]`.
- The listed production-live editor text methods remain present and called.
- Public `ViewTransform::fit` and `ViewTransform::image_to_view` remain unchanged pending an API-policy decision.

### PRs 2-3: payload extraction

- Settings CSS is loaded from `src/settings/settings.css` through `include_str!("settings.css")`.
- Editor CSS is loaded from `src/capture/editor/editor.css` through `include_str!("editor.css")`.
- Byte comparison against the original embedded payloads found both CSS moves unchanged.
- CSS-focused tests inspect the CSS constants rather than Rust source.
- Source-structure tests that still inspect `ui_support.rs` do so intentionally.
- Editor root tests are wired through `#[cfg(test)]` and `#[path = "editor/tests.rs"]` in `src/capture/editor.rs:28-30`.
- A normalized comparison found the moved editor test bodies unchanged.

### PR 4: render split

- `src/capture/editor/render/mod.rs` declares and re-exports `arrows`, `text`, and `effects` correctly.
- Arrow, text, and effect ownership matches the audit's intended seams.
- Callers continue through the existing internal render facade.
- Shared and effect-specific tests remain present.
- Comparison with the pre-split file found all 68 common functions unchanged. The two removed functions are exactly the intentional PR 1 deletions.

### PR 5: overlay drawing split

- The three child modules are wired correctly.
- Recording UI, settings UI, and specialized mode overlays have coherent ownership.
- Dependency flow is one-way; no sibling module cycle was found.
- Comparison with the pre-split file found all 25 functions unchanged.
- Two helpers were safely narrowed from `pub(crate)` to `pub(super)` because no outside caller exists.

### PR 6: recording runtime split

- `audio`, `wf_recorder`, `backend`, and `controls` are wired correctly.
- The dependency direction is coherent: controls use backend/wf-recorder; backend uses wf-recorder/audio; wf-recorder uses audio.
- Existing facade paths for audio listing, control runners, request persistence, and `PreparedOverlayRecordingRequest` remain available.
- Comparison with the pre-split file found all 105 common function bodies unchanged.
- The five old-only functions are exactly the intentional PR 1 dead-code deletions.
- No dependency was added for the split.
- Backend and controls implementation tests are now child-owned, while four coordinator/output-path tests remain in the parent.
- Crop, encoder-profile, normalization, and shell-mask test helpers are private to their owning child modules.

### PR 7: CLI split

- `src/main.rs` is now primarily dispatch, daemon GTK bridging, usage, hotkey command handling, and tests.
- Install, native-host, and capture/OCR/record handlers are in the documented child files.
- `mod cli` exists only in the binary root, so the split did not create library API.
- `--help` and `--version` run successfully.
- **Fixed:** source-inspection contract tests now target `src/cli/install.rs`.

### PR 8: hotkeys split

- Config types, persistence, and accelerator conversion are in `src/hotkeys/config.rs`.
- GNOME, portal, KDE, and wlroots responsibilities are in the documented platform files.
- Existing externally visible names and signatures are preserved.
- The GNOME/portal relationship is one-way and does not form a module cycle.

### PR 9: daemon structure

- Audio monitoring, hotkey listeners, capture handlers, recording handlers, and scroll injection are in the documented child files.
- `run_daemon_inner` remains in `src/daemon/mod.rs` as a real coordinator rather than being moved only for line count.
- Most daemon facade paths remain stable, including clipboard/open helpers, recording notifications, action types, GTK work types, and D-Bus constants.
- No module cycle or unresolved Rust caller was found.
- **Fixed:** stable IPC action/member strings, hotkey name match, and crate-private `find_physical_input_device`.

### Deferred work and metrics

- All 161 current `src/**/*.rs` files are reachable from a crate root; no orphan Rust module was found.
- `capture_overlay.rs` is unchanged from the baseline and remains intentionally deferred.
- PR 10+ is genuinely not started: no `state/` child tree exists, and the large editor/overlay coordinators have not been relocated.
- No new Cargo dependency was introduced by the structural splits. The current `Cargo.toml` change is a license-expression change.

---

## Current inventory

Current `src/**/*.rs` inventory:

| Tier | Files | Lines |
|------|------:|------:|
| XL, at least 2,000 | 6 | 18,606 |
| L, 1,000-1,999 | 12 | 15,817 |
| M, 500-999 | 36 | 26,698 |
| S, below 500 | 107 | 22,435 |
| **Total** | **161** | **83,556** |

Current files at or above 2,000 lines:

| File | Exact lines | Assessment |
|------|------------:|------------|
| `src/capture/editor/window/mod.rs` | 4,247 | PR 10+ setup coordinator remains. |
| `src/capture/editor/window/events.rs` | 3,720 | PR 10+ event coordinator remains. |
| `src/capture/editor/state.rs` | 3,678 | State behavior split remains. |
| `src/overlay/window.rs` | 2,483 | PR 10+ overlay coordinator remains. |
| `src/capture_overlay.rs` | 2,431 | Explicitly deferred. |
| `src/capture/editor/tests.rs` | 2,047 | Test payload only. |

The approximate large-file table in the status document is accurate.

---

## Verification evidence

### Original validation (2026-08-04)

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo check --all-targets` | Pass |
| `cargo check --all-targets --no-default-features --features flatpak` | Pass with 10 unique Tesseract-only dead-code warnings |
| `cargo clippy --all-targets -- -D warnings` | Fail on 3 `needless_return` findings in `src/ocr/mod.rs` |
| `cargo test --lib` | Pass: 584 passed |
| `cargo test --bin apexshot` | Pass: 8 passed |
| `cargo test --test desktop_identity` | Fail: 0 passed, 1 failed |
| `cargo test --test package_metadata` | Fail: 3 passed, 3 failed |
| `git diff --check` | Pass |

### After corrections (2026-08-05)

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo check --all-targets` | Pass |
| `cargo check --all-targets --no-default-features --features flatpak` | Pass (no dead-code warnings) |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test --lib recording::` | Pass: 60 passed |
| `cargo test --lib` | Pass: 587 passed |
| `cargo test --bin apexshot` | Pass: 8 passed |
| `cargo test --test desktop_identity` | Pass: 1 passed |
| `cargo test --test package_metadata` | Pass: 6 passed |
| `git diff --check` | Pass |

No live GTK, daemon D-Bus, portal, X11/Wayland capture, audio, or recording smoke test was performed. The manual exit gates in the audit therefore remain unverified.

---

## Recommended correction order

1. [x] Restore the stable daemon strings for `open_file`, the preview D-Bus member, and hotkey name matching. Add focused protocol tests before doing more module movement.
2. [x] Narrow `find_physical_input_device` back to crate-private visibility and re-check the public facade diff.
3. [x] Update the desktop-identity and openSUSE package contract tests to inspect `src/cli/install.rs` after the PR 7 move.
4. [x] Move PR 6 implementation tests to their owning child modules and narrow test-driven visibility.
5. [x] Finish the remaining PR 0 OCR Clippy and Flatpak feature-gating work.
6. [x] Resolve or explicitly baseline the two packaging/workflow failures in their separate workstream. *(tests aligned to current Ubuntu 24.04 workflow and openSUSE installer messaging)*
7. [x] Re-run the full automated gates. [ ] Still needed: the audit's manual smoke matrix before calling PRs 1-9 merge-ready.

All actionable automated/code/documentation corrections are addressed. Remaining before merge-ready: the manual smoke matrix and repository-history evidence for per-PR process guardrails.
