---
phase: 03-tui-integration
plan: "01"
subsystem: git-scan
tags: [tui-deps, scan, preview, cascade, tdd]
dependency_graph:
  requires: []
  provides: [git::scan::RewritePreview, git::scan::scan_rename, git::scan::scan_drop, tui module skeleton]
  affects: [src/git/mod.rs, src/lib.rs, Cargo.toml]
tech_stack:
  added: [ratatui 0.30, crossterm 0.29, nucleo 0.5, signal-hook 0.4]
  patterns: [cascade-tracking revwalk, HashSet<Oid> for cascade set, commit_create_buffer+commit_signed for test fixtures]
key_files:
  created:
    - src/git/scan.rs
    - src/tui/mod.rs
    - tests/scan_test.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/git/mod.rs
    - src/lib.rs
decisions:
  - "build_full_revwalk extracted as private helper shared by scan_rename and scan_drop — eliminates revwalk setup duplication without widening crate visibility"
  - "RemoteArray iter yields Result<Option<&str>>; use filter_map(|r| r.ok().flatten()) not flatten()"
  - "RewritePreview.annotated_tags_affected stores short names (no refs/tags/ prefix) for display use"
  - "scan_drop message check uses strip_coauthor_prefix + parse_coauthor_value directly (not drop_coauthor_from_message) — simpler, avoids CRLF normalization complexity"
metrics:
  duration: "~35 minutes"
  completed: "2026-05-20"
  tasks_completed: 5
  files_changed: 7
---

# Phase 3 Plan 01: TUI Dependencies and Git Scan Module Summary

Added four TUI-stack dependencies to Cargo.toml (ratatui, crossterm, nucleo, signal-hook), implemented the read-only `git::scan` module with cascade-accurate `RewritePreview`, and created the empty `src/tui/` namespace for Wave 2.

## What Was Built

### src/git/scan.rs
The load-bearing gap that RENAME-05 and DROP-04 require. Two public functions:

- `scan_rename(repo, old_name, old_email) -> Result<RewritePreview>` — replicates the exact cascade logic from `rewrite_author` (topological revwalk + `would_remap: HashSet<Oid>`) without writing any commits. Returns affected count matching what `rewrite_author` would produce.
- `scan_drop(repo, target_email) -> Result<RewritePreview>` — same cascade tracking for co-author drops.

`RewritePreview` fields:
- `affected_count`: cascade-accurate commit count (not a naive identity match count)
- `signed_commit_count`: GPG/SSH signed commits in the cascade set (SAFE-03)
- `annotated_tags_affected`: short tag names pointing at cascade commits (SAFE-04)
- `has_notes_ref`: true if `refs/notes/commits` or configured notes default exists (SAFE-05)
- `remote_name`: "origin" if present, else first remote, else None (OUT-01)

Private helpers extracted: `build_full_revwalk`, `collect_warnings`, `count_signed_commits`, `commit_is_signed`, `collect_affected_annotated_tags`, `check_has_notes_ref`, `detect_remote_name`, `message_has_matching_coauthor`.

### src/tui/mod.rs
Empty Wave 2 namespace with `pub fn run(_repo: git2::Repository) -> Result<(), crate::error::AppError>` stub. Compiles cleanly. `_repo` suppresses unused-variable warning until Wave 2.

### Cargo.toml
Four new dependencies added (supply-chain-verified in Task 1):
- `ratatui = "0.30"`
- `crossterm = "0.29"`
- `nucleo = "0.5"`
- `signal-hook = "0.4"`

### tests/scan_test.rs
11 tests covering all required contracts:
- Cascade equivalence: scan count == rewrite count (RENAME-05, DROP-04)
- Cascade descendant counting (Pitfall 2 regression guard)
- Zero count on no match
- Signed commit detection via `commit_create_buffer` + `commit_signed` (SAFE-03)
- Annotated tag detection; lightweight tags excluded (SAFE-04)
- Notes ref present/absent (SAFE-05)
- Remote preference: origin > first > None (OUT-01)

## TDD Gate Compliance

- RED commit `8b5341e`: tests fail with `error[E0432]: unresolved imports` — scan API didn't exist
- GREEN commit `a2334c1`: all 11 scan tests pass; 44 total tests pass
- REFACTOR commit `3d50814`: `cargo fmt` applied; `build_full_revwalk` extracted in GREEN (no further refactor needed); all tests still pass

## Commits

| Task | Commit | Message |
|------|--------|---------|
| Task 2 (deps + skeletons) | `4fa476d` | chore(03-01): add TUI deps and scan/tui module skeletons |
| Task 3 (RED) | `8b5341e` | test(03-01): add failing scan tests for cascade equivalence and warnings |
| Task 4 (GREEN) | `a2334c1` | feat(03-01): implement git::scan with RewritePreview cascade tracking |
| Task 5 (REFACTOR) | `3d50814` | refactor(03-01): apply cargo fmt to scan.rs and scan_test.rs |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] git2 RemoteArray iterator type mismatch**
- **Found during:** Task 4 GREEN
- **Issue:** `remotes.iter()` in git2 yields `Result<Option<&str>, Error>`, not `Option<&str>`. The `Research.md` example used `.flatten()` directly which fails to type-check.
- **Fix:** Changed to `.filter_map(|r| r.ok().flatten())` — flattens both the `Result` and `Option` layers correctly.
- **Files modified:** `src/git/scan.rs`
- **Commit:** `a2334c1`

**2. [Rule 1 - Bug] Unused import `RewritePreview` in scan_test.rs**
- **Found during:** Task 4 clippy check
- **Issue:** `RewritePreview` imported in test file but only used through function return types, triggering `-D unused-imports`.
- **Fix:** Removed the import. Tests still compile because the struct is used indirectly via returned values.
- **Files modified:** `tests/scan_test.rs`
- **Commit:** `a2334c1`

### Environment Issue (pre-existing, not a code deviation)

**macOS CommandLineTools ranlib sandbox issue**
- `cargo build` fails with `ranlib: Operation not permitted` when adding new deps (because new hashes invalidate the libgit2-sys build cache).
- Root cause: `/Library/Developer/CommandLineTools/usr/bin/ranlib` is blocked by sandbox; a pre-existing `/tmp/ar-wrapper.sh` workaround uses `libtool -static` instead.
- Resolution: All `cargo build`/`cargo test`/`cargo clippy` commands run with `AR=/tmp/ar-wrapper.sh`.
- This is a pre-existing environment constraint; all build invocations in this environment require the wrapper.

### Task structure note

The prompt's success-criteria summary listed "Task 5: tui skeleton" but the PLAN.md places the tui skeleton inside Task 2. Task 5 in PLAN.md is the REFACTOR step. PLAN.md ordering was followed exactly — the tui skeleton landed in commit `4fa476d` (Task 2).

## Known Stubs

- `src/tui/mod.rs`: `pub fn run(_repo)` returns `Ok(())` — intentional placeholder. Wave 2 fills in the body with the full TUI state machine.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes at trust boundaries. The `scan_rename`/`scan_drop` functions are read-only (no commits written). The four new deps were verified at the supply-chain checkpoint (Task 1, approved before this execution resumed).

## Self-Check: PASSED
