#!/bin/bash
set -e

# Pre-fetch Cargo dependencies so the codebase is ready to build.
# Idempotent and non-interactive.
cargo fetch
