# Phase 2: Rewrite Engine - Pattern Map

**Mapped:** 2026-05-20
**Files analyzed:** 5 (2 new, 3 modified)
**Analogs found:** 5 / 5

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/git/rewrite.rs` | service (library API) | batch/transform | `src/git/reader.rs` | role-match |
| `tests/rewrite_test.rs` | test | batch | `tests/reader_test.rs` | exact |
| `src/git/mod.rs` | config | — | `src/git/mod.rs` (self) | exact |
| `src/git/reader.rs` | service | batch | — (modify only: visibility change) | N/A |
| `tests/common/mod.rs` | utility | batch | `tests/common/mod.rs` (self) | exact |

## Pattern Assignments

### `src/git/rewrite.rs` (service, batch/transform)

**Analog:** `src/git/reader.rs`

**Imports pattern** (`src/git/reader.rs` lines 1–2):
```rust
use git2::Sort;
use std::collections::HashMap;
```
The rewrite module needs these plus `git2::Oid`. Follow the same convention: no `use` aliases for crate-local paths — use `crate::error::AppError` inline in function signatures.

**Revwalk construction pattern** (`src/git/reader.rs` lines 75–80):
```rust
fn build_revwalk(repo: &git2::Repository) -> Result<git2::Revwalk<'_>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("refs/heads/*")?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
    Ok(revwalk)
}
```
`rewrite.rs`'s `build_rewrite_revwalk` extends this with one extra line:
```rust
revwalk.push_glob("refs/tags/*")?;  // include commits reachable only via tags
```

**Error propagation pattern** (`src/error.rs` lines 14–16 + `src/git/reader.rs` line 6):
```rust
// error.rs — Git(#[from] git2::Error) covers all git2 failures via ?
#[error("Git error: {0}")]
Git(#[from] git2::Error),

// reader.rs — public function signature shape
pub fn enumerate_authors(
    repo: &git2::Repository,
) -> Result<Vec<crate::git::types::AuthorIdentity>, crate::error::AppError> {
```
All public functions in `rewrite.rs` must use `Result<T, crate::error::AppError>` and propagate `git2::Error` via `?`. No new error variants are needed for Phase 2 — the existing `AppError::Git` covers all `git2` failures.

**For-loop over revwalk pattern** (`src/git/reader.rs` lines 10–17):
```rust
for oid in revwalk {
    let oid = oid?;
    let commit = repo.find_commit(oid)?;
    // ... process commit
}
```
`rewrite.rs` uses the same loop shape; add the OID-map insert after `repo.commit()`.

**Reuse of trailer helpers from `reader.rs`** — `strip_coauthor_prefix` and `parse_coauthor_value` are private today and must become `pub(crate)` before `rewrite.rs` can call them. See the modified-file section below. `rewrite.rs` calls them directly from the same crate path without duplicating the parsing logic.

**Core rewrite algorithm pattern** (from RESEARCH.md Pattern 1):

The per-commit decision + OID-map insert forms the inner body of the revwalk loop:
```rust
let any_parent_remapped = (0..commit.parent_count())
    .any(|i| oid_map.contains_key(&commit.parent_id(i).unwrap()));

let needs_rewrite = identity_matches(&commit) || any_parent_remapped;

if needs_rewrite {
    // Step 1: collect new parent OIDs in index order — Vec preserves merge order
    let new_parent_oids: Vec<Oid> = (0..commit.parent_count())
        .map(|i| {
            let p = commit.parent_id(i).unwrap();
            *oid_map.get(&p).unwrap_or(&p)
        })
        .collect();

    // Step 2: collect owned Commit objects (must outlive step 3)
    let parent_commits: Vec<git2::Commit> = new_parent_oids
        .iter()
        .map(|oid| repo.find_commit(*oid))
        .collect::<Result<Vec<_>, _>>()?;

    // Step 3: collect references (borrows from step 2)
    let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

    let new_oid = repo.commit(
        None,           // update_ref = None — never update a ref mid-walk
        &new_author,
        &new_committer,
        raw_msg,        // use message_raw(), NOT message()
        &commit.tree()?,
        &parent_refs,
    )?;

    oid_map.insert(old_oid, new_oid);
    count += 1;
}
```

**Conditional author/committer rewrite pattern** (RESEARCH.md Pattern 2, RENAME-03):
```rust
// Rewrite committer ONLY when it matches old identity — not unconditionally
let committer_matches = orig_committer.name().unwrap_or("") == old_name
    && orig_committer.email().unwrap_or("") == old_email;

let new_committer = if committer_matches {
    git2::Signature::new(new_name, new_email, &orig_committer.when())?
} else {
    // Preserve original — .when() keeps seconds + offset_minutes byte-identical
    git2::Signature::new(
        orig_committer.name().unwrap_or(""),
        orig_committer.email().unwrap_or(""),
        &orig_committer.when(),
    )?
};
```

**Co-author trailer drop pattern** (RESEARCH.md Pattern 3, DROP-02/03):
```rust
fn drop_coauthor_from_message(message: &str, target_email: &str) -> String {
    // Check trailing newline BEFORE lines() strips it
    let had_trailing_newline = message.ends_with('\n');

    let kept: Vec<&str> = message
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if let Some(rest) = strip_coauthor_prefix(trimmed) {
                if let Some((_, email)) = parse_coauthor_value(rest.trim()) {
                    return !email.eq_ignore_ascii_case(target_email);
                }
            }
            true
        })
        .collect();

    let mut out = kept.join("\n");
    if had_trailing_newline {
        out.push('\n');
    }
    out
    // Known: CRLF → LF normalization occurs here; acceptable v1 limitation
}
```

**Ref update pass pattern** (RESEARCH.md Pattern 5):
```rust
// Branch refs — local only, never refs/remotes/*
for branch_result in repo.branches(Some(git2::BranchType::Local))? {
    let (branch, _) = branch_result?;
    let r = branch.get();
    if let Some(old_tip) = r.target() {
        let new_tip = resolve_new_tip(&oid_map, old_tip);
        if new_tip != old_tip {
            let mut branch_ref = repo.find_reference(r.name().unwrap())?;
            branch_ref.set_target(new_tip, "rewrite")?;
        }
    }
}

