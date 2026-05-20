---
created: 2026-05-20T12:08:35.585Z
title: Add support for GitHub Actions
area: tooling
files: []
---

## Problem

The project currently has no CI/CD pipeline. GitHub Actions workflows are needed to automate building, testing, and releasing the tool across all three target platforms (Linux x86_64 musl, macOS aarch64, macOS x86_64). Without this, releases must be built and uploaded manually.

## Solution

Create `.github/workflows/` with at minimum:
- A CI workflow (on push/PR): `cargo check`, `cargo test`, `cargo clippy`
- A release workflow (on tag push): build static binaries for all three targets using native runners (musl for Linux, native macOS runners for both macOS targets), strip symbols, and upload to GitHub Releases

Use `dtolnay/rust-toolchain@stable` (not archived `actions-rs`), `Swatinem/rust-cache@v2` for caching. Linux target uses `x86_64-unknown-linux-musl` with musl-tools. macOS targets use `macos-14` (aarch64) and `macos-13` (x86_64).
