---
phase: 01-foundation-read-layer
plan: 02
subsystem: preflight
tags: [rust, git2, tdd, safety-gates, SAFE-01, SAFE-02]

requires:
  - 01-01  # AppError variants, stub fn signatures, tests/common/mod.rs helpers

provides:
  - check_stash: blocks when refs/stash exists (SAFE-01)
  - check_worktrees: blocks when linked worktrees exist (SAFE-02)

affects:
  - 01-04-wiring  # main.rs calls these gates before any rewrite begins
  - all-future-phases  # safety invariant: no rewrite runs with stash or linked worktrees

tech-stack:
  added: []
  patterns:
    - "find_reference(\"refs/stash\").is_ok() for stash detection — no ? propagation, missing ref is the success path"
    - "repo.worktrees()? -> StringArray; iter() yields Result<Option<&str>>; filter_map(|r| r.ok().flatten()) collects names"
    - "Integration tests use mod common; convention to share fixture helpers across test binaries"

key-files:
  created:
    - src/lib.rs
    - build.rs
    - tests/preflight_test.rs
  modified:
    - src/git/preflight.rs
    - src/main.rs
    - tests/common/mod.rs
    - Cargo.toml

key-decisions:
  - "filter_map(|r| r.ok().flatten()) over flatten() alone — StringArray::iter() yields Result<Option<&str>>, not Option<&str>; RESEARCH.md had this slightly wrong"
  - "src/lib.rs added (two lines: pub mod error; pub mod git;) — required for integration test crate imports; Plan 01 omitted this"
  - "build.rs added (empty, just rerun-if-changed) — required for Cargo to propagate native C lib link flags to integration test binaries on this toolchain"
  - "git2 added to [dev-dependencies] — same version as [dependencies]; required for Cargo to make the native link flags available to the test binary"
  - "mod decls stripped from main.rs, moved to lib.rs — avoids duplicate-module errors when both bin and lib exist"
  - "tests/common/mod.rs borrow fix: tree dropped before (dir, repo) return by scoping in a block"

metrics:
  duration: "~90 minutes (build environment debugging dominated)"
  completed: "2026-05-20"
  tasks_completed: 2
  files_changed: 7
---

# Phase 01 Plan 02: Preflight Gates (check_stash, check_worktrees) Summary

Implements SAFE-01 and SAFE-02: two pure blocking gates that prevent history rewrites when unsafe repo state exists.

## What Was Built

`check_stash` checks for a `refs/stash` reference. If found, returns `Err(AppError::StashDetected)`. Stash entries would be orphaned after a commit graph rewrite because the stash commits are not reachable from any ref that gets updated.

`check_worktrees` calls `repo.worktrees()` which returns only *linked* worktrees (the main worktree is excluded by libgit2 — Pitfall 4). If any exist, returns `Err(AppError::WorktreesDetected(names))` where names is a comma-joined list. Linked worktrees hold locks on their checked-out branches, making those branches unupdatable during a rewrite.

## Test Coverage (4 assertions)

| Test | Purpose | Result |
|------|---------|--------|
| `test_check_stash_passes_on_clean_repo` | Stash gate is transparent when no stash exists | PASS |
| `test_check_stash_blocks_when_stash_ref_exists` | Stash gate blocks when refs/stash is present | PASS |
| `test_check_worktrees_passes_on_single_worktree_repo` | Pitfall 4: main-only repo returns Ok(()) | PASS |
| `test_check_worktrees_blocks_when_linked_worktree_exists` | Linked worktree triggers WorktreesDetected | PASS |

## Implementation Notes

`check_stash` uses `find_reference(...).is_ok()` rather than `?` — a missing ref is the normal/success path, not an error to propagate. Only `Err` variants that are truly unexpected (like I/O errors) propagate via the `#[from] git2::Error` variant.

`check_worktrees` deviates slightly from RESEARCH.md Pattern 4. The research showed `worktrees.iter().flatten().collect()` but `StringArray::iter()` yields `Result<Option<&str>, Error>`, not `Option<&str>`. Correct form: `filter_map(|r| r.ok().flatten())`.

