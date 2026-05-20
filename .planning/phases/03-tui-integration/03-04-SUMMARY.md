---
phase: 03-tui-integration
plan: "04"
subsystem: tui-drop-flow
tags: [tui, ratatui, nucleo, fuzzy-filter, drop-flow, tdd, drop-01]
dependency_graph:
  requires: [03-03]
  provides:
    - tui::app::Screen::CoAuthorList
    - tui::app::PendingOp::Drop
    - tui::app::build_coauthor_nucleo
    - tui::app::apply_coauthor_filter
    - tui::render::render_coauthor_list
    - tui::event::CoAuthorList key handling
  affects:
    - src/tui/app.rs
    - src/tui/event.rs
    - src/tui/render.rs
tech_stack:
  added: []
  patterns:
    - nucleo-Nucleo-T-fuzzy-filter-with-injector
    - Screen-enum-state-machine-extension
    - TDD-RED-GREEN-per-task
    - render-only-reads-screen-state
key_files:
  created: []
  modified:
    - src/tui/app.rs
    - src/tui/event.rs
    - src/tui/render.rs
decisions:
  - "Duplicated nucleo helpers as build_coauthor_nucleo / apply_coauthor_filter (option a) — 14 lines of duplication is cleaner than a generic with type parameter propagating into call sites; Karpathy Rule 2"
  - "render_preview_placeholder uses explicit match on PendingOp variants — no Debug-formatted fallback; dispatcher match is exhaustive with no wildcard arm"
  - "Empty co-author list on MainMenu Enter produces CoAuthorList with matched:[] — safe no-op on Enter (plan spec: no panic on empty index)"
metrics:
  duration: "~20 minutes"
  completed: "2026-05-20"
  tasks_completed: 4
  files_changed: 3
---

# Phase 3 Plan 04: Drop Co-author Flow (CoAuthorList) Summary

Fuzzy-filterable co-author list (DROP-01) backed by nucleo with commit counts, wiring from MainMenu "Drop a co-author" directly to Screen::Preview(PendingOp::Drop), completing the PendingOp enum for Plan 03-05's scan integration.

## What Was Built

### src/tui/app.rs (extended)

New Screen variant:
- `Screen::CoAuthorList { items, filter, matched, nucleo, selected }` — mirrors AuthorList but typed for CoAuthorEntry; owns the nucleo instance + pre-computed matched Vec

New PendingOp variant:
- `PendingOp::Drop { target: CoAuthorEntry }` — completes the PendingOp enum; both variants now have `#[derive(Debug)]`

New nucleo helpers (pub):
- `build_coauthor_nucleo(items: &[CoAuthorEntry]) -> Nucleo<CoAuthorEntry>` — injects display strings as `"{name} <{email}>"`
- `apply_coauthor_filter(nucleo: &mut Nucleo<CoAuthorEntry>, query: &str) -> Vec<CoAuthorEntry>` — calls `pattern.reparse` + `tick(10)`, returns matched items

### src/tui/event.rs (extended)

`handle_key` now covers all six Screen variants exhaustively:

- `MainMenu` Enter(selected=1): calls `enumerate_coauthors(&app.repo)`, builds nucleo, transitions to `CoAuthorList`; error path goes to `NotImplemented("error")` (Plan 03-05)
- `CoAuthorList`: Down/Up navigate with wrap, Char appends to filter + recomputes matched + resets selection, Backspace pops filter + recomputes, Enter transitions to `Screen::Preview(PendingOp::Drop { target })` if matched non-empty, Esc returns to MainMenu
- All prior arms (MainMenu, NotImplemented, AuthorList, RenameForm, Preview) unchanged

Old placeholder test `test_main_menu_enter_with_selected_1_transitions_to_drop_placeholder` updated to reflect new behavior: bare repo now produces CoAuthorList with empty items (enumerate returns Ok([])).

### src/tui/render.rs (extended)

`render()` dispatcher is exhaustive (no wildcard arm):

