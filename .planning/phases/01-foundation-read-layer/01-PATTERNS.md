# Phase 1: Foundation + Read Layer - Pattern Map

**Mapped:** 2026-05-20
**Files analyzed:** 10
**Analogs found:** 0 / 10 — greenfield project, no Rust source exists yet

## File Classification

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `Cargo.toml` | config | — | none | greenfield |
| `src/main.rs` | CLI entry | request-response | none | greenfield |
| `src/error.rs` | utility | — | none | greenfield |
| `src/git/mod.rs` | module facade | — | none | greenfield |
| `src/git/types.rs` | model | — | none | greenfield |
| `src/git/reader.rs` | service | batch-read | none | greenfield |
| `src/git/preflight.rs` | service | request-response (guard) | none | greenfield |
| `tests/fixtures.rs` | test helper | — | none | greenfield |
| `tests/reader_test.rs` | test | — | none | greenfield |
| `tests/preflight_test.rs` | test | — | none | greenfield |

## Pattern Assignments

No existing codebase analogs. All patterns come from `01-RESEARCH.md` — see the specific Pattern reference per file below.

### `Cargo.toml` (config)
**Source:** `01-RESEARCH.md` — "Cargo.toml Dependencies" code block (lines 75–91)
Use the exact `[dependencies]` block shown: `git2` with `default-features = false, features = ["vendored-libgit2"]`, `thiserror = "2"`, `clap` with `features = ["derive"]`. Add `tempfile = "3"` under `[dev-dependencies]`. Include `[profile.release]` with `strip = true`, `lto = true`, `codegen-units = 1`.

### `src/error.rs` (utility — error type)
**Source:** `01-RESEARCH.md` — Pattern 7 (lines 353–371)
`AppError` enum using `thiserror::Error` derive. Four variants: `NotARepo(String)`, `StashDetected`, `WorktreesDetected(String)`, `Git(#[from] git2::Error)`. This is a shared cross-cutting dependency — define it first; all other modules reference it.

### `src/git/mod.rs` (module facade)
**Source:** `01-RESEARCH.md` — Pattern 1 (lines 172–179)
Declare `pub mod types; pub mod reader; pub mod preflight;` and house `open_repo() -> Result<Repository, AppError>` which calls `Repository::open_from_env()`.

### `src/git/types.rs` (model)
**Source:** `01-RESEARCH.md` — Pattern 5 (lines 253–272) and Pattern 6 (lines 289–316)
Define `AuthorIdentity { name: String, email: String, commit_count: usize }` and `CoAuthorEntry { name: String, email: String, commit_count: usize }`. Both are returned from reader functions and consumed by Phase 3 TUI — make them `pub` with `Clone` and `Debug` derives.

### `src/git/reader.rs` (service — batch-read)
**Source:** `01-RESEARCH.md` — Pattern 2 (revwalk helper, lines 188–199), Pattern 5 (`enumerate_authors`, lines 249–273), Pattern 6 (`enumerate_coauthors` + helpers, lines 285–349)
The private `build_revwalk()` helper is shared by both public functions. Use `push_glob("refs/heads/*")` — never `refs/*` or `refs/tags/*`. Use `HashMap<(String, String), usize>` for counting. Use `eq_ignore_ascii_case` for the co-author prefix match — not `to_lowercase()`.

### `src/git/preflight.rs` (service — guard)
**Source:** `01-RESEARCH.md` — Pattern 3 (`check_stash`, lines 216–222), Pattern 4 (`check_worktrees`, lines 228–242)
Both functions take `&Repository` and return `Result<(), AppError>`. Stash: `repo.find_reference("refs/stash").is_ok()`. Worktrees: `repo.worktrees()` non-empty. Both are blocking gates — return `Err` immediately on detection.

### `src/main.rs` (CLI entry — request-response)
**Source:** `01-RESEARCH.md` — Architecture diagram (lines 126–148) and Pattern 1 (open_repo)
Wire: parse clap args → `open_repo()` → `check_stash()` → `check_worktrees()`. On any `Err`, print to stderr and `std::process::exit(1)`. Phase 1 main does nothing further (TUI is Phase 3). Use `clap` derive macro for `--version` and `--help` only.

### `tests/fixtures.rs` (test helper)
**Source:** `01-RESEARCH.md` — "In-Process Test Fixture" code block (lines 430–453)
`create_fixture_repo() -> (TempDir, Repository)` using `git2::Repository::init()`. `add_commit_with_message(repo, name, email, message)` helper. No git binary — all via git2 in-process.

### `tests/reader_test.rs` (test)
**Source:** `01-RESEARCH.md` — "Test: Author Enumeration" (lines 465–477), "Test: Co-author Case-insensitive" (lines 480–495)
Uses `fixtures::create_fixture_repo` and `fixtures::add_commit_with_message`. Tests verify sort order (count descending) and case-insensitive co-author deduplication.

### `tests/preflight_test.rs` (test)
**Source:** `01-RESEARCH.md` — "Test: Pre-flight Stash Block" (lines 499–513)
Simulates stash by calling `repo.reference("refs/stash", head_oid, false, "test stash")`. Verify result is `Err(AppError::StashDetected)`. Add analogous test for worktrees using `git2::Repository::worktree()` API.

## Shared Patterns

### Error Type
**Source:** `01-RESEARCH.md` Pattern 7
**Apply to:** All modules (`main.rs`, `git/mod.rs`, `git/reader.rs`, `git/preflight.rs`)
All public functions return `Result<_, AppError>`. The `#[from] git2::Error` variant means `?` propagation works without explicit mapping for raw git2 errors.

### Revwalk Helper
**Source:** `01-RESEARCH.md` Pattern 2
**Apply to:** `git/reader.rs` — both `enumerate_authors` and `enumerate_coauthors`
`build_revwalk()` is a private function in `reader.rs`, shared by both public functions. Not exported from the module.

### `unwrap_or("")` on Signature Fields
**Source:** `01-RESEARCH.md` Pitfall 2
**Apply to:** `git/reader.rs` — any access to `commit.author().name()` or `.email()`
Never `.unwrap()` — always `.unwrap_or("")`. Non-UTF-8 commit fields are a valid (if rare) input.

## No Analog Found

All files are in the "no analog" bucket — this is a greenfield Rust project with no prior source.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `Cargo.toml` | config | — | No prior Cargo.toml in repo |
| `src/main.rs` | CLI entry | request-response | No Rust source exists |
| `src/error.rs` | utility | — | No Rust source exists |
| `src/git/mod.rs` | module facade | — | No Rust source exists |
| `src/git/types.rs` | model | — | No Rust source exists |
| `src/git/reader.rs` | service | batch-read | No Rust source exists |
| `src/git/preflight.rs` | service | guard | No Rust source exists |
| `tests/fixtures.rs` | test helper | — | No Rust source exists |
| `tests/reader_test.rs` | test | — | No Rust source exists |
| `tests/preflight_test.rs` | test | — | No Rust source exists |

**Planner action:** Use `01-RESEARCH.md` patterns directly. Each Pattern N in that file contains a complete, copy-ready code block for the corresponding module.

## Metadata

**Analog search scope:** Entire repository (`src/`, `tests/`, `*.toml`)
**Files scanned:** 0 Rust source files found (project is pre-implementation)
**Pattern extraction date:** 2026-05-20
