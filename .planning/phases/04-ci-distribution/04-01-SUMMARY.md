---
phase: 04-ci-distribution
plan: 01
subsystem: infra
tags: [github-actions, rust, musl, cross-compile, release, ci, distribution]

# Dependency graph
requires: []
provides:
  - GitHub Actions release workflow that builds 3 platform binaries on tag push
  - SHA256 checksum files for each binary
  - Automated Cargo.toml version-to-tag verification gate
  - ldd static-linking verification for Linux musl binary
affects: [04-02-install-script, distribution, release]

# Tech tracking
tech-stack:
  added:
    - dtolnay/rust-toolchain@stable
    - Swatinem/rust-cache@v2
    - softprops/action-gh-release@v2
    - actions/checkout@v4
  patterns:
    - Three-entry matrix.include (not 2D matrix) for per-platform CI configuration
    - RUSTFLAGS in matrix entry, consumed by build step env block only
    - shasum -a 256 for cross-platform checksum generation (not sha256sum)

key-files:
  created:
    - .github/workflows/release.yml
  modified: []

key-decisions:
  - "Use macos-15 for aarch64 and macos-15-intel for x86_64 (macos-13 retired Dec 2025)"
  - "softprops/action-gh-release@v2 chosen over gh release shell commands for concurrent-upload safety"
  - "RUSTFLAGS='-C target-feature=+crt-static' explicit for musl (default changing in future rustc)"
  - "cmake and pkg-config added alongside musl-tools as defensive insurance for vendored-libgit2"

patterns-established:
  - "GHA matrix.include pattern: each platform entry is self-contained with os/target/binary/asset_name/rustflags"
  - "version-check step strips 'v' prefix from tag via ${TAG#v} bash parameter expansion"

requirements-completed: [DIST-01, DIST-02, DIST-03, DIST-05]

# Metrics
duration: 12min
completed: 2026-05-20
---

# Phase 4 Plan 01: CI + Distribution — Release Workflow Summary

**GitHub Actions release workflow with 3-platform native matrix (ubuntu musl, macos-15 aarch64, macos-15-intel x86_64), SHA256 checksums, version-gate check, and ldd static-linking verification**

## Status: COMPLETE

Both tasks completed. Live GitHub Actions run verified — 6 release artifacts confirmed on GitHub Releases, Linux binary confirmed static via ldd.

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-20T00:00:00Z
- **Completed:** 2026-05-20
- **Tasks:** 2 of 2
- **Files modified:** 1

## Accomplishments

- Created `.github/workflows/release.yml` with three-platform matrix release workflow
- All 17 structural acceptance criteria pass (actionlint exits 0)
- Linux musl target configured with explicit `crt-static` flag for future-proof static linking
- Version-check step prevents tag/Cargo.toml mismatch from producing a misnamed binary
- Live GitHub Actions run verified: 6 artifacts (3 binaries + 3 SHA256 checksums) published to GitHub Releases
- ldd confirmed Linux binary is static (ldd fix: grep pattern accepts both "statically linked" and "not a dynamic executable")

## Task Commits

1. **Task 1: Create three-platform release workflow** — `6cb8f8b` (feat)
2. **Task 2: Live CI verification** — checkpoint approved by user; ldd fix committed `ca97f86` (fix)

## Files Created/Modified

- `.github/workflows/release.yml` — Three-platform GitHub Actions release workflow (92 lines)

## Matrix Entries

| Build | Runner | Target | Asset Name |
|-------|--------|--------|------------|
| linux-x86_64-musl | ubuntu-latest | x86_64-unknown-linux-musl | git-author-reformer-linux-x86_64 |
| macos-aarch64 | macos-15 | aarch64-apple-darwin | git-author-reformer-macos-aarch64 |
| macos-x86_64 | macos-15-intel | x86_64-apple-darwin | git-author-reformer-macos-x86_64 |

## Decisions Made

- **macos-15-intel for x86_64:** macos-13 was retired December 4, 2025. macos-15-intel is the native Intel runner available until August 2027.
- **macos-15 for aarch64:** Current stable arm64 runner; replaces macos-14.
- **softprops/action-gh-release@v2:** Handles concurrent matrix-job uploads to the same release without race condition failures. Preferred over `gh release` shell commands.
- **cmake + pkg-config in musl apt step:** Added defensively for vendored-libgit2 C compilation path (cheap insurance; musl-tools alone may suffice but cmake is a known cc-rs dependency).
- **Explicit RUSTFLAGS="-C target-feature=+crt-static":** Rust PR #133386 (merged Dec 2024) changes musl default to dynamic CRT linking. Explicit flag future-proofs the build.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] ldd grep pattern updated to accept musl "statically linked" output**
- **Found during:** Task 2 (live CI verification)
- **Issue:** The ldd verification step used `grep "not a dynamic executable"` but some musl toolchains produce `statically linked` instead. The CI check would have passed/failed inconsistently depending on the toolchain.
- **Fix:** Updated grep pattern to `grep -E "not a dynamic executable|statically linked"` so both valid forms of static-linking confirmation are accepted.
- **Files modified:** `.github/workflows/release.yml`
- **Commit:** `ca97f86`

## Issues Encountered

- actionlint download script required `latest /tmp` argument order (not `-b /tmp`). Corrected automatically; lint passed on first run.

## Task 2 — Live CI Verification: APPROVED

**User confirmed:** All 6 release artifacts appeared on GitHub Releases, and ldd confirmed the Linux binary is static.

### Artifacts Verified

| Artifact | Present |
|----------|---------|
| `git-author-reformer-linux-x86_64` | confirmed |
| `git-author-reformer-linux-x86_64.sha256` | confirmed |
| `git-author-reformer-macos-aarch64` | confirmed |
| `git-author-reformer-macos-aarch64.sha256` | confirmed |
| `git-author-reformer-macos-x86_64` | confirmed |
| `git-author-reformer-macos-x86_64.sha256` | confirmed |

### Static Linking Verified

`ldd git-author-reformer-linux-x86_64` confirmed binary is statically linked (musl, no dynamic dependencies).

## Next Phase Readiness

- `.github/workflows/release.yml` is committed, pushed, and verified in production
- Phase 4 Plan 02 (install script) may proceed — release artifacts are live at the expected URLs
- DIST-01, DIST-02, DIST-03, DIST-05 satisfied

---
*Phase: 04-ci-distribution*
*Completed: 2026-05-20*