- `render_coauthor_list`: identical layout to render_author_list — 3-zone vertical layout (filter row Length 3, body Fill, footer Length 1); title `Co-authors ({n} match)`; same item format `{:>4}  {name} <{email}>`; same cursor placement; same key-binding footer
- `render_preview_placeholder` now explicitly pattern-matches both `PendingOp::Rename` and `PendingOp::Drop` variants, rendering a human-readable summary line in each case

render.rs contains no calls to `enumerate_authors`, `enumerate_coauthors`, `scan_rename`, `scan_drop`, or any git2 function (verified by negative grep gate).

## TDD Gate Compliance

Task 1 TDD:
- RED commit `e489bf4`: 2 failing tests for `build_coauthor_nucleo` / `apply_coauthor_filter` (functions don't exist yet — compile error)
- GREEN commit `44bb687`: all tests pass; Screen::CoAuthorList, PendingOp::Drop, nucleo helpers implemented; stub arms added to event.rs and render.rs

Task 2 TDD:
- RED commit `837cf49`: 4 failing tests for CoAuthorList event handling (stub arm does nothing at runtime)
- GREEN commit `72ab022`: all 31 lib tests pass; full CoAuthorList arm + drop branch in MainMenu

Task 4 checkpoint:
- ⚡ Auto-approved — AUTO_MODE active; gate is `blocking` (not `blocking-human`)

## Commits

| Task | Commit | Message |
|------|--------|---------|
| Task 1 RED | `e489bf4` | test(03-04): add failing tests for CoAuthorList nucleo helpers |
| Task 1 GREEN | `44bb687` | feat(03-04): extend Screen enum with CoAuthorList + PendingOp with Drop variant + nucleo helpers |
| Task 2 RED | `837cf49` | test(03-04): add failing tests for CoAuthorList/drop flow event handling |
| Task 2 GREEN | `72ab022` | feat(03-04): implement CoAuthorList event dispatch + wire MainMenu drop branch |
| Task 3 | `bb15321` | feat(03-04): implement render_coauthor_list + update preview placeholder for both PendingOp variants |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Exhaustive match breakage in event.rs and render.rs after Task 1 GREEN**
- **Found during:** Task 1 GREEN phase
- **Issue:** Adding `Screen::CoAuthorList` to app.rs broke exhaustive matches in event.rs and render.rs, preventing build from compiling.
- **Fix:** Added temporary stub arms `Screen::CoAuthorList { .. } => {}` in both files so Task 1 could compile independently. Task 2 and 3 replaced these stubs with full implementations.
- **Files modified:** `src/tui/event.rs`, `src/tui/render.rs`
- **Commit:** `44bb687`

**2. [Rule 1 - Bug] Old test `test_main_menu_enter_with_selected_1_transitions_to_drop_placeholder` tested removed behavior**
- **Found during:** Task 2 GREEN phase
- **Issue:** The test asserted `Screen::NotImplemented("drop")` which is the behavior this plan explicitly replaces. Leaving it would have caused a false regression failure.
- **Fix:** Updated test to assert `Screen::CoAuthorList { .. }` for the bare-repo case (enumerate returns Ok([]) → empty CoAuthorList).
- **Files modified:** `src/tui/event.rs` (test only)
- **Commit:** `72ab022`

## Known Stubs

- `Screen::Preview(PendingOp)` renders a placeholder paragraph. Plan 03-05 replaces `render_preview_placeholder` with real scan results + confirmation UI. The variant shape will be extended to `Screen::Preview { op: PendingOp, scan: RewritePreview }` in Plan 03-05.
- `MainMenu Enter(selected=1) error path` routes to `Screen::NotImplemented("error")` on `enumerate_coauthors` failure. Plan 03-05 introduces `Screen::Err(String)`.

## Threat Flags

None — no new network endpoints, auth paths, or file access patterns introduced beyond what the threat model already accepts (T-03-07, T-03-08: co-author strings rendered as plain ratatui text).

## Self-Check: PASSED

- src/tui/app.rs: FOUND
- src/tui/event.rs: FOUND
- src/tui/render.rs: FOUND
- 03-04-SUMMARY.md: FOUND
- All 5 task commits verified present in git log
- Full test suite: 75 passed (8 suites)
