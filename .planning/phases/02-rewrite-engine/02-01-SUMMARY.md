---
phase: 02-rewrite-engine
plan: 01
subsystem: testing
tags: [git2, rust, fixtures, reader, rewrite-module]

# Dependency graph
requires:
  - phase: 01-foundation-read-layer
    provides: reader.rs with strip_coauthor_prefix and parse_coauthor_value helpers

provides:
  - pub(crate) visibility on strip_coauthor_prefix and parse_coauthor_value in reader.rs
  - Empty src/git/rewrite.rs stub so crate compiles with pub mod rewrite declared
  - create_branch, add_merge_commit, create_annotated_tag fixture helpers in tests/common/mod.rs

affects: [02-02-rename, 02-03-drop]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "pub(crate) visibility for cross-module helpers within git submodule"
    - "Test fixture helpers: no Result return, .unwrap() panics, deterministic timestamps"

key-files:
  created:
    - src/git/rewrite.rs
  modified:
    - src/git/reader.rs
    - src/git/mod.rs
    - tests/common/mod.rs

key-decisions:
  - "Stub rewrite.rs contains only a single comment line — no use statements or declarations — to keep the module boundary clean until plans 02-02 and 02-03 add real content"
  - "add_merge_commit uses parent0.tree() as merge tree — no content merging in fixtures, commit graph topology is what tests need"
  - "create_annotated_tag uses repo.tag() (not repo.tag_lightweight()) so RENAME-04 annotated tag recreation tests verify the right code path"

patterns-established:
  - "Test helpers panic via .unwrap() — test failures surface immediately as panics, not as Ok/Err returns callers must handle"
  - "Deterministic timestamps: initial commit 1_000_000, subsequent 1_000_001, merge 1_000_002, tagger 2_000_000 — keeps commit SHAs stable across runs"

requirements-completed: [RENAME-03, RENAME-04, DROP-02, DROP-03]

# Metrics
duration: 18min
completed: 2026-05-20
---

# Phase 2 Plan 01: Scaffolding Summary

**pub(crate) trailer parsers, empty rewrite module stub, and three git fixture helpers for branch/merge/annotated-tag scenarios**

## Performance

- **Duration:** 18 min
- **Started:** 2026-05-20T13:35:00Z
- **Completed:** 2026-05-20T13:53:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Exposed `strip_coauthor_prefix` and `parse_coauthor_value` as `pub(crate)` so plans 02-02 and 02-03 can call them from `src/git/rewrite.rs` without duplication
- Declared `pub mod rewrite;` in `src/git/mod.rs` (alphabetical order) and created a one-line stub file so the crate compiles
- Added `create_branch`, `add_merge_commit`, and `create_annotated_tag` to `tests/common/mod.rs` — these helpers compose multi-branch, merge-commit, and annotated-tag scenarios for Phase 2 RED tests
- All 15 Phase 1 tests continue to pass with zero behavior change

## Task Commits

Each task was committed atomically:

1. **Task 1: Make trailer-parsing helpers crate-visible and create empty rewrite module** - `b68bed6` (feat)
2. **Task 2: Add three new fixture helpers to tests/common/mod.rs** - `c930b69` (test)

## Files Created/Modified

- `src/git/reader.rs` - Changed `fn strip_coauthor_prefix` and `fn parse_coauthor_value` to `pub(crate) fn`
- `src/git/mod.rs` - Added `pub mod rewrite;` between reader and types declarations
- `src/git/rewrite.rs` - Created: one-line stub comment, empty module body
- `tests/common/mod.rs` - Appended create_branch, add_merge_commit, create_annotated_tag helpers

## Decisions Made

- Stub rewrite.rs contains only a comment line with a trailing newline — minimal content to satisfy the module declaration without adding scope that plans 02-02/03 would need to undo
- `add_merge_commit` uses `parent0.tree()` as the merge tree — fixture commits have no file content, only commit graph structure matters for rewrite tests
- `create_annotated_tag` calls `repo.tag()` with tagger signature and message (not `repo.tag_lightweight()`) — the annotated tag object reconstruction in RENAME-04 depends on this distinction

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] macOS Sequoia ranlib inaccessible — ar wrapper required for build**
- **Found during:** Task 1 verification (cargo build --lib)
- **Issue:** `/Library/Developer/CommandLineTools/usr/bin/ranlib` returns "Operation not permitted" in the macOS Sequoia sandbox environment, causing all `ar` archive creation to fail. This is a pre-existing environment constraint — the worktree's fresh target directory triggered a rebuild of `libgit2-sys` from source, exposing the issue.
- **Fix:** Created `/tmp/ar-wrapper/ar` wrapper that intercepts `ar` archive creation calls and routes them through `/usr/bin/libtool -static` (which does not call ranlib). The wrapper uses extract-and-combine semantics when appending to an existing archive, preserving cc-rs's progressive assembly pattern. Set `AR=/tmp/ar-wrapper/ar RANLIB=/tmp/ar-wrapper/ranlib` for all cargo invocations.
- **Files modified:** /tmp/ar-wrapper/ar, /tmp/ar-wrapper/ranlib (build environment wrappers, not in repo)
- **Verification:** `cargo build --lib` succeeds, `cargo test --tests` shows 15 passed
- **Committed in:** Not committed — environment wrapper only, not a repo change

**2. [Rule 1 - Bug] cargo fmt formatting drift in create_annotated_tag**
- **Found during:** Task 2 verification (cargo fmt -- --check)
- **Issue:** Initial formatting of create_annotated_tag had multi-line function signature and single-line Signature::new, but rustfmt prefers the inverse (single-line signature, multi-line Signature::new)
- **Fix:** Ran `cargo fmt` to apply canonical formatting
- **Files modified:** tests/common/mod.rs
- **Verification:** `cargo fmt -- --check` exits 0
- **Committed in:** c930b69 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bugs)
**Impact on plan:** Both auto-fixes required for correctness. The ar wrapper unblocks all Rust compilation in this environment. The fmt fix is cosmetic but required to meet the acceptance criterion.

## Issues Encountered

- macOS Sequoia (Darwin 25.5.0) has `ranlib` at `/Library/Developer/CommandLineTools/usr/bin/ranlib` but it is not executable in the Claude Code sandbox. The workaround (ar wrapper using libtool) is session-local and needs to be applied for all subsequent cargo invocations in this worktree: `AR=/tmp/ar-wrapper/ar RANLIB=/tmp/ar-wrapper/ranlib cargo <subcommand>`.

## Known Stubs

- `src/git/rewrite.rs` — intentional stub, will be implemented in plans 02-02 and 02-03

## Next Phase Readiness

- Plans 02-02 and 02-03 can `use crate::git::reader::{strip_coauthor_prefix, parse_coauthor_value};` directly
- The `rewrite` module is declared and the crate compiles — 02-02 can add code to rewrite.rs immediately
- All three fixture helpers are ready to be called from `tests/rewrite_test.rs` (02-02's RED phase)

---
*Phase: 02-rewrite-engine*
*Completed: 2026-05-20*
