---
phase: 02-rewrite-engine
reviewed: 2026-05-20T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src/git/rewrite.rs
  - src/git/reader.rs
  - tests/rewrite_test.rs
  - tests/common/mod.rs
findings:
  critical: 0
  warning: 6
  info: 2
  total: 8
status: clean
fixes_applied: 5
fixes_skipped: 1
---

# Phase 02: Code Review Report

**Reviewed:** 2026-05-20T00:00:00Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** clean (all actionable findings fixed)

## Summary

Four files reviewed: the core rewrite engine (`rewrite.rs`), the repo reader (`reader.rs`), the integration test suite (`rewrite_test.rs`), and the test fixture helper (`tests/common/mod.rs`).

The OID-map graph-rewrite logic is correct: topological+reverse walk guarantees parent OIDs are always resolved before children; merge parent order is preserved via index-ordered Vec; the annotated-tag recreation pattern is structurally sound. The test coverage is thorough for the happy path.

Six warnings were found, none of which individually crash the binary, but two create observable data corruption in real repositories (CRLF normalization and the reader/rewriter scope gap), and the non-UTF-8 message silent-truncation limits the tool's applicability to the subset of git history that is fully UTF-8.

---

## Warnings

### WR-01: CRLF messages trigger unintended rewrites in `drop_coauthor`

**File:** `src/git/rewrite.rs:224-226`

**Issue:** `drop_coauthor_from_message` normalizes CRLF line endings to LF (documented in the line-170 comment). The caller compares `new_msg != raw_msg` to decide whether a rewrite is needed. For any commit whose `message_raw()` contains `\r\n`, this comparison is always `true` even when no matching co-author was found, because the normalization alone makes the strings differ. Every CRLF commit gets rewritten (new OID, ref update), changing history that should be untouched.

The existing comment acknowledges the normalization side-effect but does not acknowledge the false-positive rewrite consequence.

**Fix:** After applying `drop_coauthor_from_message`, compare on a normalized copy of the original rather than the raw bytes, or — simpler and preferred — compare byte counts of dropped lines before deciding to rewrite:

```rust
// Before running the transform, count how many matching lines exist.
let lines_to_drop = raw_msg
    .lines()
    .filter(|line| {
        let trimmed = line.trim();
        if let Some(rest) = strip_coauthor_prefix(trimmed) {
            if let Some((_n, email)) = parse_coauthor_value(rest.trim()) {
                return email.eq_ignore_ascii_case(target_email);
            }
        }
        false
    })
    .count();
let message_changed = lines_to_drop > 0;
```

This avoids the CRLF false-positive without altering the transform logic.

---

### WR-02: `enumerate_authors` / `enumerate_coauthors` miss tag-only commits

**File:** `src/git/reader.rs:76-79`

**Issue:** `build_revwalk` (used by both `enumerate_authors` and `enumerate_coauthors`) only pushes `refs/heads/*`. The rewrite functions in `rewrite.rs` push both `refs/heads/*` and `refs/tags/*`. Any commit reachable only via a tag ref (e.g., a tag pointing at a commit not on any branch) is invisible to the enumeration UI but is silently rewritten by `rewrite_author` and `drop_coauthor`. The user sees no such author in the list, selects nothing, and the commit is still rewritten.

**Fix:** Mirror the rewrite walk scope in the reader:

```rust
fn build_revwalk(repo: &git2::Repository) -> Result<git2::Revwalk<'_>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("refs/heads/*")?;
    revwalk.push_glob("refs/tags/*")?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
    Ok(revwalk)
}
```

---

### WR-03: `enumerate_coauthors` uses `commit.message()` while rewriter uses `commit.message_raw()`

**File:** `src/git/reader.rs:46`

**Issue:** `enumerate_coauthors` calls `commit.message()` (returns `Result<&str, Error>`; errors on non-UTF-8; pretty-prints by stripping leading newlines) while `drop_coauthor` feeds `commit.message_raw()` to `drop_coauthor_from_message`. These two functions see different views of the same commit messages:

1. Non-UTF-8 commits: `message()` returns `Err`, so those co-authors are never enumerated — but `message_raw()` also returns `Err` on non-UTF-8, so `.unwrap_or("")` makes them appear empty to the rewriter. The behavior is consistent on non-UTF-8, but the silent skip in enumeration is still surprising.
2. Messages with leading newlines: `message()` strips them; `message_raw()` does not. A co-author on a line following a stripped leading newline would appear in a different position. In practice this is unlikely, but the divergence is a maintenance footgun — future changes to either path could create silent inconsistencies.

**Fix:** Use `commit.message_raw()` in `enumerate_coauthors` to match the rewriter's source of truth:

```rust
let message = commit.message_raw().unwrap_or("");
```

---

### WR-04: Reflog message hardcoded to `"rewrite_author"` in shared helper used by `drop_coauthor`