// Tag refs — annotated vs lightweight detection
for tag_ref in repo.references_glob("refs/tags/*")? {
    let tag_ref = tag_ref?;
    let ref_oid = tag_ref.target().unwrap();
    let obj = repo.find_object(ref_oid, None)?;
    match obj.kind() {
        Some(git2::ObjectType::Tag) => {
            // Annotated: recreate the tag object with force=true
            let tag = obj.as_tag().unwrap();
            if let Some(&new_target_oid) = oid_map.get(&tag.target_id()) {
                let new_target_obj = repo.find_object(new_target_oid, None)?;
                let tagger = tag.tagger().unwrap_or_else(|| {
                    git2::Signature::now("unknown", "unknown@unknown").unwrap()
                });
                // tag.message() returns Result<Option<&str>> — handle both layers
                let msg = tag.message().unwrap_or(Ok(None)).unwrap_or("");
                repo.tag(tag.name().unwrap_or(""), &new_target_obj, &tagger, msg, true)?;
            }
        }
        Some(git2::ObjectType::Commit) => {
            // Lightweight: update the ref target directly
            if let Some(&new_oid) = oid_map.get(&ref_oid) {
                let mut lw_ref = repo.find_reference(tag_ref.name().unwrap())?;
                lw_ref.set_target(new_oid, "rewrite")?;
            }
        }
        _ => {}
    }
}

// Detached HEAD — update after ref pass if target was rewritten
if repo.head_detached()? {
    if let Ok(head_ref) = repo.head() {
        if let Some(head_oid) = head_ref.target() {
            if let Some(&new_head_oid) = oid_map.get(&head_oid) {
                repo.set_head_detached(new_head_oid)?;
            }
        }
    }
}
```

---

### `tests/rewrite_test.rs` (test, batch)

**Analog:** `tests/reader_test.rs`

**Header pattern** (`tests/reader_test.rs` lines 1–3):
```rust
mod common;

use git_author_reformer::git::reader::{enumerate_authors, enumerate_coauthors};
```
`rewrite_test.rs` follows the same shape:
```rust
mod common;

