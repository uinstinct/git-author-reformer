---
phase: 06-hook-tui-integration
plan: "04"
subsystem: tui
tags: [tdd, state-machine, manage-hook-flow, fuzzy-selector, hook-engine, remove_strip]
dependency_graph:
  requires: [06-03]
  provides: [Manage hook flow end-to-end, HookManageList real event arm, remove_strip wiring, render_hook_manage_list real implementation]
  affects: [src/tui/event.rs, src/tui/render.rs]
tech_stack:
  added: []
  patterns: [NLL borrow clone-before-assign, HookDeleted direct-construct (HOOK-11 exception), three-zone layout, apply_strip_filter reuse]
key_files:
  created: []
  modified:
    - src/tui/event.rs
    - src/tui/render.rs
decisions:
  - HookDeleted path constructs HookSuccess(Absent) directly without re-read (HOOK-11 exception — hook file gone, I/O skip is safe)
  - Updated path re-reads via read_strip_list to populate HookSuccess with engine truth (HOOK-11 invariant)
  - NotToolManaged at ManageHook entry routes to Screen::Err (T-06-04-01 mitigated)
  - test_manage_remove_single_entry tests two-email repo (Updated path); test_manage_remove_last_entry tests one-email repo (HookDeleted path) — distinct branches, distinct tests
metrics:
  duration: "~15m"
  completed: "2026-05-21"
  tasks_completed: 2
  files_changed: 2
---

# Phase 06 Plan 04: Manage Hook Flow Implementation Summary

Replaced the 06-02 ManageHook stub with a complete Manage auto-strip hook flow: read_strip_list routing to HookSuccess(Absent) or HookManageList, remove_strip wiring with correct Updated/HookDeleted/NotFound routing, and render_hook_manage_list three-zone UI (filter input, strip email list, hint line).

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | RED: Add 5 failing tests for Manage flow state transitions | 238bc26 | src/tui/event.rs |
| 2 | GREEN: Implement Manage flow event logic and render function | 19583d0 | src/tui/event.rs, src/tui/render.rs |

## What Was Built

### src/tui/event.rs

**ManageHook branch (replaces 06-02 stub):**
- Calls `read_strip_list` and branches on HookState variant:
  - `Absent` -> `HookSuccess { state: Absent }` (empty-state, no list)
  - `Managed { emails }` -> `build_strip_nucleo` + `apply_strip_filter` -> `HookManageList`
  - `NotToolManaged(p)` -> `Screen::Err("Foreign hook at ... — remove or rename it first.")`
  - `Err(e)` -> `Screen::Err(e.to_string())`

**HookManageList arm (replaces empty placeholder):**
- Enter: NLL-clones email from `matched.get(*selected)`, then calls `remove_strip`
  - `Updated { .. }` -> `read_strip_list` re-read -> `HookSuccess` (HOOK-11 engine-truth)
  - `HookDeleted` -> `HookSuccess { state: HookState::Absent }` directly (HOOK-11 exception)
  - `NotFound` -> `Screen::Err("email not found in strip list (unexpected)")`
  - `Err(e)` -> `Screen::Err(e.to_string())`
- Char/Backspace: update filter, re-run `apply_strip_filter`, reset `selected`
- Down/Up: navigate with wrap (guarded by `!matched.is_empty()`)
- Esc: `MainMenu { selected: 3 }`

**Import update:** Added `apply_strip_filter` and `build_strip_nucleo` to imports (both already existed in app.rs from 06-02).

### src/tui/render.rs

**render_hook_manage_list (replaces "Manage hook (todo)" stub):** Three-zone vertical layout:
- Zone 1 (`Length(3)`): filter input "/ {filter}" with cursor positioning
- Zone 2 (`Fill(1)`): fuzzy strip email list — title "Strip list ({N} entries)" or "Strip list (empty)"; each item is the email string; highlight_style and highlight_symbol match other list screens
- Zone 3 (`Length(1)`): hint "type: filter  up/down: move  Enter: remove  Esc: back"

## TDD Gate Compliance

| Gate | Commit | Message |
|------|--------|---------|
| RED | 238bc26 | `test(06-04): add failing Manage flow tests` |
| GREEN | 19583d0 | `feat(06-04): implement Manage flow (HookManageList, remove_strip wiring)` |

## Deviations from Plan

### None

Plan executed exactly as written.

The RemoveResult variant names in the objective prompt's `<parallel_execution>` block (`Removed`/`HookRemoved`) contradicted the actual code (`Updated`/`HookDeleted`). Code and plan (06-04-PLAN.md) agreed — plan was followed. This was a documentation discrepancy, not a deviation.

The advisor recommended differentiating `test_manage_remove_single_entry` (two-email repo, Updated path) and `test_manage_remove_last_entry` (one-email repo, HookDeleted path) to pin distinct code branches. This matches the plan's intent and was applied.

## Verification Results

```
cargo test: 150 passed (9 suites, 1.76s)
grep remove_strip src/tui/event.rs -> 1 source call (line 387)
grep build_strip_nucleo src/tui/event.rs -> 2 source calls: import (line 3), ManageHook branch (line 143)
grep render_hook_manage_list src/tui/render.rs -> dispatch call (line 46) + real fn (line 438)
```

## Known Stubs

None — all stubs from 06-02/06-03 are now resolved:
- `render_hook_manage_list`: real three-zone implementation (this plan)
- `ManageHook` Enter branch: real implementation (this plan)

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes introduced. The `remove_strip` call passes the selected email string verbatim; `remove_strip` normalizes to lowercase internally. `NotToolManaged` at ManageHook entry routes to `Screen::Err` (T-06-04-01 mitigated). `NotFound` from `remove_strip` routes to `Screen::Err` as a defensive TOCTOU guard (T-06-04-02 accepted).

## Self-Check: PASSED

- src/tui/event.rs modified: confirmed (238bc26 + 19583d0)
- src/tui/render.rs modified: confirmed (19583d0)
- Commit 238bc26 exists: confirmed (RED gate)
- Commit 19583d0 exists: confirmed (GREEN gate)
- 150 tests pass: confirmed
