# Phase 3: TUI + Integration - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning
**Mode:** Auto-generated (discuss skipped via workflow.skip_discuss)

<domain>
## Phase Boundary

Full ratatui TUI shell wired to the git layer — both rename and drop operations end-to-end.

**Requirements**: CORE-01, RENAME-01, RENAME-02, RENAME-05, DROP-01, DROP-04, SAFE-03, SAFE-04, SAFE-05, OUT-01

**Success Criteria:**
1. Launching the tool presents a two-option main menu ("Rename an author" / "Drop a co-author") and responds to keyboard navigation
2. The rename flow shows a fuzzy-filterable author list, then a two-field free-text form (new name + new email), then a confirmation prompt showing exact affected commit count before any write
3. The drop flow shows a fuzzy-filterable co-author list, then a confirmation prompt showing exact affected commit count before any write
4. Non-blocking warnings for GPG/SSH signatures, annotated tags, and refs/notes/commits are displayed before the confirmation prompt — user can still proceed
5. After a successful rewrite, the tool shows the count of rewritten commits and a force-push reminder using the detected remote name

**Key constraints:**
- `ratatui::init()` and a SIGTERM handler (calling `ratatui::restore()`) must be the first code written — before any app logic — to prevent terminal stuck in raw mode on panic or signal
- Target author entry is a free-text two-field form (new name + new email), not a second list picker
- UI hint: yes (ratatui TUI with fuzzy-filterable lists)

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

</decisions>

<code_context>
## Existing Code Insights

Codebase context will be gathered during plan-phase research.

</code_context>

<specifics>
## Specific Ideas

No specific requirements — discuss phase skipped. Refer to ROADMAP phase description and success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — discuss phase skipped.

</deferred>
