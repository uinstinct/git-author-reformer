---
phase: 03-tui-integration
plan: "03"
subsystem: tui-rename-flow
tags: [tui, ratatui, nucleo, fuzzy-filter, rename-form, tdd, rename-01, rename-02]
dependency_graph:
  requires: [03-02]
  provides:
    - tui::app::Screen::AuthorList
    - tui::app::Screen::RenameForm
    - tui::app::Screen::Preview
    - tui::app::RenameDraft
    - tui::app::FormField
    - tui::app::PendingOp
    - tui::app::build_author_nucleo
    - tui::app::apply_filter
    - tui::render::render_author_list
    - tui::render::render_rename_form
    - tui::render::render_preview_placeholder
  affects:
    - src/tui/app.rs
    - src/tui/event.rs
    - src/tui/render.rs
tech_stack:
  added: []
  patterns:
    - nucleo-Nucleo-T-fuzzy-filter-with-injector
    - Screen-enum-state-machine-extension
    - borrow-checker-mem-take-on-transition
    - TDD-RED-GREEN-per-task
    - render-only-reads-screen-state
key_files:
  created: []
  modified:
    - src/tui/app.rs
    - src/tui/event.rs
    - src/tui/render.rs
decisions:
  - "Screen::Preview(PendingOp) without scan field — Plan 03-05 will add scan field when it implements the real preview render; variant shape change is a one-line edit"
  - "Esc from RenameForm goes to MainMenu (v1: no back-stack) — simplest correct behavior; back-stack deferred to Wave 4/5"
  - "AuthorList navigation uses only Down/Up (not j/k) — j/k must remain available as filter characters"
  - "apply_filter called only in event handlers (never in render) — render reads only pre-computed matched Vec"
  - "AR wrapper /tmp/ar-wrapper/ar used for all builds — CLT ranlib has SIP permissions issue on this machine; wrapper uses libtool -static"
metrics:
  duration: "~25 minutes"
  completed: "2026-05-20"
  tasks_completed: 3
  files_changed: 3
---

# Phase 3 Plan 03: Rename Flow (AuthorList + RenameForm) Summary

Fuzzy-filterable author list (RENAME-01) backed by nucleo, two-field free-text RenameForm with cursor visibility (RENAME-02), and a Screen::Preview placeholder that Plan 03-05 will extend with scan results and confirmation UI.

## What Was Built

### src/tui/app.rs (extended)

New Screen variants added to the existing enum:
- `Screen::AuthorList { items, filter, matched, nucleo, selected }` — owns the nucleo instance + pre-computed matched Vec
- `Screen::RenameForm { source: AuthorIdentity, draft: RenameDraft }` — two-field edit state
- `Screen::Preview(PendingOp)` — placeholder; Plan 03-05 adds scan field

New types:
- `RenameDraft { new_name, new_email, focused: FormField }` with `Default` + `is_complete()` (both fields non-empty after trim)
- `FormField { Name, Email }` with `toggle()` method
- `PendingOp { Rename { source, new_name, new_email } }` with `#[derive(Debug)]`; Drop variant added in Plan 03-04

nucleo helpers (pub):
- `build_author_nucleo(items: &[AuthorIdentity]) -> Nucleo<AuthorIdentity>` — injects display strings as `"{name} <{email}>"`
- `apply_filter(nucleo: &mut Nucleo<AuthorIdentity>, query: &str) -> Vec<AuthorIdentity>` — calls `pattern.reparse` + `tick(10)`, returns matched items

### src/tui/event.rs (extended)

`handle_key` now covers all five Screen variants exhaustively:

- `MainMenu` Enter(selected=0): calls `enumerate_authors(&app.repo)`, builds nucleo, transitions to `AuthorList`; Drop branch still goes to `NotImplemented("drop")` (Plan 03-04)
- `AuthorList`: Down/Up navigate with wrap, Char appends to filter + recomputes matched + resets selection, Backspace pops filter + recomputes, Enter transitions to RenameForm with selected author, Esc returns to MainMenu
- `RenameForm`: Tab/BackTab toggles focused field, Char appends to focused field, Backspace pops focused field, Enter transitions to Preview only when `draft.is_complete()`, Esc returns to MainMenu (v1: no back-stack)
- `Preview`: Esc/q returns to MainMenu
- Borrow-checker pattern: `source.clone()` + `mem::take` on draft fields before reassigning `app.screen` in the RenameForm Enter arm

### src/tui/render.rs (extended)

`render()` dispatcher is now exhaustive (no wildcard arm):

