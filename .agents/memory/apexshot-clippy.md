---
name: Running clippy on ApexShot in this env
description: How to run cargo clippy despite the nightly-rustc/stable-clippy toolchain mismatch
---
The nightly rust-mixed toolchain (used via `build.rustc` in .cargo/config.toml) ships NO clippy-driver; the PATH clippy is stable 1.88, and rten/rten-simd need nightly AVX-512 features. Plain `cargo clippy` fails.

**How to apply:** run clippy fully on the stable toolchain with feature gates unlocked and a separate target dir (to avoid mixing nightly-built artifacts):

```
RUSTC_BOOTSTRAP=1 \
RUSTFLAGS='-C debuginfo=line-tables-only -Zcrate-attr=feature(stdarch_x86_avx512,avx512_target_feature,generic_arg_infer)' \
CARGO_BUILD_RUSTC=/nix/store/2x94g3skvh12651bli2ab0bmx24lxb6l-rust-mixed/bin/rustc \
CARGO_TARGET_DIR=target/clippy \
cargo clippy --ignore-rust-version
```

**Why:** E0514 metadata mismatch otherwise; RUSTC_BOOTSTRAP + -Zcrate-attr lets stable compile the nightly-feature deps. ~165 pre-existing `uninlined_format_args` style warnings exist codebase-wide (not errors).
