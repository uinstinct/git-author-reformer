# Phase 5: Hook Engine - Pattern Map

**Mapped:** 2026-05-21
**Files analyzed:** 7 new/modified files
**Analogs found:** 7 / 7 (every file has at least a structural analog in-tree)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/lib.rs` (modified) | crate root | module declaration | `src/lib.rs:1-3` (itself) | exact — one-line addition |
| `src/error.rs` (modified) | error type | declarative enum | `src/error.rs:3-25` (itself) | exact — add one variant |
| `src/hook/mod.rs` (new) | module index + public API | declarative re-exports + thin facade | `src/git/mod.rs:1-9` | exact — same role (module index + one free function) |
| `src/hook/path.rs` (new) | pure helper | request-response (read-only `Repository`) | `src/git/preflight.rs:1-15` | role-match — tiny single-purpose module taking `&Repository` |
| `src/hook/parse.rs` (new) | parser | string -> struct transform | `src/git/reader.rs:84-107` (`strip_coauthor_prefix` + `parse_coauthor_value`) | exact — same in-crate pure-string parser idiom |
| `src/hook/render.rs` (new) | serializer | struct -> string transform | `src/git/rewrite.rs:179-206` (`drop_coauthor_from_message`) | role-match — same "pure transform, no I/O" shape |
| `src/hook/write.rs` (new) | filesystem I/O | file-I/O (atomic write + chmod + rename) | **no codebase analog** | none — use RESEARCH.md §Pattern 1 verbatim |
| `tests/common/mod.rs` (modified) | test helper | shells out to `/bin/sh` | `tests/main_integration_test.rs:3-25` (uses `std::process::Command`) | role-match — same `Command`-based shell-out idiom |
| `tests/hook_test.rs` (new) | integration test | tempdir + fixture + assertion | `tests/preflight_test.rs:1-65` | exact — same `common::create_fixture_repo()` + `#[test]` pattern |

---

## Pattern Assignments

### `src/lib.rs` (modified — crate root)

**Analog:** `src/lib.rs:1-3` (itself)

**Current state (verbatim from file):**
```rust
pub mod error;
pub mod git;
pub mod tui;
```

**Pattern to apply — add one line:**
```rust
pub mod error;
pub mod git;
pub mod hook;   // NEW
pub mod tui;
```

Modules are declared in alphabetical order; insert `hook` between `git` and `tui`.

---

### `src/error.rs` (modified — error type)

**Analog:** `src/error.rs:3-25` (itself — extend the existing enum)

**Existing variant pattern that the new one twins** (lines 5-6 and 11-12, picked because both carry a path-like payload formatted into the message):
```rust
#[error("Not inside a git repository: {0}")]
NotARepo(String),

#[error("Linked worktrees detected: {0}\nRemove worktrees before rewriting history.\nRun: git worktree list")]
WorktreesDetected(String),
```

**Existing `#[from]` pattern reused for `std::io::Error`** (lines 23-24 — already covers `fs::read`, `fs::write`, `fs::rename`, `fs::remove_file`, `fs::set_permissions`):
```rust
#[error("Terminal I/O error: {0}")]
Io(#[from] std::io::Error),
```

**Pattern to apply — add exactly ONE new variant** (per RESEARCH §Pitfall 6 — do not bloat the enum):
```rust
#[error("Existing commit-msg hook at {0} is not managed by git-author-reformer.\nRemove or rename the file, then re-run.")]
HookExists(std::path::PathBuf),
```

Notes:
- The existing `Io(#[from] std::io::Error)` (line 24) already absorbs every `fs::*` error via `?` — do NOT add `HookIoError` or similar.
- The error message's "Terminal I/O error" wording in line 23 is now misleading once `hook` operations also flow through it; per Karpathy Rule 3 (Surgical Changes), DO NOT rephrase it as part of Phase 5. Flag in the plan as a follow-up cleanup if desired, but don't touch it.

---

### `src/hook/mod.rs` (new — module index + public API facade)

**Analog:** `src/git/mod.rs:1-9`

**Imports / module declaration pattern** (lines 1-5):
```rust
pub mod preflight;
pub mod reader;
pub mod rewrite;
pub mod scan;
pub mod types;
```

