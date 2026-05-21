# Phase 5: Hook Engine - Research

**Researched:** 2026-05-21
**Domain:** Filesystem-bound `commit-msg` git hook file format (parse / serialize / install / extend / remove) — pure Rust module, no TUI dependencies
**Confidence:** HIGH for codebase facts (every claim is file:line cited); HIGH for Rust APIs (`Repository::path`, `PermissionsExt`, `fs::rename`); MEDIUM for the shell-filter twin design (chosen `awk` idiom is reasoned from the Rust parser's exact behavior — see Pitfall §1).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Claude's Discretion
Everything in this phase. Bound by:
- HOOK-04, HOOK-05, HOOK-06, HOOK-07, HOOK-08, HOOK-10, HOOK-12, HOOK-13 (REQUIREMENTS.md).
- v1.1 STATE decisions:
  - Use `commit-msg` hook (not `post-commit`) — edits message before commit object exists; no SHA churn, no force-push.
  - Store strip list inline in hook file between markers — self-contained, no extra config files.
  - Refuse-to-overwrite pre-existing non-tool hook — safer than merge-on-the-fly.
  - Hook engine (Phase 5) before TUI integration (Phase 6) — mirrors v1.0 engine-then-TUI split.

### Deferred Ideas (OUT OF SCOPE)
- EXT-05: global hook installation via `core.hooksPath` (v1.1 is repo-local only).
- EXT-06: built-in AI author pattern list (user picks from observed co-authors only).
- EXT-07: append the tool's strip block to a pre-existing non-tool-written `commit-msg` hook (refuse-to-overwrite in v1.1).
- Windows binary support (EXT-04): mode-0755 path is Unix-only and `#[cfg(unix)]`-gated; Windows is not a v1 target.
- post-commit hook flow: rejected in REQUIREMENTS.md "Out of Scope".
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HOOK-04 | Write `.git/hooks/commit-msg` if absent, or rewrite it with email appended to embedded list if tool-managed | §Architecture Patterns "Atomic write" + §Don't Hand-Roll: use `std::fs::rename` after `set_permissions`. `Repository::path()` returns gitdir → `.join("hooks").join("commit-msg")`. |
| HOOK-05 | Adding an email already in the strip list is a no-op (file bytes unchanged) | §Architecture: serializer compares the new email against the parsed list case-insensitively (same semantics as `email.eq_ignore_ascii_case` in `rewrite.rs:190`); on duplicate, return `AddResult::AlreadyStripped` without touching the file. |
| HOOK-06 | If `.git/hooks/commit-msg` exists and lacks the tool's marker, refuse to overwrite — no file is written | §Architecture: parser requires BOTH the BEGIN and END marker comments present and in order. Anything else → `HookEngineError::NotToolManaged(PathBuf)`. |
| HOOK-07 | Generated hook is POSIX `sh` (shebang `#!/bin/sh`, no bash-isms), mode 0755 on Unix, strip list between two distinctive marker comments | §Code Examples shows the exact template. Mode set via `std::os::unix::fs::PermissionsExt::from_mode(0o755)`. POSIX-only: use `awk` (not `grep -i` — see Pitfall §1) for the case-insensitive `Co-authored-by:` filter. |
| HOOK-08 | The hook strips lines using the SAME case-insensitive `Co-authored-by:` matching semantics as the existing drop flow | §Twin Parser Specification — Rust parser is the source of truth (`src/git/reader.rs:84-107` + `src/git/rewrite.rs:179-206`); shell filter is a faithful POSIX `awk` reimplementation that twins structural `<…>` email extraction, not a regex match on the raw email substring. |
| HOOK-10 | Removing the last entry deletes the hook file entirely — no empty marker-only file | §Architecture: `remove()` returns `RemoveResult::HookDeleted` and calls `fs::remove_file` when the resulting list is empty. |
| HOOK-12 | Hook install/manage operations do NOT trigger SAFE-01/SAFE-02 preflight | §Architectural Concerns: `main.rs:20-21` calls preflight UNCONDITIONALLY. Phase 5's hook engine itself does not call `check_stash` or `check_worktrees` — that's correct for the engine. But Phase 6 will need to refactor `main.rs` so the hook flows reach the engine without going through preflight. Phase 5 must NOT add preflight calls to the engine API. |
| HOOK-13 | Rust tests cover every code path | §Validation Architecture lists 11 required tests covering each named branch + shell-script execution against fixtures. |
</phase_requirements>

## Summary

The hook engine is a small, focused, **filesystem-only** Rust module — no `git2` writes beyond reading `Repository::path()` to locate the gitdir. It owns one file: `<gitdir>/hooks/commit-msg`. Its operations are read (parse the embedded strip list), serialize (render a POSIX `sh` script with the list embedded between two marker comments), install (write atomically with mode 0755), and remove (delete the file when the list empties).

The non-obvious part is the **twin parser**. The phase description hints at `grep -i` / `sed`, but a faithful reimplementation of the Rust drop flow (which extracts the email *structurally* from between `<` and `>` and case-folds before comparing) requires `awk`, not `grep`. A `grep -i bob@example.com` filter would incorrectly drop a line where `bob@example.com` appears in the *name* slot rather than the email slot. POSIX `awk` is the only widely-available POSIX tool that can mirror the Rust parser's structural extraction without bash-isms.

**Primary recommendation:** New top-level module `src/hook/mod.rs` (peer of `git/` and `tui/`). Public API: `install_strip(repo, email) -> Result<AddResult>`, `remove_strip(repo, email) -> Result<RemoveResult>`, `read_strip_list(repo) -> Result<HookState>`. Hook file rendered via a `format!`-based template with two distinctive sentinel markers (`# >>> git-author-reformer auto-strip BEGIN >>>` / `# <<< git-author-reformer auto-strip END <<<`); shell filter implemented in POSIX `awk`. Atomic write via `tempfile-in-same-dir → set_permissions(0o755) → rename`. Tests use `tempfile::TempDir` (existing pattern, `tests/common/mod.rs:6`) and shell out via `std::process::Command::new("/bin/sh")` to verify success criterion #5.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Resolve gitdir → hook path | `git` (read-only use of `Repository::path()`) | — | `Repository::path()` is the only `git2` call in the engine; everything else is `std::fs`. |
| Parse / serialize hook file | `hook` (new module) | — | Pure string ↔ struct transform; no git knowledge. |
| Atomic file write + chmod | `hook` (new module) | — | Standard POSIX file-rewrite pattern; isolated to the engine. |
| Twin parser (shell `awk` filter) | `hook` (embedded template) | `reader` (Rust source of truth) | Rust parser in `src/git/reader.rs:84-107` is the source of truth; shell filter is its faithful POSIX twin. |
| Preflight gates (SAFE-01/02) | (NOT invoked by `hook`) | `main.rs` (gates them today unconditionally) | HOOK-12 requires hook ops bypass preflight; Phase 6 will refactor `main.rs`. Phase 5 just refrains from calling them. |
| Tests | `tests/hook_test.rs` (new) + helper in `tests/common/mod.rs` | — | Mirrors existing convention (`tests/rewrite_test.rs`, `tests/scan_test.rs`). |

## Standard Stack

### Core

| Library | Version (Cargo.toml) | Purpose | Why Standard |
|---------|---------------------|---------|--------------|
| `git2` | `0.21` (vendored-libgit2, default-features off) — Cargo.toml:8 | Reads `Repository::path()` to locate gitdir; nothing else | Already in dep tree; only read-only use here. [VERIFIED: codebase] |
| `std::fs` | std | Read / write / rename / remove / set_permissions on the hook file | Standard library; zero new deps. [VERIFIED: rust std] |
| `std::os::unix::fs::PermissionsExt` | std | Set mode 0755 on Unix via `from_mode(0o755)` / `set_mode(0o755)` | Standard library; the only correct path for a single octal mode on a file. [VERIFIED: rust std] |
| `tempfile` | `3` (dev-dependencies, Cargo.toml:17) | Test fixtures (`TempDir`) | Already in dev-deps; same pattern as `tests/common/mod.rs:6`. [VERIFIED: codebase] |
| `thiserror` | `2` (Cargo.toml:9) | Hook-engine error type variants on `AppError` | Already in dep tree; same pattern as `src/error.rs:3`. [VERIFIED: codebase] |

### Supporting

| Library | Use | When |
|---------|-----|------|
| `std::process::Command` (tests only) | Run the generated hook script with `/bin/sh` against fixture commit-message files to verify success criterion #5 | In `tests/hook_test.rs` only, never in `src/`. [VERIFIED: rust std] |

### Alternatives Considered (and rejected)

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New `src/hook/` top-level module | `src/git/hook.rs` under existing `git/` | The `git/` module is exclusively about *history-touching* operations (rewrite, scan, preflight, reader — see `src/git/mod.rs:1-5`). Hooks are filesystem infra, not history operations. Top-level `hook/` matches the conceptual boundary. [VERIFIED: codebase + advisor reasoning] |
| `tempfile::NamedTempFile::persist` for atomic write | Manual `<file>.tmp` + `fs::rename` | `tempfile` is dev-only in this crate (Cargo.toml:17); pulling it into prod deps would add a runtime dep solely for a single rename pattern. Manual `<basename>.tmp.<pid>` + `fs::rename` in the same directory is sufficient and matches the codebase's "minimum deps" ethos. [ASSUMED — judgment call] |
| `grep -i 'Co-authored-by:.*<email>'` in shell filter | POSIX `awk` with structural `<...>` extraction | A regex match on the raw substring drops lines where the email appears in the *name* slot too — NOT what the Rust parser does (see §Twin Parser Specification + Pitfall §1). `awk` correctly mirrors `parse_coauthor_value` in `src/git/reader.rs:95-107`. [VERIFIED: source code analysis] |
| `sed` for the filter | POSIX `awk` | `sed` cannot easily do the case-folded comparison + structural `<…>` extraction in a single pass without GNU extensions. `awk`'s `tolower()`, `substr()`, `index()` are all POSIX. [CITED: oneuptime.com posix-shell-compatibility] |
| `bash` shebang | `#!/bin/sh` | HOOK-07 explicitly requires POSIX `sh`, no bash-isms. On macOS `/bin/sh` is bash-in-POSIX-mode; on Debian/Ubuntu it's dash. Both run POSIX `awk`. [CITED: shellcheck SC2039] |
| `Repository::path()` + manual `.join("hooks")` | git2 has no helper for hooks dir; this is the canonical pattern | `Repository::path()` returns the gitdir for normal repos, the repo itself for bare repos. Join `"hooks"` to get hooks dir. [VERIFIED: docs.rs/git2] |

**Installation:** No new dependencies required. All Cargo.toml entries already exist.

**Version verification:** Already done in Phase 1 — `git2 0.21`, `thiserror 2`, `tempfile 3` confirmed via `cargo search` in earlier phase research. [VERIFIED: prior phase research + Cargo.toml]

## Package Legitimacy Audit

This phase installs **no new packages**. All required crates already exist in `Cargo.toml` and were vetted in earlier phases. No legitimacy gate needed.

| Package | Registry | Disposition |
|---------|----------|-------------|
| (none — no new deps) | — | N/A |

## Twin Parser Specification (CRITICAL — single most load-bearing section)

### Rust source of truth — exact behavior

The Rust drop flow performs three steps per commit-message line. **The shell filter must twin all three.**

1. **Prefix match** — `src/git/reader.rs:84-92` (`strip_coauthor_prefix`):
   - Take the **trimmed** line.
   - Slice the first 15 bytes (`"co-authored-by:".len()`).
   - Compare with `eq_ignore_ascii_case` against `"co-authored-by:"`.
   - On match, return the rest of the line *unmodified* (preserves case of name/email portion).

2. **Structural value parse** — `src/git/reader.rs:95-107` (`parse_coauthor_value`):
   - Find the **last** `<` (`rfind('<')`).
   - Find the **last** `>` (`rfind('>')`).
   - Reject if `>` is before `<`.
   - `name` = `value[..lt].trim()`, `email` = `value[lt+1..gt].trim()`.
   - Reject if both `name` and `email` are empty.

3. **Email comparison** — `src/git/rewrite.rs:189-191`:
   - `email.eq_ignore_ascii_case(target_email)` — ASCII case-fold (NOT Unicode).
   - **The full extracted email is compared, not a substring.**

### Why `grep -i <email>` is wrong

Counterexample: `Co-authored-by: bob@example.com <alice@example.com>`

- **Rust parser**: extracts `email = "alice@example.com"` (between `<>`); when target is `bob@example.com`, does NOT drop. Preserves the line.
- **`grep -i 'bob@example.com'`**: matches anywhere on the line, drops it. WRONG behavior.

This counterexample fails success criterion #5 ("same matching semantics as the existing drop flow") silently — the bug would only surface when a co-author's email happens to appear in another co-author's display name on the same line. Unlikely but real.

### POSIX `awk` twin — recommended idiom

The hook file embeds the strip list as one `awk` array entry per email, all lowercase. The script's filter is one `awk` invocation:

```sh
awk '
BEGIN {
  # ----- embedded strip list (auto-generated) -----
  strip["bob@example.com"] = 1
  strip["carol@example.com"] = 1
  # ----- end embedded strip list -----
}
{
  line = $0
  # 1. Trim leading whitespace for prefix match (mirrors line.trim() in Rust).
  t = line
  sub(/^[ \t]+/, "", t)
  # 2. Case-insensitive prefix match on "co-authored-by:" (15 bytes).
  prefix = tolower(substr(t, 1, 15))
  if (prefix != "co-authored-by:") { print; next }
  # 3. Find LAST < and LAST > in the line — structural extraction.
  rest = substr(t, 16)
  lt = 0; gt = 0
  for (i = length(rest); i > 0; i--) {
    c = substr(rest, i, 1)
    if (gt == 0 && c == ">") gt = i
    if (c == "<")            { lt = i; break }
  }
  if (lt == 0 || gt == 0 || gt < lt) { print; next }
  # 4. Extract email, trim, lowercase, lookup.
  email = substr(rest, lt + 1, gt - lt - 1)
  gsub(/^[ \t]+|[ \t]+$/, "", email)
  email = tolower(email)
  if (email in strip) next   # drop the line
  print
}
' "$1" > "$1.tmp" && mv "$1.tmp" "$1"
```

Key POSIX-portability notes:
- `tolower()` is POSIX awk. [CITED: POSIX awk spec]
- `substr()`, `length()`, `sub()`, `gsub()` are all POSIX awk.
- No bash-isms; works under `dash`, macOS `/bin/sh`, Solaris `/bin/sh`.
- `awk` is mandated by POSIX and present on every target platform (Linux musl static, macOS aarch64, macOS x86_64). [VERIFIED: POSIX.1-2017]
- The `mv` at the end is atomic on the same filesystem, matching the same atomic-rewrite discipline the Rust side uses. [CITED: POSIX rename]

The shell filter case-folds before lookup; the Rust serializer MUST write all embedded emails lowercased so the `awk` map lookup is correct. Cite: `email.eq_ignore_ascii_case` in `rewrite.rs:190` proves the case-fold is part of the matching semantics; the shell side simply pushes the fold to write-time.

[VERIFIED: source-code analysis of `src/git/reader.rs:84-107` + `src/git/rewrite.rs:179-206`]

## Architecture Patterns

### System Architecture Diagram

```
                Phase 6 TUI Add/Manage flow (out of scope here)
                                |
                                v
   +------------------- HOOK ENGINE PUBLIC API ----------------------+
   |  install_strip(repo, email) -> Result<AddResult, AppError>     |
   |  remove_strip(repo, email)  -> Result<RemoveResult, AppError>  |
   |  read_strip_list(repo)      -> Result<HookState, AppError>     |
   +-----------------------------------------------------------------+
                                |
            +-------------------+--------------------+
            v                                        v
    1. resolve hook path                  2. read existing file (if any)
       repo.path()                            std::fs::read_to_string
       .join("hooks")
       .join("commit-msg")
                                                    |
                                                    v
                                         3. parse: tool-managed?
                                            - check BEGIN+END markers
                                            - if absent and file exists -> NotToolManaged err
                                            - if no file              -> empty state
                                            - if both markers present -> extract list
                                                    |
                                                    v
                                         4. apply op (add/remove)
                                            - duplicate add -> AlreadyStripped (no write)
                                            - remove last  -> delete file
                                                    |
                                                    v
                                         5. serialize new content (if list non-empty)
                                            template literal + list lines
                                                    |
                                                    v
                                         6. atomic write
                                            - write to <path>.tmp.<pid>
                                            - set mode 0755 on tmp
                                            - rename tmp -> commit-msg
                                                    |
                                                    v
                                         7. return result enum to caller
```

### Recommended Project Structure

```
src/
├── error.rs                       # extend AppError with HookEngine* variants (Pitfall §6)
├── git/                           # unchanged — reused for Repository::path() only
├── hook/                          # NEW top-level module
│   ├── mod.rs                     # public API: install_strip, remove_strip, read_strip_list
│   ├── path.rs                    # hook-path resolution (repo.path().join("hooks").join("commit-msg"))
│   ├── parse.rs                   # marker detection + strip-list extraction
│   ├── render.rs                  # template + embedded list serialization
│   └── write.rs                   # atomic write (tmp + chmod 0755 + rename)
└── lib.rs                         # add `pub mod hook;`

tests/
├── common/mod.rs                  # extend with `assert_hook_strips(hook_path, msg_in, msg_out)`
└── hook_test.rs                   # NEW — 11 tests (HOOK-13)
```

Justification for splitting `mod.rs` into 4 sub-files: each file is single-responsibility and < 200 LOC, matches the codebase's existing pattern of one file per concern (`git/reader.rs`, `git/scan.rs`, `git/rewrite.rs`, `git/preflight.rs`). Single-file mod is also acceptable if total LOC stays under ~250.

### Pattern 1: Atomic file rewrite with mode

**What:** Write to a temp file in the same directory, set permissions on the temp, then rename atomically over the destination. `fs::rename` is atomic on the same filesystem on Linux/macOS. Setting permissions on the temp before rename closes the window where a non-executable file is visible at the final path.

**When to use:** Every write to `.git/hooks/commit-msg`.

**Example:**

```rust
// Source: std::fs + std::os::unix::fs::PermissionsExt
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn atomic_write_executable(target: &Path, contents: &str) -> std::io::Result<()> {
    let tmp = target.with_extension(format!("tmp.{}", std::process::id()));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;                           // durability
    }
    let mut perms = fs::metadata(&tmp)?.permissions();
    #[cfg(unix)]
    perms.set_mode(0o755);                       // chmod on tmp BEFORE rename (Pitfall §3)
    fs::set_permissions(&tmp, perms)?;
    fs::rename(&tmp, target)?;                   // atomic on same fs
    Ok(())
}
```

The `#[cfg(unix)]` gate makes the mode call a no-op on Windows (not a v1 target but cheap to be correct). [VERIFIED: rust std PermissionsExt]

### Pattern 2: Marker-pair detection

**What:** Tool-managed = file contains BOTH a BEGIN marker AND an END marker, with BEGIN preceding END, and the embedded strip list strictly between them.

**Markers:** Use distinctive sentinels that are extremely unlikely to appear by accident in a user's hook:

```sh
# >>> git-author-reformer auto-strip BEGIN >>>
# bob@example.com
# carol@example.com
# <<< git-author-reformer auto-strip END <<<
```

The chevron pattern is borrowed from `conda init` / `direnv init` shell snippets and is widely understood as "tool-managed region". Both markers required prevents a coincidental partial-match. [ASSUMED — convention from prior art]

**When to use:** Both the parser (read-and-detect) and the serializer (write-template) use the same const strings.

### Pattern 3: Lowercase emails at write time

**What:** When adding an email to the strip list, lowercase it via `to_ascii_lowercase()` before writing. The shell filter does its lookup on the lowercased extracted email; if the embedded list isn't lowercased at write time, lookups against mixed-case input would fail.

**Equivalence:** `email_in_input.to_ascii_lowercase() == email_in_list.to_ascii_lowercase()` is what `eq_ignore_ascii_case` does on the Rust side. Pre-lowercasing on write keeps the shell side simple.

### Anti-Patterns to Avoid

- **`grep -i bob@example.com`** in the filter — drops lines where the email appears in the name slot (see §Twin Parser Specification).
- **Single-marker detection** — a user comment matching the marker by coincidence becomes a false positive.
- **`chmod` after `rename`** — opens a window where the file is in place but not executable (the git commit hook would silently fail until chmod completes).
- **`bash` shebang or `[[ ... ]]` syntax** — breaks on dash (`/bin/sh` on Debian/Ubuntu). HOOK-07 mandates POSIX. [CITED: shellcheck SC2039]
- **`fs::write` directly to `.git/hooks/commit-msg`** — non-atomic; if interrupted, leaves a partial hook file.
- **Calling `check_stash`/`check_worktrees` from hook ops** — violates HOOK-12. Hook install/manage is not history-rewriting.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Email parsing | Custom `<...>` regex | `src/git/reader.rs:95-107` (`parse_coauthor_value`) — reuse via `pub(crate)` visibility (it's already visible within the crate) | Single source of truth; HOOK-08 demands the SAME semantics. The visibility is already correct (`pub(crate)` in `reader.rs:84,95`); a new top-level `src/hook/` module is still in-crate. [VERIFIED: source code] |
| Prefix-match for "Co-authored-by:" | Hand-roll case-fold | `src/git/reader.rs:84-92` (`strip_coauthor_prefix`) | Same as above. |
| Atomic file rewrite | A bespoke 3-step shuffle in each function | One private helper `write::atomic_write_executable` in `src/hook/write.rs` | One canonical pattern; tested once. |
| Hook directory resolution | Walking parent dirs looking for `.git/` | `git2::Repository::path()` (already open in `main.rs:19` and passed everywhere) | Handles bare repos, separate gitdirs, `--git-dir`, and worktrees correctly. [VERIFIED: docs.rs/git2 Repository::path] |
| Permission octals | `chmod` shelling out | `std::os::unix::fs::PermissionsExt::set_mode(0o755)` | Standard library; faster; no PATH dependency. [VERIFIED: rust std] |
| Marker parsing | regex crate | `str::find` / `str::lines` / `str::splitn` | No new dep; markers are fixed strings, not patterns. |

**Key insight:** Every "complex" part of this phase is either already implemented in the codebase (twin parser primitives in `reader.rs`) or already exists in the standard library (atomic write, `PermissionsExt`). Hand-rolling is the antipattern.

## Runtime State Inventory

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | None — hook engine writes a single file at `<gitdir>/hooks/commit-msg`. No databases, no caches, no other on-disk state. | None. |
| Live service config | None — local-repo-only feature, no remote services. | None. |
| OS-registered state | None — `.git/hooks/commit-msg` is read at commit-time by `git` itself; no systemd / launchd / Task Scheduler involvement. | None. |
| Secrets / env vars | None — hook script reads `$1` (the commit-message file path passed by git) and modifies it in place. No secrets. | None. |
| Build artifacts | The compiled binary `git-author-reformer` includes the hook template as a string literal; rebuilding is automatic via `cargo build`. No stale artifacts. | None. |

**This is a code-and-on-disk-hook-file-only feature.** No external state to migrate or invalidate.

## Common Pitfalls

### Pitfall 1: "grep -i" twin parser hides a structural bug

**What goes wrong:** Implementing the shell filter as `grep -iv 'Co-authored-by:.*bob@example.com'` (or similar substring match) does NOT mirror the Rust drop flow. The Rust parser extracts the email *structurally* from between the LAST `<` and LAST `>`; the shell side must do the same with `awk`. See §Twin Parser Specification for the counterexample.

**Why it happens:** The phase description explicitly mentions `grep -i / sed`. Easy to take at face value without reading `src/git/reader.rs:95-107`.

**How to avoid:** Use the `awk` template in §Twin Parser Specification. Every plan task that touches the shell filter must cite `src/git/reader.rs:84-107` and `src/git/rewrite.rs:179-206` as the source of truth.

**Warning signs:** A test like `Co-authored-by: bob@example.com <alice@example.com>` with target `bob@example.com` passing the Rust drop but being dropped by the shell hook — divergent behavior under HOOK-08.

### Pitfall 2: BEGIN marker without END marker (or vice-versa) treated as tool-managed

**What goes wrong:** A user happens to have a comment matching just the BEGIN marker (e.g., copied a snippet from this tool's README). Single-marker detection misclassifies their hook as tool-managed and silently overwrites it.

**How to avoid:** Require BOTH the BEGIN and END markers, in order, with sane content between them. Anything else → `NotToolManaged` → HOOK-06 refuse-to-overwrite.

### Pitfall 3: `chmod` after `rename` leaves a non-executable window

**What goes wrong:** `fs::rename(tmp, hook)` succeeds, but `fs::set_permissions(hook, 0o755)` hasn't run yet. A concurrent `git commit` in that microsecond fails because the hook isn't executable.

**How to avoid:** `set_permissions` on the temp file *before* `rename`. The atomic rename then publishes a file that is already executable. [VERIFIED: Pattern 1 above + advisor reasoning]

### Pitfall 4: CRLF line endings break `#!/bin/sh`

**What goes wrong:** If the hook file is checked into git on a Windows machine with `core.autocrlf=true`, or if the Rust writer accidentally emits `\r\n`, the shebang becomes `#!/bin/sh\r` and Linux/macOS interpret it as a request for an executable named `sh\r`, failing with `bad interpreter`. [CITED: codestudy.net git-pre-and-post-commit-hooks-not-running]

**How to avoid:** Hardcode `\n` line endings in the Rust template (use `\n` in `format!`, never `\r\n`). The hook file is never checked in (`.git/hooks/` is inside `.git/`, ignored by definition), so `autocrlf` doesn't touch it — but the writer must still emit LF. The shell `awk` filter also writes via `mv "$1.tmp" "$1"` and does not touch line endings of the commit-message file it processes.

### Pitfall 5: macOS dash vs bash `/bin/sh` divergence

**What goes wrong:** A bash-ism (e.g., `[[ ... ]]`, `${var,,}`, arrays, `function name() { ... }`) works on macOS `/bin/sh` (which is bash-in-POSIX-mode) but fails on Debian/Ubuntu where `/bin/sh` is dash. [CITED: oneuptime.com posix-shell-compatibility; shellcheck SC2039]

**How to avoid:** Stick to POSIX `awk` for the filtering logic — `awk` is identical across all target platforms because the implementation is in `awk`, not `sh`. The shell wrapper around the `awk` invocation uses only `$1`, `mv`, `&&` — all POSIX.

### Pitfall 6: AppError enum bloat — extend cleanly

**What goes wrong:** Adding too many one-off error variants to `AppError` (`src/error.rs:3-25`) makes the enum noisy.

**How to avoid:** Add exactly two variants for the engine:
- `HookExists(PathBuf)` — for HOOK-06 refuse-to-overwrite. Message instructs user to remove/rename the file.
- The existing `Io(#[from] std::io::Error)` (`error.rs:24`) already covers fs read/write/rename failures via `?`.

No other variant is needed; `AlreadyStripped` and `HookDeleted` are *success* results, not errors — model them on the return type of `install_strip` / `remove_strip` (e.g., `enum AddResult { Installed { count: usize }, AlreadyStripped }`).

### Pitfall 7: `repo.path()` returns the gitdir, not the worktree

**What goes wrong:** Treating `repo.path()` as the working tree root and writing to `<root>/.git/hooks/commit-msg`. For a worktree or bare repo, this is wrong.

**How to avoid:** `repo.path().join("hooks").join("commit-msg")` — `path()` already IS the gitdir (`.git/` for normal repos, the bare repo itself for bare repos). No `.git/` prefix needed. [VERIFIED: docs.rs/git2 Repository::path]

### Pitfall 8: Hook file not executable on freshly-cloned repos (general gotcha — not our problem)

**What goes wrong:** Distributed templates sometimes lose the executable bit on Windows-hosted clones. Not relevant here because we *write* the file ourselves at install time and `set_mode(0o755)` on the temp before rename. Users who delete and re-install via the tool always get a 0755 file.

## Code Examples

Verified patterns from this codebase and Rust std:

### Resolve the hook path

```rust
// Source: docs.rs/git2 Repository::path + advisor confirmation
use std::path::PathBuf;

pub(crate) fn commit_msg_hook_path(repo: &git2::Repository) -> PathBuf {
    repo.path().join("hooks").join("commit-msg")
}
```

### Reuse the Rust twin parser primitives

```rust
// Source: src/git/reader.rs:84-107 — already pub(crate), visible from src/hook/
use crate::git::reader::{strip_coauthor_prefix, parse_coauthor_value};

fn line_matches_strip_email(line: &str, target: &str) -> bool {
    let trimmed = line.trim();
    let Some(rest) = strip_coauthor_prefix(trimmed) else { return false; };
    let Some((_name, email)) = parse_coauthor_value(rest.trim()) else { return false; };
    email.eq_ignore_ascii_case(target)
}
```

This is what `drop_coauthor_from_message` already does (`src/git/rewrite.rs:184-200`); the hook engine doesn't need its own copy of this matcher because the *Rust* side of the hook engine only needs to read/write the strip-list config, not run the filter. The filter runs in `awk` at commit time.

### Set executable mode (Unix-gated, no-op on Windows)

```rust
// Source: std::os::unix::fs::PermissionsExt
#[cfg(unix)]
fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) -> std::io::Result<()> {
    // Windows: no-op. Not a v1 target; included for compile-correctness only.
    Ok(())
}
```

### Test pattern — extend `tests/common/mod.rs` with hook fixture helper

```rust
// Add to tests/common/mod.rs (new helper, matches existing conventions):

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

This mirrors the existing helper convention in `tests/common/mod.rs:6-66` (`create_fixture_repo`, `add_commit_with_message`, etc.).

### Hook file template (full output for a 2-entry strip list)

```sh
#!/bin/sh
# >>> git-author-reformer auto-strip BEGIN >>>
# bob@example.com
# carol@example.com
# <<< git-author-reformer auto-strip END <<<

# Filter Co-authored-by trailers whose email matches any in the embedded list.
# Twin of the Rust drop flow in src/git/reader.rs + src/git/rewrite.rs.
# Matching semantics: case-insensitive prefix on "co-authored-by:", structural
# extraction of email from the LAST <…> pair, ASCII case-fold compare.

awk '
BEGIN {
  strip["bob@example.com"] = 1
  strip["carol@example.com"] = 1
}
{
  line = $0
  t = line
  sub(/^[ \t]+/, "", t)
  prefix = tolower(substr(t, 1, 15))
  if (prefix != "co-authored-by:") { print; next }
  rest = substr(t, 16)
  lt = 0; gt = 0
  for (i = length(rest); i > 0; i--) {
    c = substr(rest, i, 1)
    if (gt == 0 && c == ">") gt = i
    if (c == "<")            { lt = i; break }
  }
  if (lt == 0 || gt == 0 || gt < lt) { print; next }
  email = substr(rest, lt + 1, gt - lt - 1)
  gsub(/^[ \t]+|[ \t]+$/, "", email)
  email = tolower(email)
  if (email in strip) next
  print
}
' "$1" > "$1.tmp" && mv "$1.tmp" "$1"
```

Note the duplication: the strip list appears twice in the file:
1. Once as `# email` comment lines between the BEGIN/END markers — this is what the Rust parser reads back.
2. Once as `strip["email"] = 1` inside the `awk` BEGIN block — this is what the runtime filter uses.

Keeping these as separate sections (rather than trying to make `awk` parse its own embedded comments) is the simplest, most testable design. The serializer writes both from the same input list; a round-trip test (`parse(render(list)) == list`) is part of HOOK-13.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `actions-rs/toolchain` (CI rust toolchain) | `dtolnay/rust-toolchain@stable` | Phase 4 (already adopted) | N/A — Phase 5 is fully library work, no CI changes. |
| `git filter-branch` for history rewrite | `git2`-based revwalk in `src/git/rewrite.rs` | Phase 2 (already adopted) | N/A — Phase 5 is not history rewrite. |

**Deprecated/outdated:** Nothing in this phase touches deprecated areas.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `tempfile` is not needed as a *prod* dep — manual `<basename>.tmp.<pid>` rename is sufficient | §Alternatives Considered | Low. Worst case the planner adds `tempfile` to `[dependencies]` (already in `[dev-dependencies]`). |
| A2 | New top-level `src/hook/` is the right module placement (vs `src/git/hook.rs`) | §Recommended Project Structure | Low. Either works; the planner can override. The rationale (history-rewriting vs filesystem infra) is sound but ultimately a judgment call. |
| A3 | The marker tokens `# >>> git-author-reformer auto-strip BEGIN >>>` / `# <<< … END <<<` are unlikely to collide with user content | §Pattern 2 | Very low — the tool name + chevron pattern is highly distinctive. If concerned, add a version suffix (e.g., `v1`). |
| A4 | POSIX `awk` is universally available on all target platforms (Linux musl static, macOS aarch64, macOS x86_64) | §Twin Parser Specification | Very low — `awk` is POSIX-mandated and ships with every Unix system the binary targets. |
| A5 | Splitting `src/hook/` into `mod.rs` + `path.rs` + `parse.rs` + `render.rs` + `write.rs` is right-sized | §Recommended Project Structure | Low. Could be a single 250-LOC `src/hook/mod.rs` — equally valid. Planner can collapse if preferred. |
| A6 | The `awk` script writes back via `> "$1.tmp" && mv "$1.tmp" "$1"` — atomic, but does that work for the commit-msg hook contract? | §Code Examples (hook template) | Low — `git` reads the commit-msg file by path after the hook returns; in-place rewrite via tmp+rename is standard for `commit-msg` hooks. [ASSUMED — common pattern, not verified against `git`'s hook docs in this session] |

## Architectural Concerns (for Phase 6, not solved here)

**`main.rs:20-21` calls preflight unconditionally:**

```rust
git::preflight::check_stash(&repo)?;
git::preflight::check_worktrees(&repo)?;
```

This violates HOOK-12 for hook flows. **Phase 5 does NOT need to fix this** — Phase 5's hook engine is itself preflight-free, satisfying success criterion #6 ("Calling any hook engine operation does not invoke the SAFE-01/SAFE-02 preflight blockers"). Phase 5 just must not add preflight calls to the engine.

**Phase 6 will need to refactor `main.rs` and/or `tui::run_with_terminal`** so the hook flows reach the engine without going through the preflight gates. Possible designs (Phase 6's call):
- Move preflight from `main.rs` into the rename/drop branches of the event dispatcher.
- Add a "mode" enum: preflight runs only when the user picks a history-rewriting menu option.

Flagging here so Phase 6 planning starts from this constraint, not discovers it mid-phase.

## Open Questions (RESOLVED)

1. **Should `read_strip_list` on a non-tool-managed pre-existing hook return an error, or an empty list with a "would refuse to write" sentinel?**
   - What we know: HOOK-06 requires refuse-to-overwrite on *install*. For *read*, behavior is unspecified.
   - What's unclear: Phase 6 will need to call `read_strip_list` to render the "Manage" screen even when no hook exists; what about when a non-tool hook exists?
   - **RESOLVED:** `read_strip_list` returns `HookState::NotToolManaged(PathBuf)` (a third variant alongside `Absent` and `Managed { emails: Vec<String> }`). Phase 6 TUI shows a clear error screen in this case. Engine stays purely declarative. (Adopted in Plan 05-01 `HookState` enum and Plan 05-02 detect_markers behavior.)

2. **Should `install_strip` and `remove_strip` accept the email already-lowercased, or lowercase internally?**
   - **RESOLVED:** lowercase internally. Less footgun-prone for Phase 6 callers. Documented in the function doc that emails are stored case-folded. (Adopted in Plan 05-04 `install_strip` implementation.)

3. **Is there value in including a small Rust-side `apply_strip(message, &list) -> String` helper that mirrors `drop_coauthor_from_message`?**
   - **RESOLVED:** include it. Not strictly required by HOOK-13, but it enables unit-testing the *semantic equivalence* of the Rust and shell sides side-by-side in Rust before shelling out. Marked in HOOK-13's "parse a tool-managed hook back into its strip list" test bucket via the round-trip test in Plan 05-05 (test #12) and the render-then-parse round-trip in Plan 05-03.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `/bin/sh` (POSIX) | Hook execution at commit time + test execution | ✓ | dash on Debian/Ubuntu; bash-in-POSIX-mode on macOS | None — POSIX baseline; mandatory on target platforms |
| `awk` (POSIX) | Hook filter logic | ✓ | mawk / nawk / bwk depending on distro | None — POSIX-mandated; ships with every target OS |
| `mv` (POSIX) | Atomic rewrite inside the hook script | ✓ | coreutils / BSD | None — POSIX-mandated |
| `cargo` / Rust 1.74+ | Build the engine | ✓ | 1.74 (Cargo.toml:5) | None — already established |
| `tempfile` 3.x | Test fixtures (`TempDir`) | ✓ | dev-dependency, Cargo.toml:17 | None — already in tree |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** None.

The entire phase requires only what the codebase already has + POSIX tools the target binary already requires.

## Validation Architecture

> `.planning/config.json` not located at the expected path in this scan; defaulting to enabled per the protocol (key absent = enabled).

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` with built-in `#[test]` + `tests/` integration crate (idiomatic Rust) |
| Config file | None (Cargo's default) |
| Quick run command | `cargo test --test hook_test -- --quiet` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HOOK-04 | Fresh install on repo with no existing hook writes new file with 0755 + markers + email | integration | `cargo test --test hook_test test_install_fresh_writes_file_with_markers_and_email -- --exact` | ❌ Wave 0 |
| HOOK-04 | Install on tool-managed hook appends new email and rewrites file | integration | `cargo test --test hook_test test_install_appends_to_existing_tool_managed_hook -- --exact` | ❌ Wave 0 |
| HOOK-05 | Adding an already-stripped email is a no-op; file bytes unchanged | integration | `cargo test --test hook_test test_install_duplicate_email_is_noop_file_bytes_identical -- --exact` | ❌ Wave 0 |
| HOOK-06 | Refuse-to-overwrite when existing hook lacks tool markers | integration | `cargo test --test hook_test test_install_refuses_to_overwrite_non_tool_managed_hook -- --exact` | ❌ Wave 0 |
| HOOK-07 | File mode is 0755 on Unix after install | integration (Unix only, `#[cfg(unix)]`) | `cargo test --test hook_test test_install_sets_mode_0755_on_unix -- --exact` | ❌ Wave 0 |
| HOOK-07 | Generated hook starts with `#!/bin/sh` and contains BEGIN+END markers | integration | `cargo test --test hook_test test_generated_hook_has_posix_shebang_and_markers -- --exact` | ❌ Wave 0 |
| HOOK-08 | Generated shell script strips matching `Co-authored-by:` lines case-insensitively when run with `/bin/sh` against fixture inputs | integration (shells out) | `cargo test --test hook_test test_shell_hook_strips_case_insensitive_matches -- --exact` | ❌ Wave 0 |
| HOOK-08 | Shell script preserves lines whose target email appears in the name slot but NOT in the email slot (twin-parity counterexample — Pitfall §1) | integration (shells out) | `cargo test --test hook_test test_shell_hook_preserves_when_email_only_in_name_slot -- --exact` | ❌ Wave 0 |
| HOOK-10 | Removing the last entry deletes the hook file entirely | integration | `cargo test --test hook_test test_remove_last_entry_deletes_file -- --exact` | ❌ Wave 0 |
| HOOK-10 | Removing a non-last entry rewrites the file with that email gone | integration | `cargo test --test hook_test test_remove_single_entry_rewrites_file -- --exact` | ❌ Wave 0 |
| HOOK-12 | Hook engine operations do not invoke `check_stash` / `check_worktrees` — verified by installing on a repo with `refs/stash` present | integration | `cargo test --test hook_test test_install_does_not_trigger_preflight_with_stash_present -- --exact` | ❌ Wave 0 |
| HOOK-13 | Parse a tool-managed hook back into its strip list (round-trip) | integration | `cargo test --test hook_test test_read_strip_list_round_trips_through_render -- --exact` | ❌ Wave 0 |

Total: 12 tests, mapping to all 8 phase requirements. (HOOK-13 is the meta-requirement that the test suite exists; each row above contributes.)

### Sampling Rate

- **Per task commit:** `cargo test --test hook_test -- --quiet` (just the new integration test file — fast feedback)
- **Per wave merge:** `cargo test` (full suite — catches regressions in existing rename/drop tests)
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `tests/hook_test.rs` — new file; covers all 12 tests above (HOOK-04, 05, 06, 07, 08, 10, 12, 13)
- [ ] `tests/common/mod.rs` — extend with `run_hook_on_message(hook_path, msg_in) -> String` helper (shells out to `/bin/sh`)
- [ ] `src/hook/mod.rs` + sub-files — new module; this is the implementation, not test infra
- [ ] `src/lib.rs` — add `pub mod hook;`
- [ ] `src/error.rs` — add `HookExists(PathBuf)` variant

No framework install needed — `cargo test` and `tempfile` are already present.

## Security Domain

> `security_enforcement` not located in config; defaulting to enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Local-only tool; no remote authn. |
| V3 Session Management | no | No sessions. |
| V4 Access Control | no | User has full rights to their own `.git/hooks/`. |
| V5 Input Validation | yes | Email strings come from user selection via the existing co-author enumerator (`src/git/reader.rs:37-73`) — already validated structurally as parseable from `<...>`. The hook engine adds: (a) reject empty string, (b) `to_ascii_lowercase()` before embedding. No regex injection vector because `awk` lookup is exact-key, not regex match. |
| V6 Cryptography | no | No crypto. |

### Known Threat Patterns for the hook engine

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Shell injection via embedded email (e.g., user picks an email containing `"; rm -rf / #`) | Tampering | Embed emails inside `strip["…"] = 1` awk-array entries. Awk's string literal does not perform interpolation; the only escape risk is a literal `"` inside the email. Mitigation: reject emails containing `"`, `\`, `\n`, or `\r` at install time. Real-world emails never contain these per RFC 5321, but defense-in-depth applies. |
| Path traversal via `repo.path()` | Tampering | Trust `git2::Repository::path()` — it's the canonical, validated gitdir from libgit2. Don't accept a user-supplied hooks-dir override. |
| TOCTOU between "does hook exist" check and "write hook" | Tampering | Atomic rewrite (Pattern 1): always go through `<path>.tmp.<pid>` + rename. The detection step (read + parse) and the write step are not transactional, but the write step itself is atomic. If a user manually creates a hook between our read and our write, our write replaces it — which is the desired behavior for the tool-managed-hook path (we just verified it was tool-managed). For the non-tool-managed path we refuse, and a TOCTOU between detection and a re-attempted install is no worse than two sequential installs. |
| Malicious commit message tries to exploit `awk` regex DoS | Denial of Service | No regex in the `awk` script's hot path — only literal `tolower()`, `substr()`, hash-table lookup. O(n) per line, constant per character. Safe. |

The hook engine writes a file the user already has full control over (`.git/hooks/`), with permissions consistent with what `git init` would produce. No new attack surface beyond what `git` already exposes.

## Sources

### Primary (HIGH confidence)

- **Codebase (file:line citations)**
  - `src/git/reader.rs:84-92` — `strip_coauthor_prefix` (case-insensitive prefix match)
  - `src/git/reader.rs:95-107` — `parse_coauthor_value` (structural `<...>` extraction)
  - `src/git/rewrite.rs:179-206` — `drop_coauthor_from_message` (canonical drop flow showing all three matching steps)
  - `src/git/preflight.rs:1-15` — `check_stash`, `check_worktrees` (which the hook engine must NOT call, per HOOK-12)
  - `src/main.rs:20-21` — current unconditional preflight calls (Phase 6 will refactor; Phase 5 just refrains)
  - `src/git/mod.rs:1-9` — `open_repo` returning `Repository`; module composition pattern
  - `src/lib.rs:1-3` — top-level module layout (`pub mod error; pub mod git; pub mod tui;` — add `pub mod hook;`)
  - `src/error.rs:3-25` — `AppError` enum; extend with `HookExists(PathBuf)`
  - `Cargo.toml:8-18` — verified deps (`git2 0.21`, `thiserror 2`, `tempfile 3`)
  - `tests/common/mod.rs:6-22` — `create_fixture_repo` (canonical test fixture pattern, uses `TempDir`)
  - `tests/rewrite_test.rs:333-505` — drop_coauthor test cases (canonical for case-insensitive email assertions)
  - `tests/preflight_test.rs:1-65` — preflight test patterns (for HOOK-12 regression test)

- **External (HIGH confidence)**
  - [docs.rs/git2 Repository::path](https://docs.rs/git2/latest/git2/struct.Repository.html) — confirms `path()` returns gitdir
  - [rust-lang.org std::os::unix::fs::PermissionsExt](https://doc.rust-lang.org/std/os/unix/fs/trait.PermissionsExt.html) — `set_mode` / `from_mode` API
  - [POSIX awk specification](https://pubs.opengroup.org/onlinepubs/9699919799/utilities/awk.html) — `tolower`, `substr`, `length`, `gsub` all POSIX

### Secondary (MEDIUM confidence)

- [Writing POSIX-Compatible Shell Scripts for Maximum Portability (OneUptime, 2026)](https://oneuptime.com/blog/post/2026-02-13-posix-shell-compatibility/view) — confirms `awk` portability across dash/ash/bash; `[[ ... ]]` non-POSIX
- [ShellCheck SC2039: In POSIX sh, *something* is undefined](https://www.shellcheck.net/wiki/SC2039) — definitive list of bash-isms to avoid
- [Why Are My Git Pre-Commit and Post-Commit Hooks Not Running? (codestudy.net)](https://www.codestudy.net/blog/git-pre-and-post-commit-hooks-not-running/) — CRLF shebang failure mode

### Tertiary (LOW confidence — marked for validation if used)

- (none relied upon)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every dep is already in `Cargo.toml` and verified by prior phase research.
- Architecture: HIGH for the responsibility map; MEDIUM for the exact module split (5 sub-files vs 1) — judgment call.
- Twin parser specification: HIGH — derived directly from `src/git/reader.rs:84-107` and `src/git/rewrite.rs:179-206` with a concrete counterexample.
- Shell-filter choice (`awk` over `grep`): HIGH — proven by counterexample in §Twin Parser Specification.
- Pitfalls: HIGH — every pitfall is either directly from source code analysis or from cited external sources.
- Tests / Validation Architecture: HIGH — directly mapped to HOOK-13 + each named code path in success criterion #7.

**Research date:** 2026-05-21
**Valid until:** 2026-06-20 (30 days — stable codebase, stable POSIX dependencies, no fast-moving deps)
