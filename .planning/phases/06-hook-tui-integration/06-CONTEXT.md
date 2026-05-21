# Phase 6: Hook TUI Integration - Context

**Gathered:** 2026-05-21
**Status:** Ready for planning
**Mode:** Auto-generated (discuss skipped via workflow.skip_discuss)

<domain>
## Phase Boundary

Two new main-menu flows (Add, Manage) wired to the hook engine, with fuzzy-filterable selectors and success screens.

**Requirements:** HOOK-01, HOOK-02, HOOK-03, HOOK-09, HOOK-11, HOOK-14

**Success Criteria (from ROADMAP):**
1. Launching the tool presents a four-option main menu — "Rename an author", "Drop a co-author", "Add co-author auto-strip hook", "Manage auto-strip hook" — and responds to keyboard navigation
2. The "Manage auto-strip hook" option is always visible and selectable, even when no hook is installed; in that empty state it shows a clear "no entries configured" screen
3. Picking "Add" displays the currently-configured strip list (or "no entries yet"), then a fuzzy-filterable co-author selector reusing the same enumeration as the existing drop flow; selecting an entry hands off to the hook engine and lands on a success screen showing the resulting strip-list state
4. Picking "Manage" displays a fuzzy-filterable list of configured strip emails; selecting an entry removes it via the hook engine and lands on a success screen showing the resulting strip-list state (or "hook removed — no entries remain" when the last entry was removed)
5. Neither Add nor Manage triggers the stash/worktree pre-flight blockers — both flows reach their selectors on a repo with stash entries
6. Automated TUI/state-machine tests cover every user path: main menu routes each of the four options, Add happy path → success screen, Add duplicate → already-stripped screen, Manage empty state, Manage remove single → updated list, Manage remove last → "hook removed" screen, and a regression test verifies Add/Manage on a repo with stash entries does NOT hit the SAFE-01/SAFE-02 preflight

**Key Constraints (from ROADMAP):**
- The co-author enumeration in the Add flow must reuse the existing `enumerate_coauthors` from Phase 1, not a parallel implementation (HOOK-03).
- The Add and Manage flows must dispatch to the hook engine on a code path that bypasses the SAFE-01/SAFE-02 preflight (HOOK-12). Audit the App state machine for any unconditional preflight call before adding the new transitions.
- Success screens for both flows render the final strip-list state from the hook engine, not from a cached TUI value — the engine is the source of truth (HOOK-11).

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting. Use the ROADMAP phase goal, success criteria, and codebase conventions (TUI patterns in `src/tui/*` from Phase 3) to guide decisions.

</decisions>

<code_context>
## Existing Code Insights

Phase 5 delivered the hook engine public API:
- `git_author_reformer::hook::install_strip(repo, email) -> AddResult` (Added | AlreadyPresent)
- `git_author_reformer::hook::remove_strip(repo, email) -> RemoveResult` (Removed | NotPresent | HookRemoved)
- `git_author_reformer::hook::read_strip_list(repo) -> HookState` (NoHook | Managed(Vec<String>) | Foreign)

Phase 3 delivered the TUI scaffolding:
- `src/tui/app.rs` — `App` state machine, `Screen` enum, `PendingOp`, fuzzy filters
- `src/tui/event.rs` — `handle_key` keyboard dispatcher
- `src/tui/render.rs` — ratatui rendering
- Existing flows: Rename (author selector → form), Drop (co-author selector)

Phase 1's `enumerate_coauthors` (in `src/git/reader.rs`) is the canonical co-author enumeration that the Add flow MUST reuse.

`apply_coauthor_filter` and `build_coauthor_nucleo` in `src/tui/app.rs` already implement fuzzy selection for the existing Drop flow — the Add flow should reuse the same machinery.

The existing Rename and Drop flows trigger SAFE-01/SAFE-02 preflight (stash/worktree checks) before mutating commits. The new Add and Manage flows MUST NOT trigger this preflight — the hook installer does not rewrite history.

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond the ROADMAP success criteria and constraints — discuss phase skipped.

</specifics>

<deferred>
## Deferred Ideas

None — discuss phase skipped.

</deferred>
