# Phase 2: Rewrite Engine - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning
**Mode:** Auto-generated (discuss skipped via workflow.skip_discuss)

<domain>
## Phase Boundary

The commit cascade engine — rewrite commits across all branches with correct parent mapping, handle annotated tags, no TUI.

**Requirements**: RENAME-03, RENAME-04, DROP-02, DROP-03

**Success Criteria:**
1. After a rename operation on a fixture repo, `git log --all` shows zero occurrences of the old author identity across all branches
2. Annotated tag objects pointing at rewritten commits are recreated (not just the ref pointer), verified via `git cat-file tag <tag>` showing the new target SHA
3. Merge commit parent order is preserved byte-for-byte — `git log --first-parent` and `git bisect` produce identical results before and after rewrite
4. After a co-author drop, all other trailers, commit message bodies, trees, and timestamps are byte-identical to the originals

**Key constraints:**
- Annotated tag object recreation must occur in the same phase as branch ref updating — do not defer the tag object rewrite to Phase 3
- Merge commit parent order must be preserved by index (`commit.parent_id(i)` in 0..N order, mapped through OID table) — never use an unordered structure

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
