---
phase: 06-hook-tui-integration
plan: "06"
subsystem: tui
tags: [gap-closure, sc4, hook, tdd, screen-state]
dependency_graph:
  requires: [06-04]
  provides: [Screen::HookRemoved, render_hook_removed, SC4-behavioral-contract]
  affects: [src/tui/app.rs, src/tui/event.rs, src/tui/render.rs]
tech_stack:
  added: []
  patterns: [tdd-red-green, exhaustive-match-arm, unit-variant-screen]
key_files:
  created: []
  modified:
    - src/tui/app.rs
    - src/tui/event.rs
    - src/tui/render.rs
decisions:
  - "HookRemoved is a unit variant (no fields) — the post-removal state carries no data, only identity"
  - "Any key from HookRemoved exits (should_exit = true), mirroring HookSuccess behavior"
  - "render_hook_removed uses literal em dash via \\u{2014} for portability in source"
metrics:
  duration: "~8 minutes"
  completed: "2026-05-21"
  tasks_completed: 2
  files_modified: 3
---

# Phase 06 Plan 06: HookRemoved Distinct Screen Summary

**One-liner:** Add `Screen::HookRemoved` unit variant + `render_hook_removed` so the post-deletion state shows "Hook removed — no entries remain." distinct from the never-installed "No hook installed — no emails configured." empty state.

## What Was Built

SC4 gap closure: `RemoveResult::HookDeleted` previously routed to `Screen::HookSuccess { state: HookState::Absent }`, making it indistinguishable from the never-installed state. This plan adds a dedicated `Screen::HookRemoved` variant and routes the deletion path to it.

### Changes

**src/tui/app.rs**
- Added `HookRemoved` unit variant to `Screen` enum, placed between `HookAlreadyStripped` and `Err`

**src/tui/event.rs**
- Fixed `RemoveResult::HookDeleted` arm: now sets `app.screen = Screen::HookRemoved` (was `HookSuccess { state: Absent }`)
- Added any-key arm for `Screen::HookRemoved`: sets `app.should_exit = true`
- Added `Screen::HookRemoved => "HookRemoved"` to `screen_name` helper
- Updated `test_manage_remove_last_entry`: assertion changed from `HookSuccess(Absent)` to `HookRemoved`
- Added new test `test_manage_remove_last_entry_shows_hook_removed_distinct_from_empty_state` (both paths in one function)

**src/tui/render.rs**
- Added `Screen::HookRemoved => render_hook_removed(frame, frame.area())` dispatch arm
- Added `render_hook_removed` function: renders "Hook removed — no entries remain.\n\nAny key to exit." in a bordered "Hook Removed" block

## TDD Gate Compliance

| Gate | Commit | Message |
|------|--------|---------|
| RED  | 22ec053 | `test(06-06): add failing test for HookRemoved distinct screen` |
| GREEN | 4ed83da | `feat(06-06): add Screen::HookRemoved for post-removal distinct state` |

RED gate confirmed: `cargo test` produced `error[E0599]: no variant or associated item named 'HookRemoved' found for enum 'app::Screen'`.

## Verification

```
cargo test: 153 passed (was 152 — 1 new test added)
cargo clippy --lib --tests -- -D warnings: 0 errors
cargo fmt --check: clean
```

SC4 behavioral contract satisfied:
- Post-removal path: `Screen::HookRemoved` → "Hook removed — no entries remain."
- Never-installed path: `Screen::HookSuccess { state: Absent }` → "No hook installed — no emails configured."
- The two states are now distinct screen variants — cannot be confused

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- `Screen::HookRemoved` exists in `src/tui/app.rs` line 65
- `render_hook_removed` exists in `src/tui/render.rs` line 541
- `HookDeleted → HookRemoved` routing in `src/tui/event.rs` line 393
- Any-key arm for `HookRemoved` in `src/tui/event.rs` line 430
- `screen_name` helper updated at `src/tui/event.rs` line 995
- RED commit 22ec053 exists
- GREEN commit 4ed83da exists
- 153 tests pass, 0 clippy errors, fmt clean
- No modifications to STATE.md or ROADMAP.md