- `render_author_list`: 3-zone vertical layout (filter row Length 3, body Fill, footer Length 1); filter shows `/ {filter}` in bordered block; cursor set at end of filter text; list items formatted as `{:>4}  {name} <{email}>`; selected item highlighted with REVERSED + `"> "`; footer shows key bindings
- `render_rename_form`: header shows source identity; two stacked bordered fields; focused field gets BOLD border + `*` prefix in title; `set_cursor_position` called for the focused field using `chars().count()` for unicode safety; footer shows key bindings
- `render_preview_placeholder`: minimal WIP Paragraph; Plan 03-05 replaces the body

render.rs contains no calls to `enumerate_authors`, `apply_filter`, `scan_rename`, `scan_drop`, or any git2 function (verified by negative grep gate).

## TDD Gate Compliance

Task 1 TDD:
- RED commit `82aae74`: 5 failing tests for RenameDraft/FormField/nucleo (types don't exist yet)
- GREEN commit `fd7dbad`: all 5 tests pass; Screen variants + nucleo helpers implemented

Task 2 TDD:
- RED commit `6105969`: 10 failing tests for AuthorList/RenameForm/Preview event handling (stubs only)
- GREEN commit `394fab2`: all 12 tests pass (10 new + 2 pre-existing that needed implementation)

## Commits

| Task | Commit | Message |
|------|--------|---------|
| Task 1 RED | `82aae74` | test(03-03): add failing tests for RenameDraft, FormField, and nucleo wrapper |
| Task 1 GREEN | `fd7dbad` | feat(03-03): extend Screen enum with AuthorList, RenameForm, Preview + nucleo helpers |
| Task 2 RED | `6105969` | test(03-03): add failing tests for AuthorList/RenameForm/Preview event handling |
| Task 2 GREEN | `394fab2` | feat(03-03): implement AuthorList/RenameForm/Preview event dispatch |
| Task 3 | `ba606ca` | feat(03-03): implement render_author_list, render_rename_form, render_preview_placeholder |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Stub match arms needed in event.rs and render.rs for Task 1 build**
- **Found during:** Task 1 GREEN phase
- **Issue:** Adding new Screen variants to app.rs broke the exhaustive match in event.rs and render.rs, preventing the build from compiling even for the test-only phase.
- **Fix:** Added temporary stub arms `Screen::AuthorList { .. } | Screen::RenameForm { .. } | Screen::Preview(_) => {}` in both files so Task 1 could compile independently. Task 2 and 3 replaced these stubs with full implementations.
- **Files modified:** `src/tui/event.rs`, `src/tui/render.rs`
- **Commit:** `fd7dbad`

**2. [Rule 1 - Bug] Borrow checker error in make_test_app_with_commits**
- **Found during:** Task 2 RED phase
- **Issue:** `tree` borrowed from `repo`; then `App::new(repo)` tried to move `repo` while borrow was still live.
- **Fix:** Wrapped the `find_tree` + `commit` calls in a block to drop the borrow before moving `repo`.
- **Files modified:** `src/tui/event.rs` (test helper only)
- **Commit:** `6105969`

**3. [Rule 1 - Bug] Spurious deref `*c` on `char` in KeyCode::Char(c) arms**
- **Found during:** Task 2 GREEN phase
- **Issue:** Plan's pseudo-code used `*c` in `KeyCode::Char(c)` arms; `c` is already `char` (not `&char`) since `key` is passed by value, so `*c` was a type error.
- **Fix:** Removed the `*` dereferences in AuthorList and RenameForm Char arms.
- **Files modified:** `src/tui/event.rs`
- **Commit:** `394fab2`

### Task 4 (checkpoint:human-verify)

⚡ Auto-approved — AUTO_MODE active; gate is `blocking` (not `blocking-human`).

## Known Stubs

- `Screen::Preview(PendingOp)` renders a placeholder paragraph. Plan 03-05 replaces the `render_preview_placeholder` body with real scan results + confirmation UI. The variant shape will be extended to `Screen::Preview { op: PendingOp, scan: RewritePreview }` in Plan 03-05.
- `MainMenu Enter(selected=0) error path` routes to `Screen::NotImplemented("error")` on `enumerate_authors` failure. Plan 03-05 introduces `Screen::Err(String)`.

## Threat Flags

None — no new network endpoints, auth paths, or file access patterns introduced.

T-03-05 (RenameForm → git2::Signature::new): `RenameDraft::is_complete()` enforces non-empty-trimmed fields as required. Further validation deferred to git2::Signature::new at the rewrite call site in Plan 03-05 (as specified in threat register).

## Self-Check: PENDING
