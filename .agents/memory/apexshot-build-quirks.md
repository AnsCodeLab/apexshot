---
name: ApexShot build quirks
description: Why cargo build needs nightly rustc and LIBCLANG_PATH, and where those values live
---

## Rule
`cargo build` requires two non-obvious settings, both stored in `.cargo/config.toml`:

1. **`rustc`** — must point to the nightly binary in the nix store  
   (`/nix/store/rmzlxinjs6j26z6mnplfgjxa8adw0xba-rust-mixed/bin/rustc`, rustc 1.90.0-nightly)  
   Stable 1.88.0 (the default on this Replit) fails because `rten-simd 0.24` uses
   AVX-512 intrinsics behind the `avx512_target_feature` / `stdarch_x86_avx512` nightly
   feature gates.

2. **`LIBCLANG_PATH`** — must point to the nix clang lib dir  
   (`/nix/store/mmk6s6mc9kvz8czjfg9lh9m4sbc0wc8k-clang-21.1.7-lib/lib`)  
   The `pipewire` crate runs `bindgen` at build time, which calls into `libclang.so`.
   Without this, bindgen panics with "couldn't find any valid shared libraries matching libclang.so".

**Why:** The nix store hashes are content-addressed, so these paths are stable unless
clang or the nightly toolchain is upgraded. If either package changes, rebuild will fail
with the respective error and you must update the hash in `.cargo/config.toml`.

**How to apply:** For any `cargo build` / `cargo test` / `cargo run` commands, no manual
env var setting is needed — `.cargo/config.toml` handles it. If builds break after a Nix
package upgrade, find the new nix store path with `clang --version` (shows InstalledDir)
and `ls /nix/store/ | grep rust-mixed` to locate the nightly bin.

## Other notes
- `rten = "0.24"` / `rten-tensor = "0.24"` / `rten-imageproc = "0.24"` must all stay
  aligned with `ocrs = "0.12"`, which forces rten 0.24 transitively.
- `--ignore-rust-version` bypasses the Cargo.toml `rust-version` field check but does
  NOT enable nightly-gated language features — nightly rustc is still required.
- The C++ capture overlay (`capture-overlay/`) is built by cmake during `cargo build`;
  cmake and Qt5 headers are installed as Nix packages.
