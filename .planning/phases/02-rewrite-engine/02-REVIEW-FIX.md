---
phase: 02-rewrite-engine
fixed_at: 2026-05-20T09:15:00Z
review_path: .planning/phases/02-rewrite-engine/02-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 5
skipped: 1
status: partial
---

# Phase 02: Code Review Fix Report

**Fixed at:** 2026-05-20T09:15:00Z
**Source review:** .planning/phases/02-rewrite-engine/02-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (WR-01 through WR-06)
- Fixed: 5
- Skipped: 1

## Fixed Issues

### WR-01: CRLF messages trigger unintended rewrites in `drop_coauthor`

**Files modified:** `src/git/rewrite.rs`
**Commit:** 81b4a8b
**Applied fix:** Added `let raw_msg_normalized = raw_msg.replace("\r\n", "\n")` before the comparison, so `message_changed` compares `new_msg` against the normalized original rather than the raw bytes. CRLF commits no longer trigger a false-positive rewrite when no co-author line was actually dropped.

---

### WR-03: `enumerate_coauthors` uses `commit.message()` while rewriter uses `commit.message_raw()`

**Files modified:** `src/git/reader.rs`
**Commit:** 723f7d4
**Applied fix:** Changed `commit.message().unwrap_or("")` to `commit.message_raw().unwrap_or("")` in `enumerate_coauthors`. This aligns enumeration with the rewriter's source of truth and eliminates the leading-newline divergence between the two code paths.

---

### WR-04: Reflog message hardcoded to `"rewrite_author"` in shared helper used by `drop_coauthor`

**Files modified:** `src/git/rewrite.rs`
**Commit:** 58ca924
**Applied fix:** Added `reflog_msg: &str` parameter to `update_refs_and_head`. The `rewrite_author` call site passes `"rewrite_author"` and the `drop_coauthor` call site passes `"drop_coauthor"`. Both `set_target` calls inside the helper now use `reflog_msg` instead of the hardcoded literal.

---

### WR-05: Non-UTF-8 commit messages are silently truncated to empty string

**Files modified:** `src/error.rs`, `src/git/rewrite.rs`
**Commit:** f5d2dc1
**Applied fix:** Added `AppError::NonUtf8Message(git2::Oid)` variant to `AppError` with message `"Commit {0} has a non-UTF-8 message — cannot rewrite (git2 requires valid UTF-8)"`. Both call sites in `rewrite_author` (line 68) and `drop_coauthor` (line 229) now use `.map_err(|_| AppError::NonUtf8Message(old_oid))?` instead of `.unwrap_or("")`. The tool now fails explicitly rather than silently corrupting non-UTF-8 commits.

**Note:** This is a behavior change — previously non-UTF-8 commits were silently rewritten with empty messages; now the entire operation aborts with a descriptive error. Requires human verification that the new error behavior is acceptable for the product UX.

---

### WR-06: Annotated tag with non-UTF-8 name is recreated with empty name

**Files modified:** `src/git/rewrite.rs`
**Commit:** a22f1ad
**Applied fix:** Changed `tag.name().unwrap_or("")` to `tag.name()?` in `update_refs_and_head`. Since `update_refs_and_head` returns `Result<(), git2::Error>` and `tag.name()` returns `Result<&str, git2::Error>`, the `?` operator propagates cleanly. A non-UTF-8 tag name now causes the operation to fail rather than silently creating a tag object with an empty name.

**Note:** Behavior change — previously the operation continued with a corrupted empty tag name; now it aborts. Requires human verification that aborting on non-UTF-8 tag names is acceptable.

## Skipped Issues

### WR-02: `enumerate_authors` / `enumerate_coauthors` miss tag-only commits

**File:** `src/git/reader.rs:76-79`
**Reason:** Deliberate Phase 1 design decision — the revwalk scope mismatch between reader (heads only) and rewriter (heads + tags) is documented as an accepted v1 limitation in RESEARCH.md. Fixing it would change enumeration behavior for tag-only commits, which is outside the Phase 2 scope. See RESEARCH.md for the accepted limitation entry.
**Original issue:** `build_revwalk` in `reader.rs` only pushes `refs/heads/*` while the rewrite functions also push `refs/tags/*`, meaning tag-only commits are invisible to enumeration but still rewritten.

---

**Build verification:**
- `cargo clippy --all-targets -- -D warnings`: clean (no warnings)
- `cargo test`: 33 passed, 0 failed (7 suites, 0.41s)

---

_Fixed: 2026-05-20T09:15:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
