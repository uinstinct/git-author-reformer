# Phase 1: Foundation + Read Layer - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning
**Mode:** Auto-generated (discuss skipped via workflow.skip_discuss)

<domain>
## Phase Boundary

Solid repo detection, author enumeration, and pre-flight safety checks with no writes

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

Success Criteria:
1. Running the binary outside a git repository exits immediately with a descriptive error message and a non-zero exit code
2. A repo containing stash entries or linked worktrees is detected at startup and blocked with a clear message — no rewrite proceeds
3. Enumerating authors on a fixture repo returns the correct Name+Email pairs with accurate per-identity commit counts, sorted by count descending
4. Enumerating co-authors parses Co-authored-by trailers case-insensitively and returns unique identities with accurate commit counts

</specifics>

<deferred>
## Deferred Ideas

None — discuss phase skipped.

</deferred>