**Public-function-on-the-module pattern** (lines 7-9):
```rust
pub fn open_repo() -> Result<git2::Repository, crate::error::AppError> {
    git2::Repository::open_from_env().map_err(|e| crate::error::AppError::NotARepo(e.to_string()))
}
```

**Pattern to apply:** Declare sub-modules (`path`, `parse`, `render`, `write`) and expose the three public engine functions (`install_strip`, `remove_strip`, `read_strip_list`) plus the public result enums (`AddResult`, `RemoveResult`, `HookState`) at the `crate::hook::` level. Follow the exact same `Result<_, crate::error::AppError>` return signature as `open_repo`.

Sub-module declaration order: alphabetical (matches `git/mod.rs`).

---

### `src/hook/path.rs` (new — gitdir → hook path resolver)

**Analog:** `src/git/preflight.rs:1-15`

**Single-purpose-function-per-module idiom** (lines 1-6):
```rust
pub fn check_stash(repo: &git2::Repository) -> Result<(), crate::error::AppError> {
    if repo.find_reference("refs/stash").is_ok() {
        return Err(crate::error::AppError::StashDetected);
    }
    Ok(())
}
```

**Pattern to apply** (RESEARCH §Code Examples "Resolve the hook path"):
```rust
use std::path::PathBuf;

pub(crate) fn commit_msg_hook_path(repo: &git2::Repository) -> PathBuf {
    repo.path().join("hooks").join("commit-msg")
}
```

Notes:
- `repo.path()` is the only `git2` call in the whole hook engine — everything else is `std::fs`.
- Visibility: `pub(crate)` is correct (same as `strip_coauthor_prefix` in `reader.rs:84`); the function is only consumed by sibling sub-modules.
- **No existing codebase usage** of `Repository::path()` — `git/scan.rs` and `git/rewrite.rs` use `find_reference`, `revwalk`, etc., but never `path()`. This is a net-new API call; rely on `docs.rs/git2` (cited in RESEARCH).

---

### `src/hook/parse.rs` (new — marker-pair detection + strip-list extraction)

**Analog:** `src/git/reader.rs:82-107` (`strip_coauthor_prefix` + `parse_coauthor_value`)

**Pure-string-parser idiom** (lines 84-92):
```rust
/// Case-insensitive strip of "co-authored-by:" prefix.
/// Returns the rest of the line after the prefix, or None if no match.
pub(crate) fn strip_coauthor_prefix(line: &str) -> Option<&str> {
    let prefix = "co-authored-by:";
    let slice = line.get(..prefix.len())?;
    if slice.eq_ignore_ascii_case(prefix) {
        Some(&line[prefix.len()..])
    } else {
        None
    }
}
```

**Structural extraction idiom** (lines 95-107):
```rust
/// Parse "Name <email>" -> (name, email). Returns None on malformed input.
pub(crate) fn parse_coauthor_value(value: &str) -> Option<(String, String)> {
    let lt = value.rfind('<')?;
    let gt = value.rfind('>')?;
    if gt < lt {
        return None;
    }
    let name = value[..lt].trim().to_string();
    let email = value[lt + 1..gt].trim().to_string();
    if name.is_empty() && email.is_empty() {
        return None;
    }
    Some((name, email))
}
```

