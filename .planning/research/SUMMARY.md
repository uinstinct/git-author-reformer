# Research Summary: git-author-reformer

**Domain:** Rust TUI git history rewriting tool
**Researched:** 2026-05-20
**Confidence:** HIGH

---

## Recommended Stack

- **ratatui 0.30 + crossterm 0.29** — TUI rendering and cross-platform terminal backend. Use `ratatui::init()` (not `Terminal::new()` directly) — it auto-installs the panic hook needed to restore the terminal on crash.
- **git2 0.21 with `vendored-libgit2`, `default-features = false`** — the entire git object graph rewrite goes through this library. Disabling default features drops SSH/HTTPS which this tool never needs and which cause OpenSSL static-link failure on Linux musl.
- **nucleo 0.5** — fuzzy matching for author/co-author selector lists. Same engine as Helix editor; faster and more actively maintained than `fuzzy-matcher`.
- **clap 4.6 (derive)** — CLI entry point for `--version` and `--help`. Curl-and-run users will try `--help` immediately.
- **thiserror** — unified error type across git and TUI layers.

Do NOT use: `actions-rs/*` GHA actions (archived), `tokio`/async (no async I/O needed), `termion` (Linux/macOS only), raw pack-file parsing, `cross` (Docker macOS cross-compilation blocked by Apple SDK licensing).

---

## Table Stakes Features

**Users will expect these — missing any = product feels broken:**

- Rewrite all branches, not just current branch
- Show commit count per identity before rewriting ("Rewrite 847 commits for alex@old.com?")
- Confirm before executing (single keypress Y/n)
- Rewrite committer field when it matches old author (git stores both; missing this leaves the old identity visible)
- Match by exact Name+Email pair (not name-only or email-only)
- Post-rewrite force-push reminder using `--force-with-lease --all` (not bare `--force`)
- Graceful error if not in a git repo (check at startup, no panic)
- Preserve all other commit metadata byte-for-byte (tree, timestamps, other trailers)
- For co-author drop: case-insensitive key matching, remove all occurrences, preserve non-matching trailers

**Differentiators:**

- Zero runtime dependencies — no Python, no Java, download and run
- Commit count per identity displayed in the selector
- Warn about GPG/SSH-signed commits before rewriting (filter-repo strips silently)
- Warn about tags pointing at rewritten commits
- Warn about stash entries and worktrees before starting (blocking pre-flight)
- Accurate force-push command using detected remote name (not hardcoded `origin main`)

**Defer to v2+:**

- Rename co-author (drop is v1; rename is a distinct, more complex workflow)
- Windows support
- `--path` flag to target a non-CWD repo
- Dry-run flag (redundant with the confirmation prompt + commit count)

---

## Architecture Overview

The dominant concept is the **rewrite cascade**: changing any commit produces a new SHA, forcing all descendants to be rewritten, cascading to branch tips. The `HashMap<OldOid, NewOid>` built during the walk is the single source of truth; no ref is touched until the full walk completes.

**Build order (each layer depends on the previous):**

1. **Foundation** — `error.rs`, `main.rs` stub, `Repository::discover()` — fail-fast on non-repo CWD
2. **Read layer** — `git/types.rs`, `git/reader.rs` — read-only author/co-author enumeration; testable in isolation
3. **Rewrite engine** — `git/rewriter.rs`, `git/refs.rs` — topological walk, OID map, ref updater; testable against fixture repos, independent of TUI
4. **TUI shell** — `tui/event.rs`, `tui/app.rs`, screen modules — buildable with stubbed data in parallel with step 3
5. **Integration** — wire TUI confirm branches to rewriter calls
6. **CI + install.sh** — build matrix, GitHub Release assets, checksum verification

Steps 2, 3, and 4 are parallelizable. Single binary crate (no workspace). `git/` and `tui/` are modules, not sub-crates.

---

## Critical Pitfalls

1. **Annotated tags hold old commit SHAs inside the tag object** — updating `refs/tags/*` is not enough. Must recreate the tag object via `repo.tag()` pointing at the new commit. Must be in same phase as branch ref updating.

2. **Merge commit parent order must be preserved by index** — use `commit.parent_id(i)` in 0..N order, map each through OID table. Never use unordered structure. Swapped parents corrupt `git log --first-parent` and bisect.

3. **Terminal stuck in raw mode on panic or SIGTERM** — use `ratatui::init()` (auto-installs panic hook) and add `signal-hook` handler for SIGTERM that calls `ratatui::restore()`. Fix in TUI scaffolding phase before writing any app logic.

4. **Pre-flight blockers: stash + worktrees** — `refs/stash` entries will be silently broken after rewrite; worktree-locked branches fail silently. Detect both at startup and block with a clear message. Warn (non-blocking) if `refs/notes/commits` exists.

5. **Linux static link: OpenSSL dlopen undefined reference** — building with `vendored-libgit2` on GNU glibc produces `undefined reference to 'dlopen'` from `libcrypto.a`. Solution: always use `x86_64-unknown-linux-musl` target with `default-features = false` on git2.

---

## Phase Implications

**Phase 1: Foundation + Read Layer**
- Delivers: `Repository::discover()` with graceful error, `git/types.rs`, `git/reader.rs` (enumerate authors + co-authors with commit counts), pre-flight checks (stash, worktrees, notes detection)
- Key risk: correct revwalk setup (`Sort::TOPOLOGICAL | Sort::REVERSE`, peel annotated tag refs before pushing to revwalk); case-insensitive co-author key matching

**Phase 2: Rewrite Engine**
- Delivers: `git/rewriter.rs` (topological walk, `repo.commit(None, ...)`, OID map), `git/refs.rs` (branch ref update, annotated tag object recreation, HEAD re-attachment)
- Key risk: annotated tag object recreation (not just ref update); merge commit parent index order; complete OID map before touching any ref

**Phase 3: TUI Shell + Integration**
- Delivers: full TUI with screen state machine, nucleo fuzzy filtering, confirm → rewriter → result, SIGTERM handling
- Key risk: `ratatui::init()` + SIGTERM handler must be wired before any app logic; target author entry is a free-text form (two fields), not a second list picker

**Phase 4: CI + Distribution**
- Delivers: GitHub Actions matrix, `install.sh` with checksum verification, stripped/optimized binaries
- Key risk: never use `actions-rs/*`; musl for Linux; native macOS runners (not cross-compile); verify runner labels at implementation time

---

## Open Questions for Roadmapper

1. **Target author input UX**: needs a free-text form (new name + new email fields), not a second list picker
2. **Committer field rewrite**: should silently match committer to old author with no UI toggle (v1 default)
3. **Selector sort order**: sort by commit count descending
4. **Dirty working tree gate**: refuse to operate if `repo.statuses(None)` is non-empty
5. **Email deduplication for co-author list**: normalize to lowercase for dedup; match case-insensitively during removal
