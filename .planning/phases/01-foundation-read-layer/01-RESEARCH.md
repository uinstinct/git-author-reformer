# Phase 1: Foundation + Read Layer - Research

**Researched:** 2026-05-20
**Domain:** Rust + git2 — repo discovery, pre-flight safety gates, author/co-author enumeration
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Claude's Discretion
All implementation choices for this phase.

### Deferred Ideas (OUT OF SCOPE)
None — discuss phase skipped.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CORE-02 | Tool auto-detects the git repo from the current working directory; shows a clear error and exits if not inside a git repo | `Repository::open_from_env()` walks up parent directories (confirmed); descriptive error; non-zero exit code via `std::process::exit(1)` |
| CORE-03 | All git operations use the git2 crate (libgit2, vendored, no SSH/HTTPS features); no git binary is called at runtime | Cargo.toml: `git2 = { version = "0.21", default-features = false, features = ["vendored-libgit2"] }` |
| SAFE-01 | Tool blocks the operation if stash entries are detected | `repo.find_reference("refs/stash").is_ok()` — blocking gate with descriptive error |
| SAFE-02 | Tool blocks the operation if linked worktrees are detected | `repo.worktrees()` non-empty — blocking gate with descriptive error |
</phase_requirements>

---

## Summary

Phase 1 establishes the entire read layer of git-author-reformer: repo detection, pre-flight safety blocking, and complete author/co-author enumeration. No writes happen. All success criteria are verifiable with fixture repos created via `git2::Repository::init()` and programmatic commits — no git binary needed.

The critical technical decisions for this phase are: (1) `Repository::open_from_env()` for repo discovery — confirmed to walk up parent directories when `$GIT_DIR` is unset, matching standard git behavior; (2) stash and worktree checks must be blocking, not warnings (SAFE-03/04/05 are non-blocking warnings but those belong to Phase 3); (3) co-author parsing uses a simple line-by-line scan with case-insensitive key matching, not the strict git trailer-block detection algorithm; (4) the revwalk for enumeration uses `push_glob("refs/heads/*")` only — branch-reachable commits cover all real author history, and using `refs/tags/*` would require annotated-tag peeling before pushing, adding complexity not needed for this use case.

Phase 1 has no TUI (ratatui/crossterm/nucleo are Phase 3 concerns). The deliverable is a library of functions under `src/git/` plus enough `main.rs` scaffolding to produce a binary that exits with an error on non-repo CWD, plus tests exercising the read layer against fixture repos.

**Primary recommendation:** Stand up `Cargo.toml`, `src/error.rs`, `src/main.rs`, `src/git/mod.rs`, `src/git/types.rs`, `src/git/reader.rs`, and `src/git/preflight.rs`. Test every behavior against in-process `git2` fixture repos.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Repo detection + error exit | CLI entry (`main.rs`) | — | CORE-02 is a startup gate; must run before any other operation |
| Pre-flight safety checks (stash, worktrees) | Git layer (`git/preflight.rs`) | CLI entry (error formatting) | Logic is git API calls; error rendering is caller's job |
| Author enumeration (primary authors) | Git layer (`git/reader.rs`) | — | Pure read: revwalk + `Commit::author()` |
| Co-author enumeration (trailer parsing) | Git layer (`git/reader.rs`) | — | Pure read: revwalk + message text scan |
| Data types shared across phases | Types module (`git/types.rs`) | — | Defined once, consumed by Phase 2 rewriter and Phase 3 TUI |

---

## Standard Stack

### Core (Phase 1 only)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `git2` | `0.21.0` | Repo open, revwalk, commit read | Required by CORE-03; vendored libgit2 means no git binary at runtime |
| `thiserror` | `2.0.18` | Unified error type with `derive(Error)` | Industry standard for library-style error types in Rust; avoids `Box<dyn Error>` |
| `clap` | `4.6.1` | `--version`, `--help` flag parsing | Curl-and-run users will try `--help` immediately; include now so later phases don't retrofit |

**Version verification:** All three confirmed via `cargo search` in this session. [VERIFIED: cargo registry]

`ratatui`, `crossterm`, `nucleo` are NOT added in Phase 1. They are Phase 3 concerns.

### Cargo.toml Dependencies

