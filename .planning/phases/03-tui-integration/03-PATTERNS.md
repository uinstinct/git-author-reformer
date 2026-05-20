# Phase 3: TUI + Integration - Pattern Map

**Mapped:** 2026-05-20
**Files analyzed:** 10 (1 modified, 9 new)
**Analogs found:** 3 / 10 (7 have no prior codebase analog)

---

## Structure Conflict — Must Resolve Before Planning

The `<pattern_mapping_context>` prompt lists per-screen files under `src/tui/screens/`:
`main_menu.rs`, `author_list.rs`, `rename_form.rs`, `confirm.rs`, `coauthor_list.rs`, `result.rs`

RESEARCH.md §"Recommended Project Structure" prescribes three centralized files instead:
`src/tui/app.rs`, `src/tui/event.rs`, `src/tui/render.rs`

These are incompatible layouts. This document maps the per-screen file list (explicit directive from
the orchestrator prompt). The planner must pick one structure and document the choice. Do not blend.

**Recommendation:** The centralized `event.rs` + `render.rs` layout from RESEARCH.md is simpler
(one dispatch point per concern). The per-screen layout scatters key handling and rendering across
six files. Unless the planner has a reason to prefer per-screen, use the RESEARCH.md structure.

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/git/scan.rs` | service | CRUD (read-only) | `src/git/rewrite.rs` | exact — same revwalk, same cascade logic |
| `src/main.rs` | config/entry | request-response | `src/main.rs` (existing) | role-match — extends existing shell |
| `src/tui/mod.rs` | module root | — | `src/git/mod.rs` | partial — same mod-root pattern |
| `src/tui/app.rs` | state machine | event-driven | none | no analog |
| `src/tui/event.rs` (or per-screen key handlers) | middleware | event-driven | none | no analog |
| `src/tui/render.rs` (or per-screen render fns) | component | event-driven | none | no analog |
| `src/lib.rs` | module root | — | `src/lib.rs` (existing) | exact — one-line `pub mod tui` addition |
| `Cargo.toml` | config | — | `Cargo.toml` (existing) | exact — add 4 deps |
| `tests/scan_test.rs` | test | CRUD | `tests/rewrite_test.rs` | exact — same fixture + revwalk test pattern |

---

## Pattern Assignments

### `src/git/scan.rs` (service, CRUD read-only)

**Analog:** `src/git/rewrite.rs`

**Imports pattern** (`src/git/rewrite.rs` lines 1–3):
```rust
use crate::git::reader::{parse_coauthor_value, strip_coauthor_prefix};
use git2::{Oid, Sort};
use std::collections::HashMap;
```

scan.rs will need `HashSet` instead of `HashMap` (tracking the `would_remap` set, not building
an OID remapping). Replace `HashMap` with `HashSet`:
```rust
use git2::{Oid, Sort};
use std::collections::HashSet;
```

**Revwalk setup pattern** (`src/git/rewrite.rs` lines 19–25):
```rust
let mut revwalk = repo.revwalk()?;
revwalk.push_glob("refs/heads/*")?;
revwalk.push_glob("refs/tags/*")?;
// Sort::TOPOLOGICAL | Sort::REVERSE guarantees parents are processed before children.
// Every parent OID is either already in oid_map or unchanged when we reach each commit.
revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
```

**Core cascade tracking pattern** (`src/git/rewrite.rs` lines 30–44):
```rust
for oid_result in revwalk {
    let old_oid = oid_result?;
    let commit = repo.find_commit(old_oid)?;

    // Identity match is on AUTHOR only — committer rewrite is a consequence (RENAME-03).
    let identity_matches = commit.author().name().unwrap_or("") == old_name
        && commit.author().email().unwrap_or("") == old_email;

    // A commit must be rewritten if any parent was remapped (Pitfall 1 — prevents
    // stale parent OIDs in descendants).
    let any_parent_remapped =
        (0..commit.parent_count()).any(|i| oid_map.contains_key(&commit.parent_id(i).unwrap()));

    let needs_rewrite = identity_matches || any_parent_remapped;

    if needs_rewrite {
        // ... (in scan: insert into would_remap instead of creating new commit)
    }
}
```

In `scan.rs`, replace `oid_map: HashMap<Oid, Oid>` with `would_remap: HashSet<Oid>` and replace
`oid_map.contains_key(&p)` with `would_remap.contains(&p)`. The insertion becomes
`would_remap.insert(old_oid)` — no new commit is written.

**Error handling pattern** (`src/git/rewrite.rs` line 18 + `src/git/reader.rs` lines 4–6):
```rust
// Function signature: always return Result<T, crate::error::AppError>
pub fn scan_rename(
    repo: &git2::Repository,
    old_name: &str,
    old_email: &str,
) -> Result<RewritePreview, crate::error::AppError>

