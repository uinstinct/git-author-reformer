---
phase: 01-foundation-read-layer
type: review
status: fixes_applied
---

# Phase 1 Code Review

## Summary

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 1 | Fixed |
| WARNING | 3 | 2 Fixed, 1 False Positive |

## Findings

### CR-01 — Panic on non-ASCII input in `strip_coauthor_prefix` [FIXED]

**File:** `src/git/reader.rs`
**Severity:** CRITICAL

`line[..prefix.len()]` panics when the byte at index 15 falls in the middle of a multi-byte UTF-8 character. Any commit message with a non-ASCII character before the `co-authored-by:` prefix would crash the process.

**Fix:** Replace `line[..prefix.len()]` with `line.get(..prefix.len())?` which returns `None` instead of panicking on a non-UTF-8-boundary slice.

---

### CR-02 — Revwalk excludes commits reachable only from tags [FALSE POSITIVE]

**File:** `src/git/reader.rs`
**Severity:** WARNING → Dismissed

`push_glob("refs/heads/*")` excludes commits reachable only from annotated or lightweight tags. This is a deliberate v1 limitation documented in `01-RESEARCH.md` under "Open Questions (RESOLVED)". The decision: enumerate authors from branch tips; tag-only commits are an edge case deferred to v2. No fix required.

---

### WR-03 — Non-deterministic sort tie-breaking [FIXED]

**File:** `src/git/reader.rs` (lines 28, 61)
**Severity:** WARNING

`sort_by(|a, b| b.commit_count.cmp(&a.commit_count))` produces arbitrary ordering when two authors have the same commit count. This makes test output and UI display non-deterministic across runs.

**Fix:** Add `.then_with(|| a.name.cmp(&b.name)).then_with(|| a.email.cmp(&b.email))` as secondary/tertiary sort keys on both `enumerate_authors` and `enumerate_coauthors`.

---

### WR-04 — build.rs comment states false contract [FIXED]

**File:** `build.rs`
**Severity:** WARNING

The comment claimed `build.rs` "re-emits native link flags for libgit2 so integration test binaries can find the C library." The file only emits `cargo:rerun-if-changed=build.rs` — it does not re-emit any link flags. The comment was misleading.

**Fix:** Remove the comment entirely. The file's function is self-evident from the single `println!` it contains.

---

## Fixes Applied

All non-dismissed findings have been applied in a single commit on `main`.