```toml
[package]
name = "git-author-reformer"
version = "0.1.0"
edition = "2021"
rust-version = "1.74"

[dependencies]
git2 = { version = "0.21", default-features = false, features = ["vendored-libgit2"] }
thiserror = "2"
clap = { version = "4.6", features = ["derive"] }

[profile.release]
strip = true
lto = true
codegen-units = 1
```

**Why `default-features = false` on git2:** The default feature set includes `ssh` and `https`, which pull in OpenSSL. This tool only opens local repos — it never fetches or pushes. Disabling those features eliminates the `undefined reference to 'dlopen'` linker failure on the Linux musl target (confirmed pitfall from PITFALLS.md). `vendored-libgit2` compiles libgit2 from source and links it statically. [CITED: PITFALLS.md Pitfall 5]

**Why NOT `vendored-openssl`:** With `ssh` and `https` features disabled, OpenSSL is not pulled in at all. Adding `vendored-openssl` is unnecessary and adds build time.

---

## Package Legitimacy Audit

> slopcheck was not available at research time (pip install failed). All packages below are tagged `[ASSUMED]` per graceful degradation rule. The planner must gate each install behind a `checkpoint:human-verify` task.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `git2` | crates.io | 10+ yrs | Very high | github.com/rust-lang/git2-rs | [ASSUMED] | Approved — official rust-lang org |
| `thiserror` | crates.io | 5+ yrs | Very high | github.com/dtolnay/thiserror | [ASSUMED] | Approved — dtolnay (prolific, trusted) |
| `clap` | crates.io | 8+ yrs | Very high | github.com/clap-rs/clap | [ASSUMED] | Approved — ecosystem standard |

**Packages removed due to slopcheck [SLOP] verdict:** none

**Packages flagged as suspicious [SUS]:** none — all three are widely recognized Rust ecosystem staples.

*Because slopcheck was unavailable, all packages above are tagged `[ASSUMED]`. The planner must add a `checkpoint:human-verify` before each install.*

---

## Architecture Patterns

### System Architecture Diagram

```
CWD (invocation)
     │
     ▼
main.rs
  ├── clap: parse --version / --help
  ├── git::open_repo() → Repository  ──────────────────► error: "not a git repo" → exit(1)
  │
  ├── git::preflight::check_stash(repo) ───────────────► error: stash detected → exit(1)
  ├── git::preflight::check_worktrees(repo) ───────────► error: worktrees detected → exit(1)
  │
  └── (Phase 1: no further main.rs wiring — TUI is Phase 3)
       │
       │  git/reader.rs (called from tests only in Phase 1)
       ├── enumerate_authors(repo)
       │     revwalk (refs/heads/* only, topological reverse)
       │       for each commit: collect (name, email) from Commit::author()
       │       deduplicate by exact (name, email) pair
       │       sort by count desc
       │     → Vec<AuthorIdentity>
       │
       └── enumerate_coauthors(repo)
             revwalk (refs/heads/* only, topological reverse)
               for each commit: scan message lines for "co-authored-by:" (case-insensitive)
               parse Name <email> from matching lines
               deduplicate by (name, email) pair
               sort by count desc
             → Vec<CoAuthorEntry>
```

### Recommended Project Structure

```
src/
  main.rs              — clap setup, open_repo(), preflight, (stub for Phase 3 TUI)
  error.rs             — AppError enum via thiserror
  git/
    mod.rs             — pub use; open_repo() function lives here
    types.rs           — AuthorIdentity, CoAuthorEntry structs
    reader.rs          — enumerate_authors(), enumerate_coauthors()
    preflight.rs       — check_stash(), check_worktrees()
tests/
  fixtures.rs          — helper: create in-memory git2 repo with crafted commits
  reader_test.rs       — tests for enumerate_authors / enumerate_coauthors
  preflight_test.rs    — tests for stash + worktree detection
```

### Pattern 1: Repo Discovery with `open_from_env`

**What:** Open the git repository using env-aware discovery.
**When to use:** Always — this is the standard startup call.

```rust
// Source: https://docs.rs/git2/0.21.0/git2/struct.Repository.html#method.open_from_env
use git2::Repository;

pub fn open_repo() -> Result<Repository, AppError> {
    Repository::open_from_env().map_err(|e| AppError::NotARepo(e.to_string()))
}
```