Both functions are pure — same repo state always produces same result.

Main.rs wiring (calling these gates before any rewrite) is Plan 04.

## Deviations from Plan

### Auto-fixed Issues (Rule 3 — Blocking)

**1. [Rule 3 - Blocking] src/lib.rs missing — integration tests cannot import crate**
- Found during: Task 1 compilation
- Issue: Plan 01 produced a binary-only crate (no lib.rs). The plan's test contract uses `use git_author_reformer::...` which requires a library target.
- Fix: Added `src/lib.rs` with `pub mod error; pub mod git;`. Stripped corresponding `mod error; mod git;` from `src/main.rs` to avoid duplicate-module errors.
- Files modified: `src/lib.rs` (created), `src/main.rs`
- Commit: b51e8be

**2. [Rule 3 - Blocking] build.rs missing — native C lib link flags not propagated to test binary**
- Found during: Task 1 link phase
- Issue: Cargo does not propagate `cargo:rustc-link-lib=static=git2` from transitive build scripts to integration test binaries unless the package itself has a build.rs. The test binary linked against libgit_author_reformer.rlib (which uses git2/libgit2-sys) but the linker did not receive -lgit2, causing undefined symbol errors.
- Fix: Added minimal `build.rs` (rerun-if-changed only) and `git2` in `[dev-dependencies]`. Together these give Cargo the full dependency graph for link-flag propagation.
- Files modified: `build.rs` (created), `Cargo.toml`
- Commit: b51e8be

**3. [Rule 1 - Bug] Pre-existing borrow error in tests/common/mod.rs**
- Found during: Task 1 compilation
- Issue: `tree` (borrows `repo`) outlived the return statement `(dir, repo)`. Plan 01 committed this code but it was never compiled into an integration-test binary, so the error was latent.
- Fix: Wrapped the `tree` usage in a block so it drops before the return.
- Also added `#![allow(dead_code)]` at top of file — `add_commit_with_message` is not used by preflight_test and would fire clippy `-D warnings`.
- Files modified: `tests/common/mod.rs`
- Commit: b51e8be

**4. [Rule 1 - Bug] StringArray::iter() yields Result<Option<&str>>, not Option<&str>**
- Found during: Task 2 compilation
- Issue: RESEARCH.md Pattern 4 showed `.iter().flatten().collect()` but flatten() on `Iterator<Item=Result<Option<&str>>>` does not produce `&str`.
- Fix: Changed to `.iter().filter_map(|r| r.ok().flatten()).collect()`.
- Files modified: `src/git/preflight.rs`
- Commit: 6555186

**5. [Environment] Broken CLT ranlib symlink required ar wrapper**
- Found during: Task 1 build phase
- Issue: `/Library/Developer/CommandLineTools/usr/bin/ranlib` is inaccessible (TCC/SIP blocked), causing all archive creation to fail. cc-rs uses `ar cqD` + `ar sD` for building libgit2-sys. The wrapper at `/tmp/ar-wrapper.sh` replaces `ar cqD` with `libtool -static` (incremental, merging prior archive contents) and makes `ar sD` a no-op (libtool already indexes the archive).
- Note: This wrapper is specific to this machine. All cargo invocations during this session used `AR=/tmp/ar-wrapper.sh`. The wrapper is not committed to the repo (machine-specific). The `AR` value is captured in cargo's fingerprint; future `cargo build` invocations on this machine should also set `AR=/tmp/ar-wrapper.sh`.

## Known Stubs

None. All `todo!()` placeholders in `src/git/preflight.rs` have been replaced.

## Threat Flags

None. No new network endpoints, auth paths, file access patterns, or schema changes introduced. The functions are read-only inspections of the git repo state.

## Self-Check: PASSED

- src/git/preflight.rs: FOUND
- src/lib.rs: FOUND
- tests/preflight_test.rs: FOUND
- build.rs: FOUND
- 01-02-SUMMARY.md: FOUND
- Commit b51e8be (RED): FOUND
- Commit 6555186 (GREEN): FOUND
- `cargo test --test preflight_test`: 4 passed; 0 failed
- `cargo clippy --tests -- -D warnings`: clean
