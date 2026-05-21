---
phase: 06-hook-tui-integration
plan: "03"
subsystem: tui
tags: [tdd, state-machine, add-hook-flow, fuzzy-selector, hook-engine]
dependency_graph:
  requires: [06-02]
  provides: [Add hook flow end-to-end, HookAddList real event arm, HookSuccess arm, HookAlreadyStripped arm, render_hook_add_list, render_hook_success, render_hook_already_stripped]
  affects: [src/tui/event.rs, src/tui/render.rs]
tech_stack:
  added: []
  patterns: [NLL borrow clone-before-assign, engine-truth re-read after install, three-zone layout, enumerate_coauthors reuse]
key_files:
  created: []
  modified:
    - src/tui/event.rs
    - src/tui/render.rs
decisions:
  - Reuse build_coauthor_nucleo/apply_coauthor_filter from app.rs for HookAddList (no new nucleo helpers)
  - Reuse enumerate_coauthors from reader.rs (HOOK-03) — AddHook branch is second callsite (Drop is first)
  - read_strip_list called twice per Add flow: once at entry (to populate current_strip header), once post-install (HOOK-11 engine-truth)
  - NotToolManaged at AddHook entry -> Screen::Err with descriptive path message (T-06-03-02 mitigation)
  - HookSuccess any-key sets should_exit (Add and Manage share this arm)
  - HookAlreadyStripped any-key returns to MainMenu { selected: 2 } (index 2 = AddHook)
metrics:
  duration: "~25m"
  completed: "2026-05-21"
  tasks_completed: 2
  files_changed: 2
---

# Phase 06 Plan 03: Add Hook Flow Implementation Summary

Replaced the 06-02 AddHook stub with a complete Add co-author auto-strip hook flow: strip list header + fuzzy co-author selector (reusing enumerate_coauthors), install_strip call, and HookSuccess/HookAlreadyStripped screens populated from engine truth (read_strip_list re-read per HOOK-11).

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | RED: Add 5 failing tests for Add flow state transitions | 285b954 | src/tui/event.rs |
| 2 | GREEN: Implement Add flow event logic and render functions | 185cbfe | src/tui/event.rs, src/tui/render.rs |

## What Was Built

### src/tui/event.rs

**AddHook branch (replaces 06-02 stub):**
- Calls `read_strip_list` to get `current_strip` (Absent -> `vec![]`, Managed -> emails, NotToolManaged -> `Screen::Err`)
- Calls `enumerate_coauthors` (HOOK-03 reuse — second callsite, Drop is first)
- Builds `HookAddList` screen using `build_coauthor_nucleo` + `apply_coauthor_filter` (no new helpers)

**HookAddList arm (replaces empty placeholder):**
- Enter: NLL-clones email from `matched.get(*selected)`, then calls `install_strip`
  - `Installed` -> `read_strip_list` re-read -> `HookSuccess` (HOOK-11 engine-truth)
  - `AlreadyStripped` -> `HookAlreadyStripped { email }`
  - `Err` -> `Screen::Err`
- Char/Backspace: update filter, re-run `apply_coauthor_filter`, reset `selected`
- Down/Up: navigate with wrap
- Esc: `MainMenu { selected: 2 }`

**HookAlreadyStripped arm:** any key -> `MainMenu { selected: 2 }`

**HookSuccess arm:** any key -> `should_exit = true`

### src/tui/render.rs

**render_hook_add_list:** Four-zone vertical layout:
- Zone 1 (`Length(4)`): "Current strip list" paragraph — emails joined with newline, or "no entries yet"
- Zone 2 (`Length(3)`): filter input with cursor positioning
- Zone 3 (`Fill(1)`): fuzzy co-author list with highlight
- Zone 4 (`Length(1)`): hint line

**render_hook_success:** Exhaustive match on HookState:
- `Absent` -> "No hook installed — no emails configured."
- `Managed { emails }` -> "Hook active — stripping N email(s): ..." with list
- `NotToolManaged(_)` -> "Error: foreign hook (should not reach this screen)."

**render_hook_already_stripped:** "Already stripped: {email}" with return-to-menu hint. Wrapped in `Block::bordered().title("No change")`.

## TDD Gate Compliance

| Gate | Commit | Message |
|------|--------|---------|
| RED | 285b954 | `test(06-03): add failing Add flow tests` |
| GREEN | 185cbfe | `feat(06-03): implement Add flow (HookAddList, HookSuccess, HookAlreadyStripped)` |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `*c` dereference error in HookAddList Char arm**
- **Found during:** Task 2 (first cargo test after implementation)
- **Issue:** Used `filter.push(*c)` but `c` in `KeyCode::Char(c)` matches a `char` value directly (not a reference), so `*c` fails with "type `char` cannot be dereferenced"
- **Fix:** Changed `filter.push(*c)` to `filter.push(c)` — matching the pattern used in the existing CoAuthorList arm
- **Files modified:** src/tui/event.rs
- **Commit:** 185cbfe (included in Task 2 commit)

## Verification Results

```
cargo test: 145 passed (9 suites, 1.43s)
grep install_strip src/tui/event.rs -> 1 source call (line 316) + 2 in test code
grep enumerate_coauthors src/tui/event.rs -> 2 source calls: Drop (line 82), AddHook (line 116)
grep read_strip_list src/tui/event.rs -> 2 source calls: entry (line 101), post-install (line 318)
```

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| `render_hook_manage_list` renders `"Manage hook (todo)"` | src/tui/render.rs | Full UI implemented in 06-04 |
| `ManageHook` Enter branch -> `Screen::Err("not yet implemented")` | src/tui/event.rs | Real logic implemented in 06-04 |

These stubs are intentional — 06-04 delivers the Manage flow.

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes introduced. The `install_strip` call is guarded by `validate_email_for_embedding` inside the hook engine (Phase 5 delivery — T-06-03-01 accepted). `NotToolManaged` at AddHook entry routes to `Screen::Err` (T-06-03-02 mitigated).

## Self-Check: PASSED
