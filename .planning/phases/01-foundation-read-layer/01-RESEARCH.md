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
| CORE-02 | Tool auto-detects the git repo from the current working directory; shows a clear error and exits if not inside a git repo | `Repository::open_from_env()` + descriptive error; non-zero exit code via `std::process::exit(1)` |
| CORE-03 | All git operations use the git2 crate (libgit2, vendored, no SSH/HTTPS features); no git binary is called at runtime | Cargo.toml: `git2 = { version = "0.21", default-features = false, features = ["vendored-libgit2"] }` |
| SAFE-01 | Tool blocks the operation if stash entries are detected | `repo.find_reference("refs/stash").is_ok()` — blocking gate with descriptive error |
| SAFE-02 | Tool blocks the operation if linked worktrees are detected | `repo.worktrees()?.len() > 0` — blocking gate with descriptive error |
</phase_requirements>

---

## Summary

Phase 1 establishes the entire read layer of git-author-reformer: repo detection, pre-flight safety blocking, and complete author/co-author enumeration. No writes happen. All success criteria are verifiable with fixture repos created via `git2::Repository::init()` and programmatic commits — no git binary needed.

The critical technical decisions for this phase are: (1) `Repository::open_from_env()` for repo discovery (not `discover()`) because it respects `$GIT_DIR` and `$GIT_WORK_TREE` env vars the way standard git tooling does, (2) stash and worktree checks must be blocking, not warnings (SAFE-03/04/05 are non-blocking warnings but those belong to Phase 3), (3) co-author parsing uses a simple line-by-line scan with case-insensitive key matching, not the strict git trailer-block detection algorithm (more robust against real-world commit message variety).

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

**Version verification:** All three confirmed via `cargo search` in this session.

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

**Why `default-features = false` on git2:** The default feature set includes `ssh` and `https`, which pull in OpenSSL. This tool only opens local repos — it never fetches or pushes. Disabling those features eliminates the `undefined reference to 'dlopen'` linker failure on the Linux musl target (confirmed pitfall from PITFALLS.md). `vendored-libgit2` compiles libgit2 from source and links it statically.

**Why NOT `vendored-openssl`:** With `ssh` and `https` features disabled, OpenSSL is not pulled in at all. Adding `vendored-openssl` is unnecessary and adds build time. Only add it if those network features are ever enabled.

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
  ├── git::preflight::check(repo) ──────────────────────► error: stash detected → exit(1)
  │                              ──────────────────────► error: worktrees detected → exit(1)
  │
  └── (Phase 1: no further main.rs wiring — TUI is Phase 3)
       │
       │  git/reader.rs (called from tests only in Phase 1)
       ├── enumerate_authors(repo)
       │     revwalk (all refs, topological reverse)
       │       for each commit: collect (name, email) from Commit::author()
       │       deduplicate by exact (name, email) pair
       │       sort by count desc
       │     → Vec<AuthorIdentity>
       │
       └── enumerate_coauthors(repo)
             revwalk (all refs, topological reverse)
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
// Source: https://docs.rs/git2/latest/git2/struct.Repository.html#method.open_from_env
use git2::Repository;

pub fn open_repo() -> Result<Repository, AppError> {
    Repository::open_from_env().map_err(|e| AppError::NotARepo(e.to_string()))
}
```

**Why `open_from_env` not `discover`:** `open_from_env` respects `$GIT_DIR`, `$GIT_WORK_TREE`, and `$GIT_COMMON_DIR` environment variables, matching what `git` itself does. `discover(path)` is a filesystem-only upward walk without env var awareness. Since this tool targets developer workflows, honoring env vars is correct. Both will surface an `Err` when not inside a git repo — the error handling is identical.

### Pattern 2: Revwalk for Complete History

**What:** Walk all commits reachable from all refs.
**When to use:** Both `enumerate_authors` and `enumerate_coauthors`.

```rust
// Source: https://docs.rs/git2/latest/git2/struct.Revwalk.html
use git2::{Repository, Sort};

