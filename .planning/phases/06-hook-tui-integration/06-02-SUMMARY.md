---
phase: 06-hook-tui-integration
plan: "02"
subsystem: tui
tags: [state-machine, enum-extension, navigation, scaffold]
dependency_graph:
  requires: [06-01]
  provides: [HookAddList, HookManageList, HookSuccess, HookAlreadyStripped screen variants, MenuChoice 4-way]
  affects: [src/tui/app.rs, src/tui/event.rs, src/tui/render.rs]
tech_stack:
  added: []
  patterns: [4-way MenuChoice dispatch via from_index match, stub render arms, % 4 wrap modulus]
key_files:
  created: []
  modified:
    - src/tui/app.rs
    - src/tui/event.rs
    - src/tui/render.rs
decisions:
  - Reset worktree branch to 8121c394 (correct base) before applying changes — branch had started from an older commit without the hook module
  - Stub Enter branches for AddHook/ManageHook set Screen::Err("not yet implemented") as specified in plan
  - Diagnostic match in existing test updated to include new variants (Rule 1 — compilation blocker)
metrics:
  duration: "~20m"
  completed: "2026-05-21"
  tasks_completed: 2
  files_changed: 3
---

# Phase 06 Plan 02: App State Machine Extension Summary

Extended the App state machine with 4 new Screen variants, extended MenuChoice from 2 to 4 options, corrected the navigation modulus, added stub render arms and event dispatch — all downstream plans (06-03, 06-04) can now compile against these types.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Extend MenuChoice, Screen enum, fuzzy helpers in app.rs | 2a4143c | src/tui/app.rs |
| 2 | Update event.rs modulus, dispatch, stubs; render.rs stub arms | b2c3a87 | src/tui/event.rs, src/tui/render.rs |

## What Was Built

### src/tui/app.rs

- `MenuChoice` extended from 2 to 4 variants: `Rename`, `Drop`, `AddHook`, `ManageHook`
- `MenuChoice::from_index` updated to 4-way `match` expression
- `MenuChoice::label()` covers all four variants with exact spec strings
- `MenuChoice::all()` returns `[Self; 4]`
- Four new `Screen` variants:
  - `HookAddList { current_strip, items, filter, matched, nucleo, selected }`
  - `HookManageList { items, filter, matched, nucleo, selected }`
  - `HookSuccess { state: crate::hook::HookState }`
  - `HookAlreadyStripped { email: String }`
- `build_strip_nucleo(items: &[String]) -> Nucleo<String>` — mirrors build_coauthor_nucleo
- `apply_strip_filter(nucleo: &mut Nucleo<String>, query: &str) -> Vec<String>` — mirrors apply_coauthor_filter
- New tests: `test_menu_choice_all_has_four_items`, `test_menu_choice_labels`

### src/tui/event.rs

- `MainMenu` Down/Up modulus: `% 2` → `% 4`; Up wrap: `+ 2 - 1` → `+ 4 - 1`
- Enter dispatch refactored from `if *selected == 0 / else` to `match MenuChoice::from_index(sel)`
- `AddHook` and `ManageHook` branches: `Screen::Err("not yet implemented")` stubs
- Placeholder arms for all 4 new Screen variants in `handle_key` (empty body)
- `MenuChoice` added to imports
- Test renamed: `test_main_menu_down_increments_selected_mod_2` → `test_main_menu_down_increments_selected`
- Wrap assertions updated: Down from 3 wraps to 0; Up from 0 wraps to 3
- New test: `test_main_menu_shows_four_options`
- Existing diagnostic match in `test_rename_form_enter_calls_scan_...` extended with 4 new arms (Rule 1 fix)

### src/tui/render.rs

- `HookState` import added
- `render()` match extended with 4 new arms calling stub render functions
- Stub functions added (minimal `Paragraph::new("... (todo)")` bodies):
  - `render_hook_add_list`
  - `render_hook_manage_list`
  - `render_hook_success`
  - `render_hook_already_stripped`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Worktree branch was on incorrect base**
- **Found during:** Task 1 (first cargo test run)
- **Issue:** Worktree branch started from `0763079` ("update readme"), which predates the `hook` module being added to `lib.rs`. `crate::hook::HookState` in the new `Screen::HookSuccess` variant failed to resolve.
- **Fix:** Saved changes as patch, ran `git reset --hard 8121c394e3160277c33c158653e1929cb9b29fe1` to align worktree branch with the required base (which has `pub mod hook` in lib.rs), then reapplied all changes.
- **Files modified:** None (branch reset operation)

**2. [Rule 1 - Bug] Non-exhaustive diagnostic match in existing test**
- **Found during:** Task 2 (cargo test after adding new Screen variants)
- **Issue:** `test_rename_form_enter_calls_scan_and_transitions_to_preview_with_data` contained a diagnostic `match &app.screen { ... }` with only the original 7 arms — the 4 new variants caused a compile error.
- **Fix:** Added 4 new arms to the diagnostic match (`HookAddList`, `HookManageList`, `HookSuccess`, `HookAlreadyStripped`).
- **Files modified:** src/tui/event.rs
- **Commit:** b2c3a87 (included in Task 2 commit)

## Verification Results

```
cargo test: 140 passed (9 suites, 1.78s)
grep -c "% 2" src/tui/event.rs → 0 (correct)
grep -c "HookAddList|..." src/tui/render.rs → 4 (correct, >= 4)
grep "mod_2" src/tui/event.rs → no match (old test name removed)
```

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| `render_hook_add_list` renders `"Add hook (todo)"` | src/tui/render.rs | Full UI implemented in 06-03 |
| `render_hook_manage_list` renders `"Manage hook (todo)"` | src/tui/render.rs | Full UI implemented in 06-04 |
| `render_hook_success` renders `"Hook success (todo)"` | src/tui/render.rs | Full UI implemented in 06-03/06-04 |
| `render_hook_already_stripped` renders `"Already stripped (todo)"` | src/tui/render.rs | Full UI implemented in 06-03 |
| `AddHook` Enter branch → `Screen::Err("not yet implemented")` | src/tui/event.rs | Real logic implemented in 06-03 |
| `ManageHook` Enter branch → `Screen::Err("not yet implemented")` | src/tui/event.rs | Real logic implemented in 06-04 |

These stubs are intentional scaffolding — plan 06-02's goal is structural extension only.

## Self-Check: PASSED

- src/tui/app.rs modified: confirmed (68 insertions)
- src/tui/event.rs modified: confirmed (128 insertions, 52 deletions)
- src/tui/render.rs modified: confirmed
- Commit 2a4143c exists: confirmed
- Commit b2c3a87 exists: confirmed
- 140 tests pass: confirmed