// git2::Error propagates via #[from] on AppError::Git — use ? directly
let commit = repo.find_commit(old_oid)?;
```

No new `AppError` variant needed. `AppError::Git(#[from] git2::Error)` in `src/error.rs` line 15
covers all git2 errors from scan.

**message_raw() discipline** (`src/git/rewrite.rs` lines 70–72):
```rust
// Use message_raw(), NEVER message() — message() strips leading newlines
// and breaks byte-identity (Anti-Pattern from RESEARCH.md).
let raw_msg = commit
    .message_raw()
    .map_err(|_| crate::error::AppError::NonUtf8Message(old_oid))?;
```

scan.rs does not write commits, so it does not need to call `message_raw()` for rewriting.
However, when parsing co-author lines for `scan_drop`, use `message_raw()` for consistency with
`reader.rs` line 46: `let message = commit.message_raw().unwrap_or("");`

---

### `src/main.rs` (modified, entry point)

**Analog:** `src/main.rs` (existing, lines 1–22)

**Existing shell to preserve** (`src/main.rs` lines 8–22):
```rust
fn run() -> Result<(), error::AppError> {
    let repo = git::open_repo()?;
    git::preflight::check_stash(&repo)?;
    git::preflight::check_worktrees(&repo)?;
    println!("git-author-reformer: preflight passed");
    Ok(())
}

fn main() {
    let _cli = Cli::parse();
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
```

The new `main.rs` extends this shell. Keep the `run()` → `main()` split and the `eprintln +
exit(1)` error path. Add SIGTERM registration before `ratatui::init()`, and always call
`ratatui::restore()` before propagating any error. The `println!` stub call is replaced by
`tui::run(repo)`.

**Constraint from RESEARCH.md (no existing analog — use RESEARCH.md Pattern 1):**
SIGTERM must be registered BEFORE `ratatui::init()`. `ratatui::restore()` must be on ALL exit
paths. See RESEARCH.md Pattern 1 (lines 216–257) for the full init/restore/SIGTERM shell.

---

### `src/tui/mod.rs` (module root)

**Analog:** `src/git/mod.rs` (lines 1–8)

**Pattern** (`src/git/mod.rs` lines 1–8):
```rust
pub mod preflight;
pub mod reader;
pub mod rewrite;
pub mod types;

pub fn open_repo() -> Result<git2::Repository, crate::error::AppError> {
    git2::Repository::open_from_env().map_err(|e| crate::error::AppError::NotARepo(e.to_string()))
}
```

`src/tui/mod.rs` follows the same pattern: declare submodules with `pub mod`, expose one public
entry-point function (`pub fn run(repo: git2::Repository) -> Result<(), crate::error::AppError>`).

---

### `src/lib.rs` (modified, one-line addition)

**Analog:** `src/lib.rs` (existing, lines 1–2)

**Existing content** (`src/lib.rs` lines 1–2):
```rust
pub mod error;
pub mod git;
```

Add one line:
```rust
pub mod tui;
```

No other changes. `tui` module is at the same level as `git`.

---

### `Cargo.toml` (modified, add 4 dependencies)

**Analog:** `Cargo.toml` (existing, lines 7–9)

**Existing dep pattern** (`Cargo.toml` lines 7–9):
```toml
[dependencies]
git2 = { version = "0.21", default-features = false, features = ["vendored-libgit2"] }
thiserror = "2"
clap = { version = "4.6", features = ["derive"] }
```

Add to `[dependencies]` (no feature flags needed for these):
```toml
ratatui = "0.30"
crossterm = "0.29"
nucleo = "0.5"
signal-hook = "0.4"
```

---

### `tests/scan_test.rs` (new test file)

**Analog:** `tests/rewrite_test.rs` (lines 1–22) + `tests/common/mod.rs`

**Test file structure** (`tests/rewrite_test.rs` lines 1–6):
```rust
mod common;

use git2::{ObjectType, Repository, Signature};
use git_author_reformer::git::rewrite::drop_coauthor;
use git_author_reformer::git::rewrite::rewrite_author;
```

scan_test.rs follows the same `mod common;` + use-import pattern:
```rust
mod common;

use git_author_reformer::git::scan::{scan_rename, scan_drop};
```

**Fixture repo pattern** (`tests/common/mod.rs` lines 6–30):
```rust
pub fn create_fixture_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    let sig = Signature::new("Alice", "alice@example.com", &Time::new(1_000_000, 0)).unwrap();
    let tree_oid = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    {
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
    }

    (dir, repo)
}
```

`common::create_fixture_repo()` plus `common::add_commit_with_message()` and
`common::create_annotated_tag()` cover the fixtures needed by scan tests. The `create_annotated_tag`
helper at `tests/common/mod.rs` lines 57–66 is used for SAFE-04 annotated tag tests.

**Test structure pattern** (`tests/rewrite_test.rs` lines 24–43):
```rust
#[test]
fn test_rewrite_author_removes_old_identity_across_all_branches() {
    let (_dir, repo) = common::create_fixture_repo();
    common::add_commit_with_message(&repo, "Alice", "alice@example.com", "alice second");
    // ... setup ...
    let count = rewrite_author(&repo, "Alice", "alice@example.com", "Alice Renamed", "alice2@example.com").unwrap();
    assert!(count >= 2, "...must rewrite at least 2 commits...; got: {count}");
    // ... verify assertions ...
}
```

Each scan test: create fixture, add commits with known properties, call `scan_rename` or `scan_drop`,
assert on `RewritePreview` fields. Tests must encode WHY (business rule ID in assert message).

---

## No Analog Found

Files with no close match in the codebase. Planner must use RESEARCH.md code examples directly.

| File | Role | Data Flow | Reason | RESEARCH.md Reference |
|------|------|-----------|--------|----------------------|
| `src/tui/app.rs` | state machine | event-driven | No prior TUI state machine exists | Pattern 2 (lines 263–303): `Screen` enum, `PendingOp` enum |
| `src/tui/event.rs` (or `screens/*/mod.rs`) | input handler | event-driven | No event handling in codebase | Pattern 1 (lines 239–257): event loop with `KeyEventKind::Press` filter |
| `src/tui/render.rs` (or per-screen render fns) | renderer | event-driven | No TUI rendering in codebase | RESEARCH.md Code Examples (lines 527–555): List widget, Layout split |
| `src/tui/screens/main_menu.rs` | component | event-driven | No TUI screens exist | RESEARCH.md Code Examples: Paragraph + Block widgets |
| `src/tui/screens/author_list.rs` | component | event-driven | No fuzzy list exists | Pattern 3 (lines 309–338): `Nucleo<T>` setup + `apply_filter` |
| `src/tui/screens/coauthor_list.rs` | component | event-driven | No fuzzy list exists | Pattern 3 (lines 309–338): same as author_list |
| `src/tui/screens/rename_form.rs` | component | event-driven | No form input exists | Pattern 7 (lines 393–405): two-field form with `String::pop()` + cursor |
| `src/tui/screens/confirm.rs` | component | event-driven | No confirmation screen exists | RESEARCH.md Code Examples: Paragraph widget displaying `RewritePreview` fields |
| `src/tui/screens/result.rs` | component | event-driven | No result screen exists | Pattern 6 (lines 377–388): remote name detection for force-push message |

---

## Shared Patterns

### AppError propagation
**Source:** `src/error.rs` lines 1–19
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not inside a git repository: {0}")]
    NotARepo(String),
    // ...
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Commit {0} has a non-UTF-8 message — cannot rewrite (git2 requires valid UTF-8)")]
    NonUtf8Message(git2::Oid),
}
```
**Apply to:** `src/git/scan.rs` — no new variants needed; `AppError::Git` covers all git2 errors
from scan. A `TuiError` variant is only needed if the TUI layer produces errors not covered by
existing variants (e.g., `io::Error` from ratatui). Check at implementation time.

### Revwalk idiom (both globs + sort)
**Source:** `src/git/rewrite.rs` lines 19–25 (authoritative — includes tags glob)
**Secondary:** `src/git/reader.rs` lines 75–80 (heads only — NOT the right pattern for scan)
```rust
let mut revwalk = repo.revwalk()?;
revwalk.push_glob("refs/heads/*")?;
revwalk.push_glob("refs/tags/*")?;
revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
```
**Apply to:** `src/git/scan.rs` — must use BOTH globs (same as rewrite.rs), not just heads.
`reader.rs` uses heads-only; that is a different use case (author enumeration, not history rewrite).

### `message_raw()` discipline
**Source:** `src/git/rewrite.rs` lines 66–72 and `src/git/reader.rs` line 46
```rust
// rewrite.rs: explicit error on non-UTF-8
let raw_msg = commit
    .message_raw()
    .map_err(|_| crate::error::AppError::NonUtf8Message(old_oid))?;