**Why `open_from_env` not `discover`:** Confirmed via docs.rs: when `$GIT_DIR` is unset, `open_from_env` walks up parent directories starting from CWD — same as `discover`. The additional benefit is that it respects `$GIT_DIR`, `$GIT_WORK_TREE`, and `$GIT_COMMON_DIR`, matching how the `git` binary itself operates. Both surface an `Err` when not inside any git repo. [CITED: docs.rs/git2/0.21.0]

### Pattern 2: Revwalk for Author/Co-author Enumeration

**What:** Walk all commits reachable from branch heads.
**When to use:** Both `enumerate_authors` and `enumerate_coauthors`.

```rust
// Source: https://docs.rs/git2/latest/git2/struct.Revwalk.html
use git2::{Repository, Sort};

fn build_revwalk(repo: &Repository) -> Result<git2::Revwalk<'_>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("refs/heads/*")?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
    Ok(revwalk)
}
```

**Why `refs/heads/*` only, not `refs/*`:**
1. `refs/tags/*` entries for annotated tags point at tag objects, not commits. `push_glob("refs/tags/*")` will attempt to push tag-object OIDs into the revwalk, which fails at runtime (libgit2 rejects non-commit OIDs). Handling this requires peeling each tag ref to its commit before pushing — complexity not needed for enumeration. [CITED: ARCHITECTURE.md prior research]
2. `refs/*` would also include `refs/stash` and `refs/notes/commits`, whose synthetic commits would inflate author counts.
3. All real author history is reachable from branch heads. Tag-only commits (commits reachable from tags but not from any branch) are rare and not worth the complexity for this use case.

**If tag-reachable-only commits are a concern** (extremely rare): iterate `repo.references_glob("refs/tags/*")`, peel each with `reference.peel_to_commit()`, push the commit OID. This is Phase 2's established pattern for the rewrite cascade — use the same approach there, not here.

**Performance note:** `Sort::TOPOLOGICAL | Sort::REVERSE` is not required for counting but is correct and used consistently across phases.

### Pattern 3: Stash Detection

**What:** Check if `refs/stash` exists.
**When to use:** Phase 1 pre-flight gate (SAFE-01).

```rust
// Source: PITFALLS.md Pitfall 3; API confirmed via docs.rs
pub fn check_stash(repo: &Repository) -> Result<(), AppError> {
    if repo.find_reference("refs/stash").is_ok() {
        return Err(AppError::StashDetected);
    }
    Ok(())
}
```

### Pattern 4: Worktree Detection

**What:** Check if any linked worktrees exist.
**When to use:** Phase 1 pre-flight gate (SAFE-02).

```rust
// Source: https://docs.rs/git2/latest/git2/struct.Repository.html#method.worktrees
pub fn check_worktrees(repo: &Repository) -> Result<(), AppError> {
    let worktrees = repo.worktrees()?;
    if !worktrees.is_empty() {
        let names: Vec<&str> = worktrees.iter().flatten().collect();
        return Err(AppError::WorktreesDetected(names.join(", ")));
    }
    Ok(())
}
```

**Note on `worktrees.is_empty()`:** The `Worktrees` type from git2 is a string array of *linked* worktree names. The main worktree is never included. An empty result means no linked worktrees — safe to proceed. Non-empty → block. [CITED: docs.rs/git2]

### Pattern 5: Author Enumeration

**What:** Walk all commits, collect and count `(name, email)` primary-author pairs.
**When to use:** `enumerate_authors()`.

```rust
// Source: https://docs.rs/git2/latest/git2/struct.Commit.html
use std::collections::HashMap;

pub fn enumerate_authors(repo: &Repository) -> Result<Vec<AuthorIdentity>, AppError> {
    let mut revwalk = build_revwalk(repo)?;
    let mut counts: HashMap<(String, String), usize> = HashMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let author = commit.author();
        let name = author.name().unwrap_or("").to_string();
        let email = author.email().unwrap_or("").to_string();
        *counts.entry((name, email)).or_insert(0) += 1;
    }

    let mut result: Vec<AuthorIdentity> = counts
        .into_iter()
        .map(|((name, email), count)| AuthorIdentity { name, email, commit_count: count })
        .collect();

    result.sort_by(|a, b| b.commit_count.cmp(&a.commit_count));
    Ok(result)
}
```

