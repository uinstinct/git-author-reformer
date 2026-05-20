---
phase: 01-foundation-read-layer
plan: 04
subsystem: cli-wiring
tags: [rust, clap, integration-tests, exit-codes, preflight, subprocess]

requires:
  - 01-01
  - 01-02
  - 01-03

provides:
  - main.rs: open_repo → check_stash → check_worktrees with stderr errors and exit(1)
  - tests/main_integration_test.rs: 4 end-to-end tests invoking binary as subprocess

affects:
  - All future phases (binary entry point is now fully wired)

tech-stack:
  added:
    - "std::process::exit(1) — uniform exit-code contract for all AppError failures"
    - "CARGO_BIN_EXE_git-author-reformer — Cargo integration test env var for binary path"
  patterns:
    - "fn run() -> Result<(), AppError> helper separates logic from main()"
    - "env_remove(GIT_DIR/GIT_WORK_TREE/GIT_COMMON_DIR) in subprocess tests to prevent env leakage"
    - "refs/stash created via repo.reference() for stash fixture state"
    - "wt_parent.path().join('linked-wt') for worktree path (must not exist prior)"

key-files:
  created:
    - tests/main_integration_test.rs
  modified:
    - src/main.rs

key-decisions:
  - "run() helper pattern: clap parse first (so --help/--version short-circuit), then run() for logic — keeps exit-code contract clean"
  - "env_remove on GIT_DIR etc. in subprocess tests — defensive guard against CI/shell leakage"
  - "cargo fmt applied during Task 3 — long lines and mod ordering normalized across prior plans' test files"

requirements-completed: [CORE-02]

duration: ~20min
completed: 2026-05-20
---

# Phase 01 Plan 04: CLI Wiring and Integration Tests Summary

**main.rs wired: open_repo → check_stash → check_worktrees with stderr errors and exit(1); 4 end-to-end subprocess tests verify exit-code contract**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-05-20
- **Completed:** 2026-05-20
- **Tasks:** 3 executed (Task 1: main.rs wiring, Task 2: integration tests, Task 3: final verification)
- **Files modified:** 2 (src/main.rs updated, tests/main_integration_test.rs created)
- **Formatting applied:** cargo fmt during Task 3 (5 files — style only, no logic)

## Accomplishments

- `src/main.rs` wired with `fn run()` calling `open_repo()` → `check_stash()` → `check_worktrees()`
- `eprintln!("error: {e}")` on any `AppError` — errors go to stderr, not stdout
- `std::process::exit(1)` on any failure; exit 0 on success
- `--help` and `--version` continue to work (clap parse runs before `run()`)
- 4 end-to-end integration tests via `std::process::Command` and `CARGO_BIN_EXE_git-author-reformer`
- All 15 phase tests pass (4 preflight + 7 reader + 4 main_integration)
- Release binary built and verified at `target/release/git-author-reformer`

## Task Commits

1. **Task 1: Wire main.rs preflight chain** — `2d08886`
2. **Task 2: End-to-end CLI integration tests** — `346b12a`
3. **Task 3: Apply cargo fmt (style only)** — `6b6445a`

## main.rs Wiring Chain

```rust
fn run() -> Result<(), error::AppError> {
    let repo = git::open_repo()?;
    git::preflight::check_stash(&repo)?;
    git::preflight::check_worktrees(&repo)?;
    println!("git-author-reformer: preflight passed");
    Ok(())
}

fn main() {
    let _cli = Cli::parse();        // clap runs first — --help/--version exit here
    if let Err(e) = run() {
        eprintln!("error: {e}");    // AppError Display to stderr
        std::process::exit(1);
    }
}
```

## End-to-End Test Assertions

| Test | Fixture State | Expected Exit | Expected stderr Substring |
|------|---------------|---------------|---------------------------|
| `test_binary_exits_with_error_outside_git_repo` | plain TempDir (no .git) | non-zero | `Not inside a git repository` |
| `test_binary_blocks_when_stash_ref_exists` | fixture repo + `refs/stash` ref | non-zero | `Stash entries detected` |
| `test_binary_blocks_when_linked_worktree_exists` | fixture repo + linked worktree | non-zero | `Linked worktrees detected` |
| `test_binary_passes_preflight_on_clean_repo` | clean fixture repo | 0 (success) | (none — success path) |

## Phase 1 Success Criteria — Verification Map

| Phase 1 Criterion | Test(s) | Status |
|-------------------|---------|--------|
| 1. Binary outside repo exits 1 with descriptive stderr | `test_binary_exits_with_error_outside_git_repo` | VERIFIED |
| 2a. Binary blocks on stash | `test_binary_blocks_when_stash_ref_exists` | VERIFIED |
| 2b. Binary blocks on linked worktrees | `test_binary_blocks_when_linked_worktree_exists` | VERIFIED |
| 3. enumerate_authors counts and sorts descending | `test_enumerate_authors_counts_and_sorts_descending` (Plan 03) | VERIFIED |
| 4. enumerate_coauthors case-insensitive dedup | `test_enumerate_coauthors_case_insensitive_dedup` (Plan 03) | VERIFIED |

All 5 criteria verified — `cargo test` output: **15 passed, 0 failed**.

## AppError Display Strings Used in Tests

```
AppError::NotARepo     → "Not inside a git repository: {path}"
AppError::StashDetected → "Stash entries detected. Pop or drop all stashes..."
AppError::WorktreesDetected → "Linked worktrees detected: {names}..."
```

Tests assert on substrings, not full messages, to be resilient to path/name variation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug / Style] cargo fmt applied during Task 3 verification**
- **Found during:** Task 3 (`cargo fmt -- --check` exited 1)
- **Issue:** Formatting drifted in 5 files (src/git/mod.rs, src/git/reader.rs, tests/main_integration_test.rs, tests/preflight_test.rs, tests/reader_test.rs). Long assert! lines and mod ordering.
- **Fix:** Ran `cargo fmt`. No logic changes — whitespace and line-wrapping only.
- **Files modified:** 5 files
- **Commit:** `6b6445a`

**2. [Rule 3 - Acceptance Criteria] Factored binary() helper inlined back into each test**
- **Found during:** Task 2 grep check — `CARGO_BIN_EXE_git-author-reformer` count was 1 (helper) not ≥4 (per-test)
- **Issue:** Plan acceptance criterion requires at least 4 occurrences of `CARGO_BIN_EXE_git-author-reformer` — one per test function.
- **Fix:** Inlined `Command::new(env!("CARGO_BIN_EXE_git-author-reformer"))` into each of the 4 test functions.
- **Files modified:** tests/main_integration_test.rs

## Known Stubs

None — all preflight gates are fully wired to the binary entry point. `println!("git-author-reformer: preflight passed")` is a intentional placeholder for Phase 3's TUI (not a data stub — it's a success acknowledgment with no data to wire yet).

## Threat Flags

None — no new network endpoints or file access patterns. The binary reads from CWD's git repo only.

---
*Phase: 01-foundation-read-layer*
*Completed: 2026-05-20*
