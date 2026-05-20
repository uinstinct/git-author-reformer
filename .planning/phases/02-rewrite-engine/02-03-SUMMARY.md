---
phase: 02-rewrite-engine
plan: "03"
subsystem: git-rewrite-engine
tags: [tdd, drop-coauthor, git2, rewrite-engine, phase-2]
dependency_graph:
  requires: [02-02]
  provides: [drop_coauthor, drop_coauthor_from_message]
  affects: [src/git/rewrite.rs, tests/rewrite_test.rs]
tech_stack:
  added: []
  patterns: [tdd-red-green, shared-helper-extraction, option-a-refactor]
key_files:
  created: []
  modified:
    - src/git/rewrite.rs
    - tests/rewrite_test.rs
decisions:
  - "Option A: extracted update_refs_and_head as a shared private helper (Result<(), git2::Error>) used by both rewrite_author and drop_coauthor — DRY over Option B (code duplication)"
  - "drop_coauthor_from_message exposed as pub(crate) so #[cfg(test)] mod tests in the same file can access it via super::"
  - "had_trailing_newline captured before .lines() call to survive Pitfall 6 (str::lines strips trailing newline)"
metrics:
  duration: "~15 minutes"
  completed: "2026-05-20"
  tasks_completed: 2
  files_modified: 2
---

# Phase 2 Plan 03: drop-coauthor TDD — failing tests (RED) + implementation (GREEN)

**One-liner:** `drop_coauthor` with case-insensitive email matching, duplicate removal, and byte-identity preservation for all non-targeted content, implemented via `drop_coauthor_from_message` pure helper reusing reader.rs trailer parsers.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 (RED) | Write failing tests for drop_coauthor and drop_coauthor_from_message | 12cf642 | tests/rewrite_test.rs, src/git/rewrite.rs |
| 2 (GREEN) | Implement drop_coauthor and drop_coauthor_from_message | 2fdd8be | src/git/rewrite.rs, tests/rewrite_test.rs |

## What Was Built

### `drop_coauthor_from_message` (pub(crate), src/git/rewrite.rs)

Pure string-to-string transform that removes Co-authored-by trailer lines matching a target email (case-insensitive via `eq_ignore_ascii_case`). Implementation:
1. Captures `had_trailing_newline = message.ends_with('\n')` before `.lines()` (Pitfall 6 guard)
2. Filters lines via `strip_coauthor_prefix` + `parse_coauthor_value` from reader.rs (DRY, Karpathy Rule 7)
3. Rejoins with `"\n"` and conditionally re-appends `'\n'`

### `drop_coauthor` (pub, src/git/rewrite.rs)

Full graph rewrite function mirroring `rewrite_author`'s walk structure:
- Revwalk: `refs/heads/*` + `refs/tags/*` with `TOPOLOGICAL | REVERSE`
- Decision: `message_changed || any_parent_remapped`
- Preserves author/committer byte-for-byte via `Signature::new(..., &orig.when())` (DROP-03)
- Delegates ref/tag/HEAD update to extracted `update_refs_and_head` helper

### `update_refs_and_head` (private helper, src/git/rewrite.rs)

Extracted from `rewrite_author`'s Section D. Returns `Result<(), git2::Error>`; both callers convert via `?`. Retains the pre-bound-locals `repo.tag(tag_name, &new_target_obj, &tagger, tag_msg, true)?` pattern required by RENAME-04 acceptance grep gate.

### Tests Added

**Integration tests (tests/rewrite_test.rs, Tests 7-12):**
- Test 7: removes matching trailer
- Test 8: case-insensitive email match (DROP-02)
- Test 9: removes all duplicate occurrences within one commit (DROP-02)
- Test 10: preserves other co-authors in same commit (DROP-03)
- Test 11: byte-identity for body, other trailers, tree, timestamps, author, committer (DROP-03)
- Test 12: returns count 0 and leaves HEAD unchanged when no match found

**Unit tests (src/git/rewrite.rs #[cfg(test)] mod tests, U1-U6):**
- U1: removes single match
- U2: trailing newline preservation in both directions (Pitfall 6 pin)
- U3: case-insensitive email match
- U4: removes all duplicate trailer lines in one pass
- U5: preserves non-matching trailers
- U6: no-match returns input byte-identical

## Verification Results

| Check | Result |
|-------|--------|
| `cargo test --test rewrite_test` | 12 passed, 0 failed |
| `cargo test --lib` | 6 passed (drop_coauthor_from_message unit tests) |
| `cargo test --tests` | 33 passed, 0 failed |
| `cargo clippy --lib --tests -- -D warnings` | No issues |
| `cargo fmt -- --check` | Clean |
| All Phase 1 + Phase 2 tests | 33 passed (7 suites) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Empty line introduced in doc comment during initial edit**
- **Found during:** Task 2 (clippy run)
- **Issue:** The `/// Builds new author and committer signatures...` doc comment had an accidental blank line inserted by the edit, triggering `clippy::empty_line_after_doc_comments`
- **Fix:** Removed the extraneous blank line between doc comment lines
- **Files modified:** src/git/rewrite.rs
- **Commit:** 2fdd8be (fixed inline before GREEN commit)

## Decisions Made

1. **Option A (extract update_refs_and_head helper)** chosen over Option B (duplication). The six existing Plan 02-02 tests guarded the refactor; all passed after extraction. The helper returns `Result<(), git2::Error>` — callers use `?` which converts via `From<git2::Error> for AppError`.

2. **`drop_coauthor_from_message` as `pub(crate)`** — minimum visibility that allows `#[cfg(test)] mod tests` in the same file to access it via `use super::drop_coauthor_from_message` without making it part of the public API.

## TDD Gate Compliance

- RED gate: commit `12cf642` (`test(02-03):`) — both crates failed to compile with unresolved imports for `drop_coauthor` and `drop_coauthor_from_message`
- GREEN gate: commit `2fdd8be` (`feat(02-03):`) — all 18 new tests pass (12 integration + 6 unit)

## Known Stubs

None. Both `drop_coauthor` and `drop_coauthor_from_message` are fully implemented.

## Threat Flags

None. No new network endpoints, auth paths, or file access patterns introduced. The `target_email` parameter flows only into `eq_ignore_ascii_case` string comparison — no injection surface.

## Self-Check: PASSED

- `src/git/rewrite.rs` exists and contains `pub fn drop_coauthor` and `pub(crate) fn drop_coauthor_from_message`
- `tests/rewrite_test.rs` exists and contains all 12 `#[test]` functions
- Commits `12cf642` and `2fdd8be` exist in git log
- Phase 2 is complete: both engine operations (rewrite_author + drop_coauthor) are implemented and tested
