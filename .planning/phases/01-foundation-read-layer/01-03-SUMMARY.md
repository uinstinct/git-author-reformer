---
phase: 01-foundation-read-layer
plan: 03
subsystem: git-read-layer
tags: [rust, git2, tdd, revwalk, author-enumeration, co-author-parsing]

requires:
  - 01-01

provides:
  - enumerate_authors(repo) → Vec<AuthorIdentity> sorted by commit_count desc
  - enumerate_coauthors(repo) → Vec<CoAuthorEntry> sorted by commit_count desc
  - build_revwalk private helper using refs/heads/* only
  - strip_coauthor_prefix private helper (eq_ignore_ascii_case)
  - parse_coauthor_value private helper (rfind-based, Unicode-safe)

affects:
  - RENAME-01 (Phase 3 TUI author picker backed by enumerate_authors)
  - DROP-01 (Phase 3 TUI co-author picker backed by enumerate_coauthors)

tech-stack:
  added:
    - "src/lib.rs — exposes pub mod error; pub mod git; for integration tests"
  patterns:
    - "Revwalk: push_glob('refs/heads/*') + TOPOLOGICAL|REVERSE sort — no tag refs"
    - "Author dedup: HashMap<(String,String),usize> keyed on exact (name,email) pair"
    - "Co-author parsing: line-by-line scan, eq_ignore_ascii_case prefix check, rfind angle-bracket extraction"
    - "Pitfall 2 applied: .unwrap_or('') on git2 0.21 Result<&str> returns (name, email, message)"

key-files:
  created:
    - src/lib.rs
    - tests/reader_test.rs
  modified:
    - src/git/reader.rs
    - src/main.rs
    - tests/common/mod.rs

key-decisions:
  - "git2 0.21 API change: name(), email(), message() return Result<&str, Error> not Option<&str> — .unwrap_or('') works on Result too"
  - "src/lib.rs added to expose modules for integration tests — binary crates require lib target for test imports"
  - "strip_coauthor_prefix uses eq_ignore_ascii_case (no allocation) over to_lowercase per research recommendation"
  - "parse_coauthor_value uses rfind('<') / rfind('>') for Unicode-safety per Pitfall 9"

patterns-established:
  - "Pattern: integration tests use src/lib.rs + mod common; for shared helpers"
  - "Pattern: revwalk always refs/heads/* never refs/* or refs/tags/*"

requirements-completed: []

duration: ~13min
completed: 2026-05-20
---

# Phase 01 Plan 03: Author and Co-author Enumeration Summary

**Full read layer: enumerate_authors and enumerate_coauthors via TDD — revwalk + HashMap dedup + case-insensitive co-author trailer parsing**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-05-20
- **Completed:** 2026-05-20
- **Tasks:** 2 (RED commit + GREEN commit)
- **Files modified:** 5 (reader.rs, reader_test.rs, lib.rs, main.rs, common/mod.rs)

## Accomplishments

- `enumerate_authors`: walks all branch-reachable commits, deduplicates by exact `(name, email)`, returns sorted by `commit_count` descending
- `enumerate_coauthors`: same revwalk, line-by-line message scan, case-insensitive `Co-authored-by:` prefix match, `rfind`-based name+email parser, same dedup and sort
- `build_revwalk`: private helper with `push_glob("refs/heads/*")` and `TOPOLOGICAL | REVERSE` sort
- `strip_coauthor_prefix`: `eq_ignore_ascii_case` for zero-allocation case folding
- `parse_coauthor_value`: `rfind('<')` / `rfind('>')` — Unicode-safe, returns `None` on malformed input
- All 7 reader tests pass: empty repo, counts+sort, same-name-different-email, co-author dedup, no trailers, malformed trailer skipped, two distinct sorted desc
- `cargo clippy --tests -- -D warnings` exits clean

## Task Commits

1. **Task 1 (RED):** `a5799b4` — `test(01-03): add failing tests for read-layer enumeration`
2. **Task 2 (GREEN):** `33ebc75` — `feat(01-03): implement author and co-author enumeration`

## Constraint Verification

| Constraint | Verified |
|-----------|---------|
| `push_glob("refs/heads/*")` — exactly 1 occurrence | YES |
| No `push_glob("refs/*")` or `push_glob("refs/tags/*")` | YES |
| `.unwrap_or("")` on name/email/message | YES (3 occurrences) |
| No bare `.unwrap()` on signature fields | YES (0 occurrences) |
| `eq_ignore_ascii_case` for prefix check | YES (1 occurrence) |
| No `todo!()` remaining | YES (0 occurrences) |
| All 7 reader tests pass | YES |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Added src/lib.rs**
- **Found during:** Task 1 (RED) — `use git_author_reformer::git::reader::...` failed to resolve
- **Issue:** Project had only `src/main.rs` (binary crate). Integration tests cannot import from binary crates. Plan's test template used `use git_author_reformer::...` which requires a lib target.
- **Fix:** Created `src/lib.rs` with `pub mod error; pub mod git;`. Removed `mod error; mod git;` from `main.rs` (now sourced from lib).
- **Files modified:** `src/lib.rs` (created), `src/main.rs` (removed inline mod declarations)
- **Commit:** `a5799b4`

**2. [Rule 1 - Bug] Fixed borrow error in tests/common/mod.rs**
- **Found during:** Task 1 (RED) — first compile of reader_test.rs
- **Issue:** `create_fixture_repo()` held a live borrow of `repo` through `tree` (a `git2::Tree<'_>`) when it tried to return `repo` by value. Rustc error E0505. This was a pre-existing bug never caught because `tests/common/mod.rs` was only first compiled in this plan (Plan 01-01 SUMMARY notes: "first real compile happens in Plan 02 or 03").
- **Fix:** Added `drop(tree);` before `(dir, repo)` return.
- **Files modified:** `tests/common/mod.rs`
- **Commit:** `a5799b4`

**3. [Rule 1 - Bug] git2 0.21 API: name()/email()/message() return Result not Option**
- **Found during:** Task 2 (GREEN) — compile error E0308 mismatched types
- **Issue:** Research patterns used `commit.message().unwrap_or("")` treating `message()` as `Option<&str>`. In git2 0.21, `Signature::name()`, `Signature::email()`, and `Commit::message()` all return `Result<&str, Error>`. `unwrap_or("")` works on `Result` as well as `Option`, so the fix was already correct — only the `if let Some(message) = commit.message()` pattern needed updating to use the Result directly.
- **Fix:** Removed the `if let Some(...)` pattern and used `commit.message().unwrap_or("")` directly.
- **Files modified:** `src/git/reader.rs`
- **Commit:** `33ebc75`

**4. [Rule 3 - Blocking] Worktree branch at Initial Commit (missing Plan 01-01 foundation)**
- **Found during:** Pre-execution setup
- **Issue:** Worktree branch was created at the repo's "Initial commit" (9e3280b) but Plan 01-01's foundation files (Cargo.toml, src/*, tests/common/) were committed to `main` (5e87ba9). The worktree had no source files.
- **Fix:** `git merge --ff-only main` — safe fast-forward, no branch history changed.
- **Impact:** None on final state. All files were present before any implementation started.

**5. [Rule 3 - Blocking] CLT ranlib SIP-protected (same as Plan 01-01)**
- **Found during:** Pre-execution setup (same environment issue documented in 01-01-SUMMARY)
- **Issue:** macOS CLT `ar` internally calls SIP-protected ranlib on ALL operations. Built from a different worktree directory → different cargo hash → build script runs fresh and fails.
- **Fix:** Created `/tmp/ar_collector.sh` — a deferred-ar approach: `cq*` calls collect object filenames to a `.pending_objs` sidecar; `s*` calls finalize the archive with `/usr/bin/libtool -static`. This properly handles cc-rs's batched archive construction. Added to `.cargo/config.toml` (worktree-local, not committed to main).
- **Impact:** CI unaffected (runners have full Xcode).

## Known Stubs

None — `enumerate_authors` and `enumerate_coauthors` are fully implemented. No placeholder data flows to callers.

## Threat Flags

None — this plan adds no new network endpoints, auth paths, or file access patterns. Read-only revwalk of local repo objects.

---
*Phase: 01-foundation-read-layer*
*Completed: 2026-05-20*