**File:** `src/git/rewrite.rs:112, 149`

**Issue:** `update_refs_and_head` is called by both `rewrite_author` (line 88) and `drop_coauthor` (line 277), but both branch-ref and lightweight-tag-ref `set_target` calls always write `"rewrite_author"` as the reflog message. Running `drop_coauthor` produces misleading reflog entries: `git reflog` will show `"rewrite_author"` for a drop-coauthor operation.

**Fix:** Parameterize the reflog message:

```rust
fn update_refs_and_head(
    repo: &git2::Repository,
    oid_map: &HashMap<Oid, Oid>,
    reflog_msg: &str,     // <-- added
) -> Result<(), git2::Error> {
    // ...
    branch_ref.set_target(new_tip, reflog_msg)?;
    // ...
    lw_ref.set_target(new_oid, reflog_msg)?;
    // ...
}
```

Call sites:
```rust
update_refs_and_head(repo, &oid_map, "rewrite_author")?;
update_refs_and_head(repo, &oid_map, "drop_coauthor")?;
```

---

### WR-05: Non-UTF-8 commit messages are silently truncated to empty string

**File:** `src/git/rewrite.rs:68, 224`

**Issue:** Both `rewrite_author` (line 68) and `drop_coauthor` (line 224) do:
```rust
let raw_msg = commit.message_raw().unwrap_or("");
```

In git2 0.21, `message_raw()` returns `Result<&str, Error>` and fails on non-UTF-8 bytes. The `unwrap_or("")` silently replaces the message with an empty string for non-UTF-8 commits. The rewritten commit then has an empty message — data loss.

This is a real limitation: `Repository::commit()` only accepts `&str` (confirmed in git2 0.21 source at `repo.rs:1345`), so there is no byte-slice path to preserve non-UTF-8 messages through this API. The tool cannot faithfully rewrite such commits.

The correct response is to propagate an explicit error rather than silently corrupt the commit:

```rust
let raw_msg = commit.message_raw().map_err(|e| {
    crate::error::AppError::from(e)
    // or a dedicated variant: AppError::NonUtf8CommitMessage(old_oid)
})?;
```

This surfaces the limitation clearly to the caller/user instead of producing a rewritten repository with empty commit messages.

---

### WR-06: Annotated tag with non-UTF-8 name is recreated with empty name

**File:** `src/git/rewrite.rs:137`

**Issue:** `tag.name()` returns `Result<&str, Error>` in git2 0.21. The code uses `tag.name().unwrap_or("")`, which silently falls back to an empty string if the tag name is non-UTF-8. `repo.tag("", ...)` with `force=true` then creates (or overwrites) a tag object with an empty name and updates the ref. The original tag ref name (`ref_name`) retains its path component, but the tag object's embedded name field becomes `""`.

**Fix:** Propagate the error:

```rust
let tag_name = tag.name()?;
```

Since `git2::Error` is `From`-convertible by `?` in this context, this cleanly surfaces the non-UTF-8 case as an operation failure rather than silently corrupting the tag object.

---

## Info

### IN-01: `parse_coauthor_value` accepts malformed double-angle-bracket input

**File:** `src/git/reader.rs:96-97`

**Issue:** `parse_coauthor_value` uses `rfind('<')` and `rfind('>')` independently. For input `Name <<real@email>>`, `rfind('<')` finds the second `<`, and `rfind('>')` finds the last `>`, yielding `email = "real@email>"` — note the trailing `>`. The function contract says "returns None on malformed input", but it returns `Some` with a corrupt email string. Since `drop_coauthor_from_message` uses email for case-insensitive matching, this causes the match to fail silently rather than warning about malformed input.

**Fix:** After extracting email, verify it contains no `<` or `>`:

```rust
let email = value[lt + 1..gt].trim().to_string();
if email.contains('<') || email.contains('>') {
    return None;
}
```

---

### IN-02: `add_commit_with_message` in test fixture always uses the same timestamp

**File:** `tests/common/mod.rs:25`

**Issue:** `add_commit_with_message` hardcodes `Time::new(1_000_001, 0)` for all non-initial commits regardless of how many are added. Multiple calls in the same test create commits with identical timestamps. This doesn't cause test failures today (topological sort handles it), but any future assertion on `author().when().seconds()` for non-initial commits would be ambiguous about which commit is which. The initial commit (`create_fixture_repo`) uses `1_000_000`, so there is one timestamp value for all commits created via this helper.

**Suggestion:** Accept a timestamp parameter or increment a per-call counter, so each call produces a unique timestamp:

```rust
pub fn add_commit_with_message(repo: &Repository, name: &str, email: &str, message: &str, time_secs: i64) {
    let sig = Signature::new(name, email, &git2::Time::new(time_secs, 0)).unwrap();
    // ...
}
```

---

_Reviewed: 2026-05-20T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