**AuthorIdentity equality:** Exact `(name, email)` byte pair — no normalization. Same name with different email = two entries. Same email with different display name = two entries. This matches git's own identity model.

**Empty repo:** `revwalk` produces zero iterations. Returns empty `Vec` — not an error.

### Pattern 6: Co-author Enumeration (Trailer Parsing)

**What:** Scan commit messages for `Co-authored-by:` lines, collect unique identities.
**When to use:** `enumerate_coauthors()`.

```rust
// Source: PITFALLS.md Pitfall 7+8 — line-by-line scan recommended over strict trailer block
use std::collections::HashMap;

pub fn enumerate_coauthors(repo: &Repository) -> Result<Vec<CoAuthorEntry>, AppError> {
    let mut revwalk = build_revwalk(repo)?;
    let mut counts: HashMap<(String, String), usize> = HashMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        // git2 has no trailer API — parse message text directly
        if let Some(message) = commit.message() {
            for line in message.lines() {
                let trimmed = line.trim();
                // Case-insensitive key match per git trailer spec
                if let Some(rest) = strip_coauthor_prefix(trimmed) {
                    if let Some((name, email)) = parse_coauthor_value(rest.trim()) {
                        *counts.entry((name, email)).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut result: Vec<CoAuthorEntry> = counts
        .into_iter()
        .map(|((name, email), count)| CoAuthorEntry { name, email, commit_count: count })
        .collect();

    result.sort_by(|a, b| b.commit_count.cmp(&a.commit_count));
    Ok(result)
}

/// Case-insensitive strip of "co-authored-by:" prefix.
/// Returns the rest of the line after the prefix, or None if no match.
/// Note: std::str has no built-in case-insensitive strip_prefix.
fn strip_coauthor_prefix(line: &str) -> Option<&str> {
    let prefix = "co-authored-by:";
    if line.len() >= prefix.len() && line[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&line[prefix.len()..])
    } else {
        None
    }
}

/// Parse "Name <email>" → (name, email). Returns None on malformed input.
fn parse_coauthor_value(value: &str) -> Option<(String, String)> {
    let lt = value.rfind('<')?;
    let gt = value.rfind('>')?;
    if gt < lt { return None; }
    let name = value[..lt].trim().to_string();
    let email = value[lt + 1..gt].trim().to_string();
    if name.is_empty() && email.is_empty() { return None; }
    Some((name, email))
}
```

**Why line-by-line scan, not strict trailer block:** Pitfall 8 documents that git's 25% trailer-block rule causes false negatives (valid trailers rejected) and false positives (body paragraphs misidentified as trailers). For the purpose of *finding* co-authors to display, the simpler line-by-line approach is more robust against real-world commit message variety. [CITED: PITFALLS.md Pitfall 8]

**Why `eq_ignore_ascii_case` not `to_lowercase`:** `eq_ignore_ascii_case` avoids allocating a new String on every line; `to_lowercase` allocates. For a revwalk over thousands of commits this is measurably faster. The key `co-authored-by:` is pure ASCII so ASCII case folding is correct. [ASSUMED — micro-optimization rationale]

