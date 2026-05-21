---
phase: 06-hook-tui-integration
plan: "01"
subsystem: tui/event
tags: [tdd, preflight, safety, hook-12]
dependency_graph:
  requires: []
  provides: [preflight-in-tui]
  affects: [src/main.rs, src/tui/event.rs]
tech_stack:
  added: []
  patterns: [preflight-gated-per-flow]
key_files:
  modified:
    - src/main.rs
    - src/tui/event.rs
    - tests/main_integration_test.rs
decisions:
  - "Keep if/else structure in MainMenu Enter handler (no match conversion); 06-02 will expand arms when AddHook/ManageHook are added"
  - "Integration tests updated to verify TTY guard reached (not preflight error) when stash/worktrees present"
metrics:
  duration: "~15 minutes"
  completed: "2026-05-21T06:14:04Z"
  tasks_completed: 2
  files_modified: 3
---

# Phase 06 Plan 01: Move Preflight into TUI Rename/Drop Branches Summary

**One-liner:** Moved SAFE-01/SAFE-02 preflight (check_stash, check_worktrees) from startup in src/main.rs into the Rename and Drop branches of the MainMenu Enter handler in src/tui/event.rs, unblocking Add and Manage flows from history-rewrite safety gates (HOOK-12).

## What Was Built

Preflight calls (`check_stash`, `check_worktrees`) moved from `src/main.rs` startup path into `src/tui/event.rs` inside the `Screen::MainMenu` Enter handler, gated to the Rename (index 0) and Drop (index 1) branches only. The TUI now starts on repos with stash entries; preflight still fires before any history-rewriting operation.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 (RED) | Add failing preflight regression tests | 76192f1 | src/tui/event.rs |
| 2 (GREEN) | Move preflight into event.rs Rename/Drop branches | 4f3af85 | src/main.rs, src/tui/event.rs, tests/main_integration_test.rs |

## Verification

```
grep -n "check_stash|check_worktrees" src/main.rs  → 0 matches
grep -n "check_stash" src/tui/event.rs             → 2 matches (lines 45, 71)
cargo test                                          → 137 passed
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated integration tests to match new startup behavior**
- **Found during:** Task 2 (GREEN)
- **Issue:** `test_binary_blocks_when_stash_ref_exists` and `test_binary_blocks_when_linked_worktree_exists` expected the binary to exit with preflight errors at startup — the behavior this plan explicitly removes. These tests were blocking the full test suite.
- **Fix:** Renamed tests to `test_binary_reaches_tty_guard_when_stash_ref_exists` and `test_binary_reaches_tty_guard_when_linked_worktree_exists`. Updated assertions: now verify the binary reaches the TTY guard ("Not an interactive terminal") and does NOT emit preflight messages at startup.
- **Files modified:** tests/main_integration_test.rs
- **Commit:** 4f3af85

## TDD Gate Compliance

| Gate | Commit | Status |
|------|--------|--------|
| RED (`test(06-01):`) | 76192f1 | PASS |
| GREEN (`feat(06-01):`) | 4f3af85 | PASS |

## Known Stubs

None — all flows wired to real preflight functions.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced.

## Self-Check: PASSED

- `src/main.rs` contains `check_stash` or `check_worktrees`: 0 matches — FOUND: 0
- `src/tui/event.rs` contains `check_stash`: 2 production call sites (lines 45, 71) — FOUND: 2
- Commit 76192f1 (RED): FOUND
- Commit 4f3af85 (GREEN): FOUND
- `cargo test` (full suite): 137 passed, 0 failed — PASSED
