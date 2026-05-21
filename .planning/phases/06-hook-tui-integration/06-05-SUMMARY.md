---
phase: 06-hook-tui-integration
plan: "05"
subsystem: tui/event
tags: [tdd, regression, stash-bypass, hook-14, preflight]
dependency_graph:
  requires: [06-04]
  provides: [HOOK-14-complete, stash-bypass-regression-tests]
  affects: [src/tui/event.rs, src/tui/render.rs]
tech_stack:
  added: []
  patterns: [stash-bypass-regression, negative-preflight-assert]
key_files:
  created: []
  modified:
    - src/tui/event.rs
    - src/tui/render.rs
decisions:
  - "Reused existing make_test_app_with_stash() from 06-01; no duplicate added"
  - "cargo fmt applied to event.rs and render.rs (phase 06 touched files); separate chore commit for honest history"
metrics:
  duration: "~10 minutes"
  completed: "2026-05-21"
  tasks_completed: 2
  files_changed: 2
---

# Phase 06 Plan 05: HOOK-14 Stash-Bypass Regression Tests Summary

**One-liner:** Added two HOOK-14 regression tests proving Add and Manage flows reach their selectors on a stash repo without hitting SAFE-01/SAFE-02 preflight, completing the 12-test HOOK-14 suite; applied cargo fmt to phase 06 touched files.

## What Was Built

Two new regression tests in `src/tui/event.rs` completing the HOOK-14 test suite:

- **`test_add_hook_no_preflight_with_stash`** — constructs a repo with `refs/stash`, navigates to Add (index 2), presses Enter, asserts the result is `HookAddList` or `HookSuccess` (never `Screen::Err` containing "stash"/"Stash")
- **`test_manage_no_preflight_with_stash`** — same repo, navigates to Manage (index 3), asserts result is `HookSuccess` or `HookManageList` (never `Screen::Err` containing stash message)

Both tests reuse the existing `make_test_app_with_stash()` helper from 06-01.

Cargo fmt was applied to `src/tui/event.rs` and `src/tui/render.rs` (formatting drift from 06-04 production code).

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add HOOK-14 stash-bypass regression tests | 6b07917 | src/tui/event.rs |
| 2 | Final phase gate: cargo fmt cleanup | 4c2e1c8 | src/tui/event.rs, src/tui/render.rs |

## HOOK-14 Test Coverage Audit

All 12 HOOK-14 behaviors are now covered:

| Test behavior | Test name | Plan added |
|---|---|---|
| Main menu shows four options | test_main_menu_shows_four_options | 06-02 |
| Main menu routes Add hook | test_main_menu_routes_add_hook | 06-03 |
| Main menu routes Manage hook | test_main_menu_routes_manage_hook_empty | 06-04 |
| Add happy path | test_add_hook_happy_path | 06-03 |
| Add duplicate → already-stripped | test_add_hook_already_stripped | 06-03 |
| Manage empty state | test_main_menu_routes_manage_hook_empty | 06-04 |
| Manage remove single → updated list | test_manage_remove_single_entry | 06-04 |
| Manage remove last → hook removed | test_manage_remove_last_entry | 06-04 |
| Add on stash repo — no SAFE-01 preflight | test_add_hook_no_preflight_with_stash | **06-05** |
| Manage on stash repo — no SAFE-02 preflight | test_manage_no_preflight_with_stash | **06-05** |
| Rename still hits preflight with stash | test_rename_with_stash_repo_hits_preflight_err | 06-01 |
| Drop still hits preflight with stash | test_drop_with_stash_repo_hits_preflight_err | 06-01 |

## Verification Results

```
cargo test --lib tui::event: 49 passed (was 47 before this plan)
cargo test (full suite): 152 passed (9 suites)
cargo clippy -- -D warnings: No issues found
cargo fmt --check: clean
grep test_add_hook_no_preflight_with_stash src/tui/event.rs: line 1439
grep test_manage_no_preflight_with_stash src/tui/event.rs: line 1464
grep make_test_app_with_stash src/tui/event.rs: 4 matches (helper + 4 call sites)
```

## Deviations from Plan

**1. [Rule 2 - Auto-fix] cargo fmt applied to phase 06 touched files**
- **Found during:** Task 2 (final phase gate)
- **Issue:** `cargo fmt --check` reported formatting drift in `src/tui/event.rs` (ManageHook match arm style, base64 ternary style) and `src/tui/render.rs` (render_hook_add_list call site) — both files touched by Phase 06 prior plans
- **Fix:** Ran `cargo fmt`; committed as separate `chore(06-05)` commit for honest history
- **Files modified:** src/tui/event.rs, src/tui/render.rs
- **Commit:** 4c2e1c8

## Known Stubs

None — all HOOK-14 behaviors wired to real implementations from 06-01 through 06-04.

## Threat Surface Scan

No new network endpoints, auth paths, file access, or schema changes introduced. All changes are test code only.

## Self-Check: PASSED

- test_add_hook_no_preflight_with_stash at line 1439: FOUND
- test_manage_no_preflight_with_stash at line 1464: FOUND
- make_test_app_with_stash at line 777: FOUND
- Commit 6b07917 (test): FOUND
- Commit 4c2e1c8 (chore fmt): FOUND
- cargo test 152 passed: CONFIRMED
- cargo clippy -D warnings: CLEAN
- cargo fmt --check: CLEAN
