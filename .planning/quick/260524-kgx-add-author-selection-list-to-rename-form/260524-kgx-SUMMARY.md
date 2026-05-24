---
phase: quick-260524-kgx
plan: "01"
subsystem: tui
tags: [rename-form, author-list, focus-model, nucleo, filtering]
dependency_graph:
  requires: []
  provides: [filterable-author-list-in-rename-form, 3-way-focus-cycle, autofill-on-select]
  affects: [src/tui/app.rs, src/tui/event.rs, src/tui/render.rs]
tech_stack:
  added: []
  patterns: [nucleo-filter-reuse, nll-clone-before-reassign, make_rename_form_screen-helper]
key_files:
  modified:
    - src/tui/app.rs
    - src/tui/event.rs
    - src/tui/render.rs
decisions:
  - "Fixed 3-way Tab cycle (Name->Email->List->Name) — no empty-list branching in toggle()"
  - "List-focused Enter autofills without submitting; text-field Enter still submits when is_complete()"
  - "Source author excluded at AuthorList->RenameForm transition using full AuthorIdentity equality"
  - "Tasks 1+2+3 committed atomically — render.rs had to compile alongside app.rs/event.rs changes"
metrics:
  duration: "~25 minutes"
  completed: "2026-05-24"
  tasks_completed: 3
  files_modified: 3
---

# Phase quick-260524-kgx Plan 01: Add Author Selection List to Rename Form Summary

**One-liner:** Filterable author list embedded in rename form with 3-way Tab focus, nucleo filtering, and autofill-without-submit semantics.

## What Was Built

The `Screen::RenameForm` variant was widened to carry a second embedded `Nucleo<AuthorIdentity>` filter engine alongside the two text fields. A new `FormField::List` variant makes Tab cycle across three targets. The embedded list excludes the source author at transition time and mirrors `render_author_list` formatting exactly.

### Changes by file

**src/tui/app.rs**
- Added `FormField::List` variant
- Rewrote `toggle()` as a fixed 3-way cycle: Name->Email->List->Name
- Widened `Screen::RenameForm` with `items`, `filter`, `matched`, `nucleo: Nucleo<AuthorIdentity>`, `selected`
- Updated `test_form_field_toggle` to assert the 3-way cycle

**src/tui/event.rs**
- `Screen::AuthorList` Enter arm: `items: _` -> `items`; builds excluded list via `items.iter().filter(|a| **a != src)` before constructing `RenameForm`
- `Screen::RenameForm` arm: destructures all new fields; routes `Char`/`Backspace` to filter when List focused, `Enter` to autofill (List focus) or submit (Name/Email focus with is_complete()), `Up`/`Down` to list navigation when List focused and non-empty
- Added `make_rename_form_screen` test helper
- Updated 6 test sites to use helper or add `..` to destructures
- Updated `test_rename_form_tab_toggles_focused_field` to 3-way cycle
- Added 5 new behavior tests encoding locked decisions

**src/tui/render.rs**
- `render()` dispatch: destructures new fields and passes to `render_rename_form`
- `render_rename_form`: extended signature + 6-zone layout (header/name/email/filter/list/footer); 3-way cursor branch for `FormField::List`; footer hint updated

## Deviations from Plan

### Deviation 1: Tasks 1+2+3 committed as a single atomic commit

**Found during:** Task 1 compile verification
**Issue:** The `render.rs` dispatch arm (`render.rs:19`) needed the widened `Screen::RenameForm` fields immediately after Task 1's `app.rs` changes — it would not compile with the 2-field destructure against the 5-field variant. Similarly, the new `Char`/`Backspace` match arms in the `RenameForm` arm had to handle `FormField::List` to satisfy Rust's exhaustive match requirement, which is also part of Task 2. Since all three tasks touch the same match arms and the changes were inherently coupled at compile time, they were implemented in one pass and committed atomically.
**Impact:** None on behavior — all planned changes were delivered. The single commit covers exactly the 3 planned files.

### Deviation 2: `CARGO_TARGET_DIR` workaround for worktree build

**Found during:** Task 1 verification
**Issue:** The worktree has its own `target/` directory without pre-compiled libgit2-sys, causing `ranlib` permission failures on macOS. The main repo's `target/` has the cached C build artifacts.
**Fix:** All build and test commands used `CARGO_TARGET_DIR=/Users/instinct/Desktop/working/git-author-reformer/target` to reuse the main repo's compiled libgit2. This is a pure build-environment workaround — no source code was changed.

## Test Results

- 101 pre-existing tests: all pass
- 5 new behavior tests added: all pass
- Total: 106 tests, 0 failures
- `cargo clippy`: no new warnings

## Known Stubs

None — the list is fully wired to the live nucleo filter engine built from the same `items` slice that `Screen::AuthorList` uses.

## Threat Flags

None — no new network endpoints, auth paths, or trust-boundary crossings. Change is confined to in-process TUI state machine.

## Self-Check: PASSED

- FOUND: src/tui/app.rs
- FOUND: src/tui/event.rs
- FOUND: src/tui/render.rs
- FOUND: commit bc1b400