**Pattern to apply:** Mirror the same shape — `pub(crate) fn ...(input: &str) -> Option<_>` / `Result<_>` returning a transparent state. Use `&str` slices, `.lines()`, `str::find` over manual byte loops (per RESEARCH §Don't Hand-Roll). Two functions:

1. `pub(crate) fn detect_markers(contents: &str) -> Result<Option<(usize, usize)>, AppError>` — returns `Some((begin_idx, end_idx))` when both markers present in order; `Ok(None)` when file is absent-of-markers (would `NotToolManaged`); the marker constants are private `const BEGIN_MARKER: &str = "..."` / `const END_MARKER: &str = "..."` (exact strings from RESEARCH §Pattern 2).
2. `pub(crate) fn extract_strip_list(contents: &str) -> Result<Vec<String>, AppError>` — calls `detect_markers`, then iterates `contents.lines()` between the two markers, stripping `# ` prefix.

State enum to be defined in `mod.rs` (or `parse.rs`) — three variants per RESEARCH §Open Question 1:
```rust
pub enum HookState {
    Absent,
    Managed { emails: Vec<String> },
    NotToolManaged(PathBuf),
}
```

---

### `src/hook/render.rs` (new — serializer / template)

**Analog:** `src/git/rewrite.rs:179-206` (`drop_coauthor_from_message`)

**Pure string -> string transform idiom** (lines 179-206):
```rust
pub(crate) fn drop_coauthor_from_message(message: &str, target_email: &str) -> String {
    let had_trailing_newline = message.ends_with('\n');
    let kept: Vec<&str> = message
        .lines()
        .filter(|line| { /* ... */ })
        .collect();
    let mut out = kept.join("\n");
    if had_trailing_newline {
        out.push('\n');
    }
    out
}
```

**Pattern to apply:** A single `pub(crate) fn render_hook(emails: &[String]) -> String` that takes a strip list and returns the full POSIX `sh` script. Use `format!` with `\n` line endings (per RESEARCH §Pitfall 4 — never `\r\n`). The full template body is in RESEARCH §Code Examples "Hook file template".

Key constraints from RESEARCH:
- Shebang `#!/bin/sh` (not bash) — RESEARCH §Pitfall 5.
- Two marker comments verbatim from `parse.rs` — share the same `const` strings (single source of truth).
- Lowercase emails before embedding (RESEARCH §Pattern 3) via `.to_ascii_lowercase()`.
- The strip list is written TWICE in the file: once as `# email` comments between markers (read by Rust parser), once as `strip["email"] = 1` inside the `awk BEGIN` block (consumed by shell filter). The serializer writes both from the single `&[String]` input.

---

### `src/hook/write.rs` (new — atomic file write + chmod)

**Analog:** **No codebase analog.** No prior file in `src/` uses `std::fs::rename`, `std::fs::set_permissions`, or `std::os::unix::fs::PermissionsExt` (verified by repo-wide grep — zero hits).

**Pattern source:** RESEARCH §Pattern 1 "Atomic file rewrite with mode" + §Code Examples "Set executable mode (Unix-gated, no-op on Windows)".

**Pattern to apply verbatim** (RESEARCH §Pattern 1):
```rust
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub(crate) fn atomic_write_executable(target: &Path, contents: &str) -> std::io::Result<()> {
    let tmp = target.with_extension(format!("tmp.{}", std::process::id()));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
    }
    let mut perms = fs::metadata(&tmp)?.permissions();
    #[cfg(unix)]
    perms.set_mode(0o755);
    fs::set_permissions(&tmp, perms)?;
    fs::rename(&tmp, target)?;
    Ok(())
}
```

Also exposes:
- `pub(crate) fn delete_hook(path: &Path) -> std::io::Result<()>` — wraps `fs::remove_file`, called when `remove_strip` empties the list (HOOK-10).

Error propagation: returns `std::io::Result`; the caller in `mod.rs` converts via `?` into `AppError::Io(#[from])` (already wired by `src/error.rs:24`).

Visibility: `pub(crate)` — internal to the hook module.

---

### `tests/common/mod.rs` (modified — add `run_hook_on_message` helper)

**Analog:** `tests/common/mod.rs:1-66` (itself) + `tests/main_integration_test.rs:3-25` (for the `std::process::Command` shell-out idiom)

**Existing helper-function idiom** (lines 6-22):
```rust
pub fn create_fixture_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    /* ... */
    (dir, repo)
}
```

**Existing `Command::new(...)` shell-out idiom** (`tests/main_integration_test.rs:3,8-14`):
```rust
use std::process::Command;

let output = Command::new(env!("CARGO_BIN_EXE_git-author-reformer"))
    .current_dir(dir.path())
    .env_remove("GIT_DIR")
    /* ... */
    .output()
    .unwrap();
```

**Pattern to apply** (RESEARCH §Code Examples "Test pattern"):
```rust
/// Runs the generated commit-msg hook against `input_msg` and returns
/// the resulting filtered message. Verifies success criterion #5.
pub fn run_hook_on_message(hook_path: &std::path::Path, input_msg: &str) -> String {
    let dir = tempfile::TempDir::new().unwrap();
    let msg_file = dir.path().join("MSG");
    std::fs::write(&msg_file, input_msg).unwrap();
    let status = std::process::Command::new("/bin/sh")
        .arg(hook_path)
        .arg(&msg_file)
        .status()
        .unwrap();
    assert!(status.success(), "hook script must exit 0");
    std::fs::read_to_string(&msg_file).unwrap()
}
```

Style match notes:
- Use `.unwrap()` not `.expect("...")` — existing helpers in this file all use `.unwrap()` (lines 7, 8, 10, 12, 13, 17).
- `#![allow(dead_code)]` (line 1) already at file top — covers helpers that any one test file might not consume.
- Helper is `pub fn`, no `pub(crate)` — matches lines 6, 24, 32, 36, 57.

---

### `tests/hook_test.rs` (new — integration tests)

**Analog:** `tests/preflight_test.rs:1-65`

**Test file header pattern** (`tests/preflight_test.rs:1-4`):
```rust
mod common;

use git_author_reformer::error::AppError;
use git_author_reformer::git::preflight::{check_stash, check_worktrees};
```

**Single-`#[test]`-per-behavior pattern with doc comment explaining intent** (lines 6-17):
```rust
/// Stash detection: clean repo (no refs/stash) must pass.
/// check_stash exists to block rewrites that would orphan the stash ref.
/// If no stash exists, the gate must be transparent.
#[test]
fn test_check_stash_passes_on_clean_repo() {
    let (_dir, repo) = common::create_fixture_repo();
    let result = check_stash(&repo);
    assert!(
        result.is_ok(),
        "clean repo should pass stash gate; got: {result:?}"
    );
}
```

**`matches!`-based error-variant assertion idiom** (lines 28-32):
```rust
let result = check_stash(&repo);
assert!(
    matches!(result, Err(AppError::StashDetected)),
    "repo with refs/stash must return Err(StashDetected); got: {result:?}"
);
```

**Case-insensitive email assertion idiom** from drop tests (`tests/rewrite_test.rs:373-381`):
```rust
common::add_commit_with_message(
    &repo,
    "Alice", "alice@example.com",
    "feat: ci\n\nCo-Authored-By: Bob <BOB@EXAMPLE.COM>\n",
);
drop_coauthor(&repo, "bob@example.com").unwrap();
let rewritten = find_commit_containing(&repo, "feat: ci");
let msg = rewritten.message_raw().unwrap_or("");
assert!(
    !msg.to_ascii_lowercase().contains("bob@example.com"),
    "DROP-02: case-insensitive key matching must remove BOB@EXAMPLE.COM when target is bob@example.com; msg: {:?}",
    msg
);
```

**Pattern to apply:**
- Each of the 12 tests in RESEARCH §Validation Architecture is one `#[test] fn test_<snake_case>()` with a 1-3 line doc comment citing the HOOK-XX requirement it satisfies.
- Use `common::create_fixture_repo()` for repo setup; use the new `common::run_hook_on_message(...)` for HOOK-08 shell-execution tests.
- Use `matches!(result, Err(AppError::HookExists(_)))` for HOOK-06 refuse-to-overwrite assertion (mirrors the `StashDetected` pattern at line 30).
- Mark the 0755-mode test `#[cfg(unix)]` since `PermissionsExt` is Unix-only (RESEARCH §Validation Architecture row 5).

---

## Shared Patterns

### Module organisation
**Source:** `src/git/mod.rs:1-9`
**Apply to:** `src/hook/mod.rs`

Each module: `pub mod <name>;` lines alphabetically, then any free public functions.

### Error propagation
**Source:** `src/error.rs:14-24`, `src/git/preflight.rs:1-15`, `src/git/reader.rs:4-6`
**Apply to:** `src/hook/mod.rs`, `src/hook/write.rs`

All public functions return `Result<_, crate::error::AppError>`. Inner `std::io::Error` and `git2::Error` propagate via `?` thanks to existing `#[from]` derives on lines 15 and 24 of `error.rs`. ONE new variant `HookExists(PathBuf)` for the only domain-specific failure mode (HOOK-06).

### Visibility convention
**Source:** `src/git/reader.rs:84,95` (`pub(crate)`) vs. `src/git/reader.rs:4,37` (`pub`)
**Apply to:** All `src/hook/` sub-modules

- `pub` only on functions in `mod.rs` that constitute the engine's public API (`install_strip`, `remove_strip`, `read_strip_list`) and their result types (`AddResult`, `RemoveResult`, `HookState`).
- `pub(crate)` on sub-module helpers (`commit_msg_hook_path`, `detect_markers`, `render_hook`, `atomic_write_executable`).
- No `pub use` re-exports unless needed by Phase 6.

### Trailing-newline preservation discipline
**Source:** `src/git/rewrite.rs:181-205`
**Apply to:** `src/hook/render.rs` (when generating the hook), `src/hook/parse.rs` (when round-tripping the strip list)

Capture trailing-newline state before `.lines()` (which strips it), restore after `.join("\n")`. The hook template always ends in `\n` (single source of truth in `render.rs`); the parser must accept both with-and-without trailing newline (HOOK-13 round-trip test).

### Test fixture + integration-test layout
**Source:** `tests/preflight_test.rs:1-4`, `tests/common/mod.rs:6-22`
**Apply to:** `tests/hook_test.rs`

Every integration test file starts with `mod common;`, imports the public API under test from `git_author_reformer::hook::...`, and builds repo fixtures via `common::create_fixture_repo()`. New helpers go in `tests/common/mod.rs` with `pub fn` visibility under the existing `#![allow(dead_code)]` (line 1).

### Shell-out via `std::process::Command`
**Source:** `tests/main_integration_test.rs:3,8-14`
**Apply to:** `tests/common/mod.rs::run_hook_on_message`

`std::process::Command::new("/bin/sh").arg(hook).arg(msg_file).status().unwrap()`; assert `status.success()`. Existing `main_integration_test.rs` uses `.output()` for stderr capture; the hook helper uses `.status()` because the hook side-effects the file in place (RESEARCH §Code Examples).

---

## No Analog Found

Files with no close match in the codebase — these MUST follow RESEARCH.md patterns instead of in-tree analogs:

| File | Role | Data Flow | Reason / Pattern to follow |
|------|------|-----------|----------------------------|
| `src/hook/write.rs` | filesystem I/O | atomic write + chmod + rename | No prior use of `std::fs::rename`, `std::fs::set_permissions`, or `std::os::unix::fs::PermissionsExt` anywhere in the repo. Follow RESEARCH §Pattern 1 and §Code Examples "Set executable mode" verbatim. |
| `src/hook/path.rs` (partial) | gitdir resolution | read-only `Repository` call | No prior use of `git2::Repository::path()` anywhere in the repo. Follow RESEARCH §Code Examples "Resolve the hook path" verbatim; the structural shape (one tiny `pub(crate) fn` taking `&Repository`) does match `src/git/preflight.rs:1-6` so the analog covers form but not API. |
| The POSIX `awk` shell-script template body | embedded string literal in `render.rs` | string template | No shell script exists anywhere in the repo. Follow RESEARCH §Twin Parser Specification + §Code Examples "Hook file template" verbatim. Twin-parity assertion against `src/git/reader.rs:84-107` and `src/git/rewrite.rs:179-206` is the only correctness criterion. |

---

## Metadata

**Analog search scope:**
- `src/` — all 12 `.rs` files
- `tests/` — all 7 `.rs` files (including `tests/common/mod.rs`)
- `Cargo.toml` (dependency confirmation)

**Files scanned:** 19

**Repo-wide grep verifications performed:**
- `fs::rename` — 0 hits
- `PermissionsExt` — 0 hits
- `set_mode` — 0 hits
- `0o755` — 0 hits
- `repo.path()` / `Repository::path` — 0 hits
- `std::process::Command` — 1 hit (`tests/main_integration_test.rs:3`)

**Pattern extraction date:** 2026-05-21

**Key insight:** Every "complex" part of Phase 5 either reuses an existing codebase parser primitive (`src/git/reader.rs:84-107`) or is a textbook `std` pattern documented in RESEARCH.md. The hook engine introduces ZERO new dependencies and ZERO new external APIs beyond `Repository::path()`. No analog is missing in a load-bearing way.