// reader.rs: graceful fallback for read-only enumeration
let message = commit.message_raw().unwrap_or("");
```
**Apply to:** `src/git/scan.rs` — use `message_raw().unwrap_or("")` (reader.rs style) when
parsing co-author lines for `scan_drop`. scan.rs does not write commits, so silent skip on
non-UTF-8 is acceptable (consistent with reader.rs pattern).

### Test fixture helpers
**Source:** `tests/common/mod.rs` lines 6–66
**Apply to:** `tests/scan_test.rs` — reuse `create_fixture_repo`, `add_commit_with_message`,
`create_annotated_tag`. No new common helpers needed for scan tests unless GPG-signed commit
fixture is required (not in common/ yet; will need manual git2 commit construction for SAFE-03 tests).

### Repo open
**Source:** `src/git/mod.rs` lines 6–8
```rust
pub fn open_repo() -> Result<git2::Repository, crate::error::AppError> {
    git2::Repository::open_from_env().map_err(|e| crate::error::AppError::NotARepo(e.to_string()))
}
```
**Apply to:** `src/main.rs` (modified) — already calls `git::open_repo()`. No change needed
for the repo-open call; it's already correct.

---

## Metadata

**Analog search scope:** `src/` tree (all .rs files)
**Files scanned:** 9 source files + 5 test files
**Pattern extraction date:** 2026-05-20