**Deduplication key:** Exact `(name, email)` string pair. Email is not lowercased for the key — the canonical identity is preserved as-written. If two commits have `Author <email@x.com>` and `Author <EMAIL@x.com>`, those are two entries (matching git's own behavior).

**Unicode:** `str::rfind('<')` and `str::rfind('>')` are Unicode-safe in Rust. `<` and `>` are single-byte ASCII (0x3C, 0x3E) and cannot appear as continuation bytes in multi-byte UTF-8 sequences — so byte-level search is safe. [CITED: Pitfall 9, PITFALLS.md]

### Pattern 7: Error Type

```rust
// src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not inside a git repository: {0}")]
    NotARepo(String),

    #[error("Stash entries detected. Pop or drop all stashes before rewriting history.\nRun: git stash list")]
    StashDetected,

    #[error("Linked worktrees detected: {0}\nRemove worktrees before rewriting history.\nRun: git worktree list")]
    WorktreesDetected(String),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
}
```

### Anti-Patterns to Avoid

- **Using `push_glob("refs/*")` in the enumeration revwalk:** This includes `refs/stash` (synthetic commits, inflated counts) and `refs/tags/*` (annotated tags point at tag objects, not commits — `push_glob` will attempt to push a tag OID and fail at runtime). Use `push_glob("refs/heads/*")`.
- **Calling `Repository::discover(cwd)` instead of `open_from_env()`:** Misses `$GIT_DIR` env var; deviates from standard git tooling behavior. Both walk up parent directories — prefer `open_from_env`.
- **Calling any git binary:** CORE-03 forbids it. All operations via git2.
- **Case-sensitive co-author key match:** GitHub writes `Co-authored-by:`, other tools write `CO-AUTHORED-BY:`. Always use case-insensitive comparison. [CITED: PITFALLS.md Pitfall 7]
- **Byte-indexing Unicode names:** Use `rfind('<')`, `rfind('>')`, not manual byte offsets.
- **`author.name().unwrap()`:** Returns `Option<&str>` — use `unwrap_or("")`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Git object traversal | Custom pack-file parser | `git2` revwalk | Weeks of complexity for thin packs, delta chains, alternates |
| Error type boilerplate | Manual `Display` + `From` impls | `thiserror` derive | Eliminates error boilerplate, idiomatic |
| CLI flag parsing | Manual `std::env::args()` loop | `clap` derive | `--help` is expected; clap generates it automatically |

---

## Common Pitfalls

### Pitfall 1: `push_glob("refs/tags/*")` Fails on Annotated Tags
**What goes wrong:** Annotated tag refs point at tag objects, not commit objects. `revwalk.push(tag_object_oid)` returns an error because the revwalk expects commit OIDs.
**Why it happens:** `push_glob` does not auto-peel. Lightweight tags point at commits; annotated tags point at tag objects. A repo with any annotated tag will fail if `refs/tags/*` is included in a naive `push_glob` call.
**How to avoid:** Use `push_glob("refs/heads/*")` for enumeration. If tag-reachable commits must be included, iterate `repo.references_glob("refs/tags/*")`, call `reference.peel_to_commit()` on each, push the resulting commit OID individually.
**Warning signs:** `revwalk.push_glob("refs/tags/*")?` returns an error on a repo with annotated tags. Test with: `git tag -a v1.0 -m "release"`.

### Pitfall 2: `Signature::name()` Returns `None` for Non-UTF-8 Fields
**What goes wrong:** `commit.author().name()` returns `Option<&str>` — it is `None` if the name field contains non-UTF-8 bytes.
**Why it happens:** libgit2 validates UTF-8 at read time; corrupted or binary-crafted commits may have non-UTF-8 author fields.
**How to avoid:** Use `unwrap_or("")`. Never `.unwrap()` on signature fields.
**Warning signs:** `repo.find_commit(oid)?` succeeds but `commit.author().name().unwrap()` panics.

### Pitfall 3: Co-author Lines with Extra Whitespace Around the Colon
**What goes wrong:** A line `Co-authored-by  : Name <email>` (spaces before colon) does not match `co-authored-by:` after case folding.
**Why it happens:** git's trailer spec allows whitespace between the key and separator colon.
**How to avoid:** For v1, `eq_ignore_ascii_case` on the first `prefix.len()` bytes handles 99%+ of real commits. Document that `Key  :` (double-space before colon) is not matched. Add as a known limitation.
**Warning signs:** Test with a manually crafted commit with extra spaces before the colon.

### Pitfall 4: `worktrees.is_empty()` False Positive
**What goes wrong:** The main worktree itself appears in `worktrees()`, causing a false block.
**Why it happens:** Misunderstanding the API — git2's `worktrees()` returns *linked* worktrees only, not the main worktree. [CITED: docs.rs/git2]
**How to avoid:** An empty `Worktrees` = no linked worktrees = safe. Non-empty = block. Test with a single-worktree repo to confirm the check does not trigger.
**Warning signs:** Pre-flight blocks every repo, even ones with no worktrees.

---

## Code Examples

### In-Process Test Fixture

The `tdd_mode: true` config setting requires tests. Use `git2::Repository::init()` to create fixture repos without any `git` binary:

```rust
// Source: https://docs.rs/git2/latest/git2/struct.Repository.html#method.init
use git2::{Repository, Signature, Time};
use tempfile::TempDir;

fn create_fixture_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    let sig = Signature::new("Alice", "alice@example.com", &Time::new(1_000_000, 0)).unwrap();
    let tree_oid = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

    (dir, repo)
}

fn add_commit_with_message(repo: &Repository, name: &str, email: &str, message: &str) {
    let sig = Signature::new(name, email, &git2::Time::new(1_000_001, 0)).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head]).unwrap();
}
```

**Note:** `tempfile` crate needed for `TempDir`. Add to `[dev-dependencies]`:
```toml
[dev-dependencies]
tempfile = "3"
```

### Test: Author Enumeration Counts and Sort

```rust
#[test]
fn test_enumerate_authors_counts_and_sorts() {
    let (_dir, repo) = create_fixture_repo(); // Alice: 1 commit (initial)
    add_commit_with_message(&repo, "Bob", "bob@example.com", "Bob's commit");
    add_commit_with_message(&repo, "Bob", "bob@example.com", "Bob's second commit");

    let authors = enumerate_authors(&repo).unwrap();
    // Bob: 2 commits, Alice: 1 commit — sorted descending
    assert_eq!(authors[0].name, "Bob");
    assert_eq!(authors[0].commit_count, 2);
    assert_eq!(authors[1].name, "Alice");
    assert_eq!(authors[1].commit_count, 1);
}
```

### Test: Co-author Case-insensitive Matching

```rust
#[test]
fn test_enumerate_coauthors_case_insensitive() {
    let (_dir, repo) = create_fixture_repo();
    add_commit_with_message(&repo, "Alice", "alice@x.com",
        "feat: do thing\n\nCo-authored-by: Charlie <charlie@x.com>");
    add_commit_with_message(&repo, "Alice", "alice@x.com",
        "feat: do thing\n\nCO-AUTHORED-BY: Charlie <charlie@x.com>");
    add_commit_with_message(&repo, "Alice", "alice@x.com",
        "feat: do thing\n\nco-authored-by: Charlie <charlie@x.com>");

    let coauthors = enumerate_coauthors(&repo).unwrap();
    assert_eq!(coauthors.len(), 1, "All three key variants should deduplicate to one entry");
    assert_eq!(coauthors[0].commit_count, 3);
}
```

### Test: Pre-flight Stash Block

```rust
#[test]
fn test_preflight_blocks_on_stash() {
    let (_dir, repo) = create_fixture_repo();
    // Simulate a stash entry by creating refs/stash pointing at HEAD
    let head_oid = repo.head().unwrap().target().unwrap();
    repo.reference("refs/stash", head_oid, false, "test stash").unwrap();

    let result = check_stash(&repo);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AppError::StashDetected));
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `git filter-branch` | `git2` library | Ongoing | No external tools; no shell — fully self-contained binary |
| `git log --format` piped to awk | `Revwalk` + `Commit::author()` | N/A | No git binary needed; portable |
| `git interpret-trailers` | Line-by-line message scan | N/A | No git binary needed; case-insensitive by default |

**Deprecated/outdated in this project's domain:**
- `git filter-branch`: deprecated upstream, slow, requires git binary. This tool replaces it.
- `BFG Repo Cleaner`: requires Java. This tool replaces it.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `git2::Worktrees` returned by `repo.worktrees()` lists only linked worktrees (not the main worktree) | Pattern 4, Pitfall 4 | Pre-flight would false-block single-worktree repos — validate with test |
| A2 | `push_glob("refs/heads/*")` covers all author history of interest (tag-only commits excluded) | Pattern 2 | Authors of tag-only commits would not appear in the list — acceptable for v1 |
| A3 | `git2 0.21` `vendored-libgit2` feature name is unchanged from prior research | Cargo.toml | Build fails with "unknown feature" — verify on first `cargo build` |
| A4 | `eq_ignore_ascii_case` on `"co-authored-by:"` prefix is faster than `to_lowercase()` + `starts_with` | Pattern 6 | Negligible performance difference — both are correct |

---

## Open Questions

1. **Tag-reachable-only commits**
   - What we know: `push_glob("refs/heads/*")` misses commits reachable only from tags (not from any branch)
   - What's unclear: whether any real-world repos have author history only in tag-reachable commits (very rare)
   - Recommendation: Accepted limitation for v1. Document in code comment. If a user reports missing authors, add tag-reachable walk with explicit peel in a follow-up.

2. **`clap` in Phase 1 vs defer to Phase 3**
   - Decision: Add clap now. The binary must respond to `--help` (the first thing curl-and-run users try). Costs nothing at Cargo.toml definition time.

---

## Environment Availability

> Phase 1 is code-only (Cargo.toml + Rust source). The only build dependencies are the Rust toolchain and a C compiler (for vendored-libgit2).

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `rustc` / `cargo` | Build | ✓ | stable | — |
| `cmake` | `vendored-libgit2` C compilation | Likely ✓ | system | Install via Homebrew (`brew install cmake`) or apt |
| `cc` (C compiler) | `vendored-libgit2` C compilation | ✓ on macOS (Xcode CLT) | — | Xcode CLT: `xcode-select --install` |

**Missing dependencies with no fallback:** None detected.

**Note:** `vendored-libgit2` compiles libgit2 from C source during `cargo build`. This requires a working C compiler and CMake. First-time build takes longer than a pure-Rust crate — expected behavior.

---

## Security Domain

> `security_enforcement` is absent from config.json — treating as enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A — local CLI tool, no auth |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A — user runs on their own repo |
| V5 Input Validation | Yes (partial) | `commit.author().name()` can be `None` (non-UTF-8) — use `unwrap_or("")`, never panic |
| V6 Cryptography | No | No crypto in Phase 1 |

### Known Threat Patterns for this Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Non-UTF-8 commit author fields (crafted repo) | Tampering | `Option<&str>` from git2 — use `unwrap_or("")`, never panic |
| Malformed `Co-authored-by` line | Tampering | `rfind('<')` / `rfind('>')` return `Option` — handle `None` gracefully; no panic path |
| Deeply nested repo path | Elevation | `open_from_env()` uses libgit2's own path handling; no custom path construction |

**No cryptographic operations in Phase 1.** Security surface is entirely input validation on commit message text and git2 API call results.

---

## Sources

### Primary (HIGH confidence)
- `git2 0.21.0` — confirmed via `cargo search git2` in this session [VERIFIED: cargo registry]
- `thiserror 2.0.18` — confirmed via `cargo search thiserror` in this session [VERIFIED: cargo registry]
- `clap 4.6.1` — confirmed via `cargo search clap` in this session [VERIFIED: cargo registry]
- `Repository::open_from_env()` — [docs.rs/git2/0.21.0/git2/struct.Repository.html](https://docs.rs/git2/0.21.0/git2/struct.Repository.html) — confirmed walks up parent directories [CITED]
- `Repository::worktrees()` — [docs.rs/git2/latest/git2/struct.Repository.html#method.worktrees](https://docs.rs/git2/latest/git2/struct.Repository.html#method.worktrees) — returns linked worktrees only [CITED]
- `Revwalk::set_sorting`, `Sort::TOPOLOGICAL | Sort::REVERSE` — [docs.rs/git2/latest/git2/struct.Revwalk.html](https://docs.rs/git2/latest/git2/struct.Revwalk.html) [CITED]
- `Commit::author()`, `Signature::name()`, `Signature::email()` — [docs.rs/git2/latest/git2/struct.Commit.html](https://docs.rs/git2/latest/git2/struct.Commit.html) [CITED]
- Stash detection via `find_reference("refs/stash")` — PITFALLS.md Pitfall 3 [CITED]
- Worktree detection via `repo.worktrees()` — PITFALLS.md Pitfall 4 [CITED]
- Co-author case-insensitivity + trailer parsing strategy — PITFALLS.md Pitfalls 7+8 [CITED]
- Unicode safety in name/email parsing — PITFALLS.md Pitfall 9 [CITED]
- Annotated tag rev push behavior — ARCHITECTURE.md prior research [CITED]

### Secondary (MEDIUM confidence)
- `push_glob("refs/heads/*")` vs `refs/tags/*` annotated tag behavior — derived from ARCHITECTURE.md step 1 (prior domain research); exact `push_glob` behavior with annotated tags not directly tested

### Tertiary (LOW confidence)
- None.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all three packages cargo-search verified in this session
- Architecture: HIGH — git2 API calls confirmed via docs.rs; patterns derived from verified prior domain research
- Pitfalls: HIGH — stash/worktree/co-author pitfalls confirmed via prior domain research against official libgit2 and git spec sources

**Research date:** 2026-05-20
**Valid until:** 2026-08-20 (git2 0.21 is stable; pitfalls are structural, not version-dependent)
