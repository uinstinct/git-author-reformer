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

## Status: PARTIAL — Task 1 complete, Task 2 pending human verification

Task 2 is a `checkpoint:human-verify` gate requiring a live GitHub Actions run. It cannot be auto-approved.

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-20T00:00:00Z
- **Completed:** 2026-05-20
- **Tasks:** 1 of 2 (Task 2 is a blocking checkpoint)
- **Files modified:** 1

## Accomplishments

- Created `.github/workflows/release.yml` with three-platform matrix release workflow
- All 17 structural acceptance criteria pass (actionlint exits 0)
- Linux musl target configured with explicit `crt-static` flag for future-proof static linking
- Version-check step prevents tag/Cargo.toml mismatch from producing a misnamed binary

## Task Commits

1. **Task 1: Create three-platform release workflow** — `6cb8f8b` (feat)

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

None - plan executed exactly as written.

## Issues Encountered

- actionlint download script required `latest /tmp` argument order (not `-b /tmp`). Corrected automatically; lint passed on first run.

## Pending: Task 2 — Human Verification Checkpoint

**What to verify:** Push a tag to GitHub and confirm the workflow runs correctly end-to-end.

### Steps

1. Ensure Cargo.toml version matches the intended tag. Currently `version = "0.1.0"` — use tag `v0.1.0`, or bump the version to match a different tag.
2. Push the workflow to main: `git push origin main`
3. Push a version tag: `git tag v0.1.0 && git push origin v0.1.0`
4. Visit https://github.com/uinstinct/git-author-reformer/actions — confirm 3 parallel jobs launch (linux, macos-aarch64, macos-x86_64)
5. After all 3 jobs complete, visit https://github.com/uinstinct/git-author-reformer/releases
6. Confirm exactly 6 artifacts are present:
   - `git-author-reformer-linux-x86_64`
   - `git-author-reformer-linux-x86_64.sha256`
   - `git-author-reformer-macos-aarch64`
   - `git-author-reformer-macos-aarch64.sha256`
   - `git-author-reformer-macos-x86_64`
   - `git-author-reformer-macos-x86_64.sha256`
7. Download the Linux binary and run: `ldd git-author-reformer-linux-x86_64` — expected output: `not a dynamic executable`

### Resume Signal

Type "approved" once all 6 release artifacts are visible and ldd confirms the Linux binary is static. Describe any failures so they can be diagnosed.

### Common Failure Modes

- "Cargo.toml version does not match tag" → bump `Cargo.toml` version to match the tag before pushing
- Linux musl build C error → cmake/pkg-config apt step should cover it; check build log for specifics
- "Resource not accessible by integration" → `permissions: contents: write` is in the YAML; if this appears, the file has been modified incorrectly

## Next Phase Readiness

- `.github/workflows/release.yml` is committed and ready to push
- Task 2 (live CI verification) must be completed before Phase 4 Plan 02 (install script) begins
- No blockers on the workflow file itself — all acceptance criteria pass locally

---
*Phase: 04-ci-distribution*
*Completed: 2026-05-20 (partial — Task 2 pending)*