use git_author_reformer::git::rewrite::{rewrite_author, drop_coauthor};
```

**Fixture composition pattern** (`tests/reader_test.rs` lines 17–19):
```rust
let (_dir, repo) = common::create_fixture_repo(); // Alice: 1 commit (initial)
common::add_commit_with_message(&repo, "Bob", "bob@example.com", "Bob's commit");
```
Tests compose fixtures by calling helpers in sequence — no large setup blocks. `rewrite_test.rs` adds new helpers from `common/mod.rs` for multi-branch and annotated-tag scenarios.

**Error assertion pattern** (`tests/preflight_test.rs` lines 30, 62):
```rust
assert!(
    matches!(result, Err(AppError::StashDetected)),
    "repo with refs/stash must return Err(StashDetected); got: {result:?}"
);
```

**Assertion message style** — every `assert!` and `assert_eq!` call in the existing tests includes a trailing message string. Follow the same pattern in `rewrite_test.rs`.

---

### `src/git/mod.rs` (config — one-line addition)

**Analog:** `src/git/mod.rs` (self — lines 1–3):
```rust
pub mod preflight;
pub mod reader;
pub mod types;
```
Add one line:
```rust
pub mod rewrite;
```
No other changes.

---

### `src/git/reader.rs` (modify — visibility change only)

Two private functions become `pub(crate)` so `rewrite.rs` can call them without duplication.

**Current** (`src/git/reader.rs` lines 84, 95):
```rust
fn strip_coauthor_prefix(line: &str) -> Option<&str> {
fn parse_coauthor_value(value: &str) -> Option<(String, String)> {
```

**Change to:**
```rust
pub(crate) fn strip_coauthor_prefix(line: &str) -> Option<&str> {
pub(crate) fn parse_coauthor_value(value: &str) -> Option<(String, String)> {
```

No logic changes. No other lines touched.

---

### `tests/common/mod.rs` (extend — new fixture helpers)

**Analog:** `tests/common/mod.rs` (self — lines 6–30, existing helpers)

**Existing helper shape to copy** (`tests/common/mod.rs` lines 24–30):
```rust
pub fn add_commit_with_message(repo: &Repository, name: &str, email: &str, message: &str) {
    let sig = Signature::new(name, email, &git2::Time::new(1_000_001, 0)).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .unwrap();
}
```

Three new helpers are needed, following the same signature style (no `Result`, panic on failure in test helpers):

1. **`create_branch`** — creates a new local branch pointing at a specific commit:
   ```rust
   pub fn create_branch(repo: &Repository, name: &str, target: &git2::Commit) {
       repo.branch(name, target, false).unwrap();
   }
   ```

2. **`add_merge_commit`** — creates a commit with two parents in index order (parent0, parent1):
   ```rust
   pub fn add_merge_commit(
       repo: &Repository,
       name: &str,
       email: &str,
       message: &str,
       parent0: &git2::Commit,
       parent1: &git2::Commit,
   ) {
       let sig = Signature::new(name, email, &git2::Time::new(1_000_002, 0)).unwrap();
       let tree = parent0.tree().unwrap();
       repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[parent0, parent1])
           .unwrap();
   }
   ```

3. **`create_annotated_tag`** — creates a tag object (not a lightweight tag):
   ```rust
   pub fn create_annotated_tag(repo: &Repository, name: &str, target: &git2::Commit, message: &str) {
       let tagger = Signature::new("Tagger", "tagger@example.com", &git2::Time::new(2_000_000, 0)).unwrap();
       repo.tag(name, target.as_object(), &tagger, message, false).unwrap();
   }
   ```

---

## Shared Patterns

### Error Handling
**Source:** `src/error.rs` lines 14–16
**Apply to:** `src/git/rewrite.rs` (all public functions)
```rust
#[error("Git error: {0}")]
Git(#[from] git2::Error),
```
All `git2::Error` values propagate to `AppError::Git` via `?`. No new `AppError` variants are needed for Phase 2.

### Revwalk Construction
**Source:** `src/git/reader.rs` lines 75–80
**Apply to:** `src/git/rewrite.rs` (`build_rewrite_revwalk` private helper)

Copy the private `build_revwalk` shape; extend with `push_glob("refs/tags/*")` and retain `Sort::TOPOLOGICAL | Sort::REVERSE`.

### Test Fixture Pattern
**Source:** `tests/common/mod.rs` lines 6–30
**Apply to:** `tests/rewrite_test.rs` + new helpers in `tests/common/mod.rs`

Use `TempDir + Repository::init`, `Signature::new` with fixed `Time::new` timestamps, and `repo.commit(Some("HEAD"), ...)` for setup. No external git binary, no live repos.

## No Analog Found

None. All five files have analogs in the existing codebase.

## Metadata

**Analog search scope:** `src/` and `tests/` directories
**Files scanned:** 10 (all source + test files)
**Pattern extraction date:** 2026-05-20