fn build_revwalk(repo: &Repository) -> Result<git2::Revwalk<'_>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("refs/*")?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
    Ok(revwalk)
}
```

**Important:** `push_glob("refs/*")` covers all branches, tags, stash, and notes. If `push_glob("*")` is used instead (no `refs/` prefix), it may also match on loose objects in some libgit2 versions and produce unexpected behavior. Use the explicit `refs/*` glob.

**Performance note:** For the enumeration use case, `Sort::TOPOLOGICAL | Sort::REVERSE` is not strictly required (we don't need ordering for counting), but it is still correct and costs nothing.

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

**Note on `worktrees.is_empty()`:** The `Worktrees` type from git2 is a string array of linked worktree names. An empty result means only the main worktree exists (which is always present). The check is: non-empty `Worktrees` → block.

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
                if let Some(rest) = trimmed.strip_prefix_ci("co-authored-by:") {
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

// Case-insensitive prefix strip (std str doesn't have this built-in)
fn has_coauthor_prefix(line: &str) -> Option<&str> {
    let lower = line.to_lowercase();
    if lower.starts_with("co-authored-by:") {
        Some(&line["co-authored-by:".len()..])
    } else {
        None
    }
}

// Parse "Name <email>" → (name, email). Returns None on malformed input.
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

**Deduplication key:** Exact `(name, email)` string pair. Email is not lowercased for the key — the canonical identity is preserved as-written. If two commits have `Author <email@x.com>` and `Author <EMAIL@x.com>`, those are two entries (matching git's own behavior).

**Unicode:** `str::rfind('<')` and `str::rfind('>')` are Unicode-safe in Rust (they search for ASCII bytes in a UTF-8 string — valid because `<` and `>` are single-byte ASCII and cannot appear as continuation bytes in multi-byte UTF-8 sequences). [CITED: Pitfall 9, PITFALLS.md]

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

- **Using `push_glob("*")` in revwalk:** Omit the `refs/` prefix and some libgit2 versions match loose objects. Use `push_glob("refs/*")` or `push_glob("refs/heads/*")` + `push_glob("refs/tags/*")` for explicit coverage.
- **Calling `repo.discover(cwd)` instead of `open_from_env()`:** Misses `$GIT_DIR` env var, deviates from standard git tooling behavior.
- **Calling any git binary:** CORE-03 forbids it. All operations via git2.
- **Case-sensitive co-author key match:** GitHub writes `Co-authored-by:`, other tools write `CO-AUTHORED-BY:`. Always lower before comparing. [CITED: PITFALLS.md Pitfall 7]
- **Byte-indexing Unicode names:** Use `rfind('<')`, `rfind('>')`, not manual byte offsets.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Git object traversal | Custom pack-file parser | `git2` revwalk | Weeks of complexity for thin packs, delta chains, alternates |
| Error type boilerplate | Manual `Display` + `From` impls | `thiserror` derive | Eliminates error boilerplate, idiomatic |
| CLI flag parsing | Manual `std::env::args()` loop | `clap` derive | `--help` is expected; clap generates it automatically |

---

## Common Pitfalls

### Pitfall 1: Revwalk Returns Old OIDs from Stash / Notes
**What goes wrong:** `push_glob("refs/*")` includes `refs/stash` and `refs/notes/commits`. Commits reachable from those refs appear in the revwalk, inflating author counts.
**Why it happens:** Stash commits have synthetic authors; notes are keyed blobs, not real commits.
**How to avoid:** Pre-flight check already blocks stash. For notes, the revwalk will include note-blob objects but `repo.find_commit(oid)` will return `Err` for non-commit objects — handle that error as `continue` (skip), not as a fatal error.
**Warning signs:** A test repo with a stash shows extra author entries. Fix: use `push_glob("refs/heads/*")` + `push_glob("refs/tags/*")` instead of `push_glob("refs/*")` to avoid stash/notes refs entirely.

### Pitfall 2: `Signature::name()` Returns `None` for Malformed Commits
**What goes wrong:** `commit.author().name()` returns `Option<&str>` — it can be `None` if the name field contains non-UTF-8 bytes.
**Why it happens:** libgit2 validates UTF-8 at read time; corrupted or binary-crafted commits may have non-UTF-8 author fields.
**How to avoid:** Use `unwrap_or("")` or map to an empty string. Don't `.unwrap()`.
**Warning signs:** `repo.find_commit(oid)?` succeeds but `commit.author().name().unwrap()` panics.

### Pitfall 3: Co-author Lines with Extra Whitespace Around the Colon
**What goes wrong:** A line like `Co-authored-by  : Name <email>` (spaces before colon) will not match `starts_with("co-authored-by:")` after lowercasing.
**Why it happens:** git's trailer spec allows whitespace between the key and the separator.
**How to avoid:** After lowercasing the line, use a regex or manual `find(':')` to split on the first colon, then trim both sides. For v1, the simpler `starts_with("co-authored-by:")` handles 99% of real commits; document the edge case.
**Warning signs:** Test with a manually crafted commit with `Co-authored-by  : Name <email>`.

### Pitfall 4: Worktrees Returns the Main Worktree
**What goes wrong:** `repo.worktrees()` might return the main working tree as an entry, causing a false block when no linked worktrees exist.
**Why it happens:** The git2 `worktrees()` API returns a list of *linked* worktree names only (not the main worktree). An empty `Worktrees` list means no linked worktrees. [CITED: docs.rs/git2 worktrees]
**How to avoid:** Check `worktrees.is_empty()` — empty means safe. Non-empty means at least one linked worktree exists → block.
**Warning signs:** The pre-flight check blocks even on single-worktree repos.

---

## Code Examples

### In-Process Test Fixture

The TDD mode (`tdd_mode: true` in config) requires tests. Use `git2::Repository::init()` to create fixture repos without any `git` binary:

```rust
// Source: https://docs.rs/git2/latest/git2/struct.Repository.html#method.init
use git2::{Repository, Signature, Time};
use tempfile::TempDir;

fn create_fixture_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    // Create initial commit
    let sig = Signature::new("Alice", "alice@example.com", &Time::new(1000000, 0)).unwrap();
    let tree_oid = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

    (dir, repo)
}

fn add_commit(repo: &Repository, author_name: &str, author_email: &str, message: &str) {
    let sig = Signature::new(author_name, author_email, &git2::Time::new(1000001, 0)).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let parent_tree = head.tree().unwrap();
    // Reuse same tree (metadata-only commit)
    repo.commit(Some("HEAD"), &sig, &sig, message, &parent_tree, &[&head]).unwrap();
}
```

### Test: Author Enumeration

```rust
#[test]
fn test_enumerate_authors_counts_and_sorts() {
    let (dir, repo) = create_fixture_repo(); // Alice: 1 commit (initial)
    add_commit(&repo, "Bob", "bob@example.com", "Bob's commit");
    add_commit(&repo, "Bob", "bob@example.com", "Bob's second commit");

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
    let (dir, repo) = create_fixture_repo();
    // Three variants of the key
    add_commit(&repo, "Alice", "alice@x.com",
        "feat: do thing\n\nCo-authored-by: Charlie <charlie@x.com>");
    add_commit(&repo, "Alice", "alice@x.com",
        "feat: do thing\n\nCO-AUTHORED-BY: Charlie <charlie@x.com>");
    add_commit(&repo, "Alice", "alice@x.com",
        "feat: do thing\n\nco-authored-by: Charlie <charlie@x.com>");

    let coauthors = enumerate_coauthors(&repo).unwrap();
    assert_eq!(coauthors.len(), 1, "All three variants should deduplicate to one entry");
    assert_eq!(coauthors[0].commit_count, 3);
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
| A1 | `git2::Worktrees` returned by `repo.worktrees()` lists only linked worktrees (not the main worktree) | Pattern 4, Pitfall 4 | Pre-flight would false-block single-worktree repos — needs test to validate |
| A2 | `push_glob("refs/*")` in revwalk does not include unreachable stash objects when the pre-flight block has already returned | Pitfall 1 | If stash refs survive past pre-flight (defensive coding needed), stash commits appear in enumeration |
| A3 | `git2 0.21` is the current version (confirmed via `cargo search`) but `vendored-libgit2` feature name unchanged | Cargo.toml | Build fails with "unknown feature" — verify Cargo.toml on first compile |

**If this table is empty:** All claims in this research were verified or cited — no user confirmation needed.
*Three assumptions remain — all low-impact (can be validated by running tests).*

---

## Open Questions

1. **`push_glob("refs/*")` vs targeted globs**
   - What we know: `refs/*` is broad and includes stash/notes refs
   - What's unclear: whether commits reachable only from `refs/stash` should appear in author enumeration (they probably should not — stash commits use synthetic authors)
   - Recommendation: Use `push_glob("refs/heads/*")` + `push_glob("refs/tags/*")` in `enumerate_authors` and `enumerate_coauthors`. This excludes stash/notes. The pre-flight check already blocks stash, so using the narrow glob is defensive-in-depth.

2. **`clap` in Phase 1 vs defer to Phase 3**
   - What we know: Phase 1 has no user-visible UI; `--version`/`--help` are the only flags ever needed at the binary entry point
   - What's unclear: whether adding clap now vs Phase 3 matters for build complexity
   - Recommendation: Add clap now. The binary must be invocable (`--help` is the first thing users try after curl download); it costs nothing to add at Cargo.toml definition time. The planner can include a single low-effort task for it.

---

## Environment Availability

> Phase 1 is code-only (Cargo.toml + Rust source). The only external dependency is the Rust toolchain.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `rustc` / `cargo` | Build | ✓ | (stable) | — |
| `cmake` | `vendored-libgit2` C compilation | Likely ✓ | (system) | Install via Homebrew/apt |
| `cc` (C compiler) | `vendored-libgit2` C compilation | ✓ on macOS (Xcode CLT) | — | Xcode CLT: `xcode-select --install` |

**Missing dependencies with no fallback:** None detected.

**Note:** `vendored-libgit2` compiles libgit2 from C source during `cargo build`. This requires a working C compiler (`cc`) and CMake. On macOS, Xcode Command Line Tools provide both. On CI (Linux musl), `musl-tools` + system GCC are available. First-time build of this crate takes longer than a pure-Rust crate — expected behavior.

---

## Security Domain

> `security_enforcement` is absent from config.json — treating as enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A — local CLI tool, no auth |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A — user runs on their own repo |
| V5 Input Validation | Yes (partial) | `commit.author().name()` can be non-UTF-8 — use `unwrap_or("")`, never `unwrap()` |
| V6 Cryptography | No | No crypto in Phase 1 |

### Known Threat Patterns for this Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Non-UTF-8 commit author fields | Tampering (crafted repo) | `Option<&str>` from git2 — use `unwrap_or("")`, never panic |
| Deeply nested repo (path traversal) | Elevation | `open_from_env()` uses libgit2's own safety; no custom path construction |
| Malformed `Co-authored-by` line (buffer confusion) | Tampering | `rfind('<')` / `rfind('>')` bounds-check naturally; `str` is always valid UTF-8 in Rust |

**No cryptographic operations in Phase 1.** Security surface is entirely input validation on commit message text and git2 API calls.

---

## Sources

### Primary (HIGH confidence)
- `git2 0.21.0` — confirmed via `cargo search git2` in this session [VERIFIED: cargo registry]
- `thiserror 2.0.18` — confirmed via `cargo search thiserror` in this session [VERIFIED: cargo registry]
- `clap 4.6.1` — confirmed via `cargo search clap` in this session [VERIFIED: cargo registry]
- `Repository::open_from_env()` — [docs.rs/git2/latest/git2/struct.Repository.html](https://docs.rs/git2/latest/git2/struct.Repository.html) [CITED]
- `Repository::worktrees()` — [docs.rs/git2/latest/git2/struct.Repository.html#method.worktrees](https://docs.rs/git2/latest/git2/struct.Repository.html#method.worktrees) [CITED]
- `Revwalk::set_sorting`, `Sort::TOPOLOGICAL | Sort::REVERSE` — [docs.rs/git2/latest/git2/struct.Revwalk.html](https://docs.rs/git2/latest/git2/struct.Revwalk.html) [CITED]
- `Commit::author()`, `Signature::name()`, `Signature::email()` — [docs.rs/git2/latest/git2/struct.Commit.html](https://docs.rs/git2/latest/git2/struct.Commit.html) [CITED]
- Stash detection via `find_reference("refs/stash")` — PITFALLS.md Pitfall 3 (verified in prior domain research session)
- Worktree detection via `repo.worktrees()` — PITFALLS.md Pitfall 4 (verified in prior domain research session)
- Co-author case-insensitivity + trailer parsing strategy — PITFALLS.md Pitfalls 7+8 (verified against git-interpret-trailers spec)
- Unicode safety in name/email parsing — PITFALLS.md Pitfall 9

### Secondary (MEDIUM confidence)
- `push_glob("refs/*")` vs targeted glob behavior — [git2 revwalk docs](https://docs.rs/git2/latest/git2/struct.Revwalk.html) + ARCHITECTURE.md prior research

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
