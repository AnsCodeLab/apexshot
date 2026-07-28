---
name: Running ApexShot tests in the Replit env
description: Why cargo test binaries fail to start here, and the LD_LIBRARY_PATH recipe that fixes it
---

## Rule
`cargo build` succeeds but `cargo test` dies immediately with
`error while loading shared libraries: libgobject-2.0.so.0: cannot open shared object file`
(exit status 127) before any test runs.

Compiled test binaries do not carry an rpath to the Nix-provided GTK/GStreamer/Tesseract
libraries, so the loader cannot find them. Export `LD_LIBRARY_PATH` built from pkg-config
before running tests:

```bash
export LD_LIBRARY_PATH="$(for p in gobject-2.0 glib-2.0 gio-2.0 cairo gtk4 \
  gdk-pixbuf-2.0 pango harfbuzz graphene-gobject-1.0 gstreamer-1.0 \
  gstreamer-app-1.0 gstreamer-video-1.0 libpipewire-0.3 tesseract lept \
  x11 xcb wayland-client libadwaita-1 gtk4-layer-shell-0; do
    pkg-config --variable=libdir "$p" 2>/dev/null
  done | sort -u | paste -sd:)"
cargo test
```

**Why:** Deriving the dirs from pkg-config keeps working after Nix package upgrades, whereas
hardcoded `/nix/store/<hash>` paths go stale. This is a runtime-loader issue only — it never
affects `cargo build`, which is why a green build tells you nothing about whether tests can start.

**How to apply:** Any time you need to run the test suite (or any built test binary) in this
environment. Do not "fix" it by adding `LD_LIBRARY_PATH` to `.cargo/config.toml` — that would
also inject the paths into build scripts, bindgen, and the cmake overlay build, where shadowing
system libs can break the build. Keep it in the test invocation.

## Other notes
- The `Doc-tests` phase always fails here with `error[E0514]: found crate ... compiled by an
  incompatible version of rustc` (for `image`, `gtk4`, `rten_tensor`, …). `.cargo/config.toml`
  overrides `rustc` to the nix nightly but leaves `rustdoc` as stable, so rustdoc cannot load
  nightly-built rlibs. All real test binaries pass before this phase. Judge `cargo test` on the
  per-target `test result: ok` lines, not the final exit code, and use `--doc`-free invocations
  (e.g. `--lib`, `--tests`) when you need a clean exit status.
- `recording::tests::reap_child_if_exited_clears_completed_child` is timing-dependent: it sleeps
  50 ms and asserts a spawned `sh -c 'exit 0'` has been reaped. It fails under CPU contention
  (e.g. parallel test threads plus rust-analyzer) and passes reliably on its own. Treat a lone
  failure of this test as environmental flakiness, not a regression.
- Long builds cannot be backgrounded: processes launched with `nohup`/`setsid &` from a shell
  tool call are reaped when that call ends. Run `timeout 250 cargo build` in the foreground
  across successive calls instead — Cargo resumes from `target/` each time.
- rust-analyzer's own `cargo check --workspace --all-targets` holds the `target/` build lock and
  can block your build for many minutes ("Blocking waiting for file lock on build directory").
