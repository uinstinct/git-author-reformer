# Architecture Patterns

**Domain:** Rust TUI git history rewriting tool
**Researched:** 2026-05-20
**Confidence:** HIGH (git2 API confirmed via Context7/docs.rs; ratatui patterns confirmed via official docs and forum)

---

## Central Concept: The Rewrite Cascade

Commits in git are content-addressed. Rewriting any commit produces a new SHA, which invalidates every descendant's parent pointer, which forces those descendants to be rewritten too, producing new SHAs, and so on to the branch tips. This cascade is the dominant architectural concept; everything else flows from it.

**Cascade steps (proven pattern from git-filter-repo/BFG):**

1. Push all ref tips into a `Revwalk`. For each ref, peel to commit first (`ref.peel_to_commit()`) before pushing — annotated tag refs point to tag objects, not commits, and `revwalk.push(non-commit-oid)` errors at runtime. (Confirmed API: `repo.references()` → iterate, peel, then `revwalk.push(commit_oid)`)
2. Set sort to `Sort::TOPOLOGICAL | Sort::REVERSE` — this yields commits root-first, guaranteeing parents always appear before children (confirmed: `revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)`)
3. Walk the sorted sequence. For each commit, apply the filter predicate and transform function; always remap parent OIDs through `HashMap<Oid, Oid>` before creating the new commit
4. Create the new commit with `repo.commit(None, ...)` — passing `None` as `update_ref` creates an orphan commit object without moving any ref (confirmed API signature from docs.rs). Pass `&original.tree()?` unchanged; the rewriter only modifies commit metadata (author, committer, message), not tree objects.
5. Store `old_oid → new_oid` in the map; unmapped OIDs (untouched commits) pass through as-is
6. After the full walk, update all refs: for each branch/tag that pointed to a commit in the map, set it to the mapped new OID
7. Annotated tags need separate handling: peel the tag object to find the target commit OID, remap through the table, then create a new tag object pointing to the new commit (confirmed: `ref.peel_to_tag()`, `repo.tag(...)`)
8. Symbolic refs (HEAD) are updated last: if HEAD points to a branch ref that was updated, `repo.set_head(refname)` keeps it attached; if HEAD was detached and the commit was rewritten, use `repo.set_head_detached(new_oid)` (confirmed: both methods exist in git2)

**Invariant:** The `HashMap<Oid, Oid>` is the single source of truth for all ref updates. No ref is touched before the walk completes.

---

## Recommended Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  main.rs                                                    │
│  - Repository::discover(cwd)                                │
│  - Construct App, run event loop                            │
└────────────────────┬────────────────────────────────────────┘
                     │
          ┌──────────┴──────────┐
          │                     │
┌─────────▼──────────┐  ┌───────▼─────────────────────────────┐
│  tui/              │  │  git/                               │
│  app.rs            │  │  reader.rs                          │
│  event.rs          │  │  rewriter.rs                        │
│  ui/               │  │  refs.rs                            │
│    screens/        │  └─────────────────────────────────────┘
│      main_menu.rs  │
│      select.rs     │
│      confirm.rs    │
│      result.rs     │
└────────────────────┘
```

---

## Component Boundaries

### `main.rs`

**Responsibility:** Entry point. Call `Repository::discover(std::env::current_dir()?)` and surface an early error if not in a git repo. Pass the `Repository` handle into both the git layer and the TUI, then run the event loop.

**Does NOT own:** Business logic, UI rendering, git operations.

**Communicates with:** `tui::App`, `git::reader`

---

### `git/reader.rs`

**Responsibility:** Read-only enumeration of the repository. Two public functions:

- `enumerate_authors(repo) -> Vec<AuthorIdentity>` — revwalk all ref tips, collect `(name, email)` pairs, deduplicate. Author identity is the `(name, email)` pair; same name with different emails is two separate entries.
- `enumerate_coauthors(repo) -> Vec<CoAuthorEntry>` — same revwalk, parse `Co-authored-by:` trailers from each commit message. **Important:** git2 has no trailer-parsing API (confirmed by absence in docs.rs); parse the message text directly with a line-by-line scan looking for `Co-authored-by:` prefix in the commit message body (lines after the first blank line).

**Does NOT own:** Any writes, UI state.

**Communicates with:** `tui::App` (returns data structures that populate selector lists)

---

### `git/rewriter.rs`

**Responsibility:** The cascade engine. Two public functions sharing the same core algorithm:

- `rename_author(repo, from: AuthorIdentity, to: AuthorIdentity) -> RewriteResult`
- `drop_coauthor(repo, target: CoAuthorEntry) -> RewriteResult`

Both functions take a filter predicate and a transform closure, then execute the cascade:

```
revwalk (topological, reverse) →
  for each commit:
    remap parents through oid_map
    apply transform if matches predicate
    repo.commit(None, author, committer, message, &original.tree()?, &remapped_parents)
    // None = orphan, no ref move; tree is reused unchanged
    oid_map.insert(old, new)
→ return (oid_map, count_rewritten)
```

`RewriteResult` carries the count of rewritten commits plus the `oid_map` for the ref-update phase.

**Does NOT own:** Ref updates, UI, discovery.

**Communicates with:** `git/refs.rs` (hands off `oid_map` after walk completes)

---

### `git/refs.rs`

**Responsibility:** Apply the completed `oid_map` to all repository refs. Steps:

1. Iterate `repo.references()` — collect all branch refs (`refs/heads/*`) and tag refs (`refs/tags/*`)
2. For branch refs: if `ref.target()` is in `oid_map`, call `reference.set_target(new_oid, reflog_msg)`
3. For annotated tag refs: peel to the underlying commit OID, look up in `oid_map`, create new tag object pointing to new commit, update the ref
4. Lightweight tag refs: same as branch refs — just a direct OID pointer
5. HEAD: inspect `repo.head()` — if symbolic (normal case), the branch ref was already updated in step 2; call `repo.set_head(branch_refname)` to re-attach. If detached HEAD: look up the raw OID in `oid_map`, call `repo.set_head_detached(new_oid)`

**Does NOT own:** The cascade walk, UI.

**Communicates with:** `git/rewriter.rs` (receives `oid_map`)

---

### `tui/app.rs`

**Responsibility:** Owns `AppState` enum, drives the event loop, coordinates between the git layer and the rendering layer.

```rust
pub enum AppState {
    MainMenu,
    SelectSourceAuthor { items: Vec<AuthorIdentity>, list_state: ListState },
    SelectTargetAuthor { source: AuthorIdentity, items: Vec<AuthorIdentity>, list_state: ListState },
    SelectCoAuthor { items: Vec<CoAuthorEntry>, list_state: ListState },
    ConfirmRename { from: AuthorIdentity, to: AuthorIdentity, affected_count: usize },
    ConfirmDrop { target: CoAuthorEntry, affected_count: usize },
    Working,
    Result { message: String },
    Error { message: String },
}

pub struct App {
    pub state: AppState,
    pub repo: Repository,
    pub should_quit: bool,
}
```

`handle_event(event) -> ()` applies transitions: key presses move through states, confirmations trigger git operations (synchronous — no async needed for this scope), results transition to `Result` or `Error`.

**Does NOT own:** Rendering logic, git cascade algorithm.

**Communicates with:** `tui/ui/`, `git/reader.rs`, `git/rewriter.rs`, `git/refs.rs`

---

### `tui/ui/` (screens)

**Responsibility:** Pure rendering. Each screen is a function (or impl Widget) that reads from `&App` and calls ratatui draw APIs. No state mutation.

Confirmed ratatui pattern: `terminal.draw(|frame| ui::render(frame, &app))` — the draw closure receives an immutable borrow of `App`. Stateful widgets (list selectors) use `frame.render_stateful_widget(list, area, &mut app.state.list_state)`.

Screen modules:
- `main_menu.rs` — two-item menu: "Rename an author" / "Drop a co-author"
- `select.rs` — generic scrollable list selector; reused for source author, target author, co-author selection
- `confirm.rs` — shows affected count and y/n prompt
- `result.rs` — shows rewritten count and force-push command hint
- `error.rs` — shows error message and exit hint

**Does NOT own:** Any state, git operations.

**Communicates with:** `tui/app.rs` (reads from it, renders to frame)

---

### `tui/event.rs`

**Responsibility:** Crossterm event polling. Confirmed pattern: crossterm `CrosstermBackend` + `enable_raw_mode()` + `EnterAlternateScreen` for setup; `event::poll(Duration)` + `event::read()` for the loop.

Returns typed `Event` variants (key presses, resize) consumed by `App::handle_event`.

---

## Data Flow

```
CWD
  └─► Repository::discover()
        └─► Repository handle (shared)
              │
              ├─► reader::enumerate_authors/coauthors()
              │     └─► Vec<AuthorIdentity> / Vec<CoAuthorEntry>
              │           └─► App::state (selector screen data)
              │                 └─► ui::select renders list
              │
              └─► (on confirm)
                    ├─► rewriter::rename_author / drop_coauthor()
                    │     └─► (oid_map, count)
                    │
                    └─► refs::apply_oid_map(oid_map)
                          └─► Result { count, force_push_hint }
                                └─► ui::result renders message
```

Information flows in one direction per operation: read → display → confirm → rewrite → update refs → display result. No shared mutable state between the git layer and the TUI layer; the TUI holds data returned by git functions as plain Rust structs.

---

## Multi-Screen State Machine

```
MainMenu
  ├── "Rename" ──► SelectSourceAuthor
  │                   └── select ──► SelectTargetAuthor
  │                                     └── select ──► ConfirmRename (affected_count)
  │                                                       ├── confirm ──► Working ──► Result
  │                                                       └── cancel  ──► MainMenu
  │
  └── "Drop" ───► SelectCoAuthor
                    └── select ──► ConfirmDrop (affected_count)
                                     ├── confirm ──► Working ──► Result
                                     └── cancel  ──► MainMenu

Error ◄──── (any git error anywhere)
```

`Working` state runs the rewrite synchronously. Because the operation is CPU-bound and short (not I/O-bound or long-running for typical repos), there is no need for async or background threads. If the rewrite freezes the UI, the terminal frame simply stops updating — acceptable for v1; the result screen appears immediately after.

---

## Module / Crate Structure

```
src/
  main.rs           — discover repo, construct App, run loop
  error.rs          — unified error type (thiserror recommended)
  git/
    mod.rs
    reader.rs       — read-only: author/coauthor enumeration
    rewriter.rs     — cascade engine: topological walk + commit recreation
    refs.rs         — ref updater: apply oid_map to all refs
    types.rs        — AuthorIdentity, CoAuthorEntry, RewriteResult
  tui/
    mod.rs
    app.rs          — App struct, AppState enum, handle_event
    event.rs        — crossterm polling
    ui/
      mod.rs
      screens/
        main_menu.rs
        select.rs
        confirm.rs
        result.rs
        error.rs
```

Single binary crate. No workspace needed at this scope. `git/` and `tui/` are modules not crates — splitting into sub-crates adds complexity without benefit for a tool this size.

Dependencies in `Cargo.toml`:
```toml
[dependencies]
git2 = { version = "0.20", features = ["vendored-libgit2"], default-features = false }
ratatui = "0.29"
crossterm = "0.28"
thiserror = "2"

[profile.release]
strip = true
lto = true
codegen-units = 1
```

Disabling `default-features` on git2 drops the `https` and `ssh` features — this tool never touches remotes, which simplifies static linking significantly on Linux musl. The `vendored-libgit2` feature ensures libgit2 is compiled from source and linked statically.

---

## Build Pipeline: GitHub Actions Cross-Compilation

### Platform Strategy

| Target | Runner | Approach |
|--------|--------|----------|
| `x86_64-unknown-linux-musl` | `ubuntu-latest` | `musl-tools` + rustup target + native cargo; true static binary |
| `x86_64-apple-darwin` (Intel) | `macos-15-intel` | Native build on Intel runner; vendored-libgit2; accepts dynamic libSystem link |
| `aarch64-apple-darwin` (M-series) | `macos-latest` (currently ARM64/M1) | Native build; same approach |

**macOS static linking caveat (HIGH confidence):** Apple does not ship a static `libSystem`. macOS binaries always dynamically link `/usr/lib/libSystem.B.dylib`. This is unavoidable and acceptable — every macOS installation has this library. "Statically linked" for the macOS targets means only libgit2 is vendored; system frameworks are still dynamic. The result is still "download and run" with no user-installed dependencies.

**Linux musl (HIGH confidence):** `x86_64-unknown-linux-musl` with `vendored-libgit2` produces a genuinely self-contained binary with no glibc dependency. `musl-tools` package on Ubuntu provides the musl toolchain. No Docker/cross-rs required for this target.

**Runner label notes (HIGH confidence, verified against GitHub Docs as of 2026-05):**
- `macos-latest` currently points to macOS 15 ARM64 (M1) — use for aarch64 builds
- `macos-15-intel` is the correct label for Intel x86_64 macOS builds; `macos-13` is being retired
- `ubuntu-latest` is ubuntu-24.04 (x86_64)

### Workflow Structure

```yaml
jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            runner: ubuntu-latest
          - target: x86_64-apple-darwin
            runner: macos-15-intel
          - target: aarch64-apple-darwin
            runner: macos-latest

    runs-on: ${{ matrix.runner }}

    steps:
      - uses: actions/checkout@v4

      - name: Install musl tools (Linux only)
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: sudo apt-get install -y musl-tools

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: git-author-reformer-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/git-author-reformer
```

On release tag push, a second job (or `release` workflow) downloads the artifacts and uploads them to GitHub Releases. The install script detects `uname -s` and `uname -m`, constructs the download URL, fetches the binary, `chmod +x`, and execs.

---

## Build Order Implications for Roadmap

Components can be built and tested independently because the git layer has no TUI dependency and vice versa.

**Recommended build sequence:**

| Phase | Components | Rationale |
|-------|-----------|-----------|
| 1. Foundation | `error.rs`, `main.rs` stub, `Repository::discover()` | Everything depends on error types and repo handle; fail-fast on non-repo CWD |
| 2. Read layer | `git/types.rs`, `git/reader.rs` | Pure reads; testable with tmp git repos via `git2::Repository::init()`; no rewriting risk |
| 3. Rewrite engine | `git/rewriter.rs`, `git/refs.rs` | Core algorithm; testable in isolation against fixture repos; independent of TUI |
| 4. TUI shell | `tui/event.rs`, `tui/app.rs`, `tui/ui/*` | Build with stub/hardcoded data first; verify screen flow without git |
| 5. Integration | Wire TUI actions to git layer | Connect `App::handle_event` confirm branches to rewriter calls |
| 6. Distribution | CI matrix, `install.sh` | Build artifacts, test download flow |

Phases 2 and 3 are parallelizable if two engineers are working; they share only `git/types.rs`. Phase 4 (TUI shell) is also parallelizable with phases 2 and 3 since it can run against stubbed data. Phase 5 requires both prior tracks to complete.

---

## Key Architecture Constraints and Edge Cases

**Detached HEAD:** `repo.head()` returns a reference. Check `head.is_branch()` — if false, HEAD is detached (points directly to a commit OID). After refs update, look up the raw HEAD commit OID in `oid_map`; if present, call `repo.set_head_detached(new_oid)`. If not present (HEAD pointed to an untouched commit), leave it alone.

**Merge commits:** The cascade handles them correctly because the topological sort guarantees both parents are processed before the merge commit. The OID map remaps both parent slots independently.

**Annotated tags:** Cannot be handled as simple OID updates because the tag is itself a git object containing the target OID. Must create a new tag object with the new target OID, then update the ref. Failing to handle annotated tags leaves old commit SHAs embedded in the tag objects — the ref points to a new tag object, but the old SHAs persist as dangling objects. (Peel API confirmed: `ref.peel_to_tag()` and `ref.peel_to_commit()` are both available.)

**Dirty working tree:** Refuse to operate. Check `repo.statuses(None)` for non-empty before starting the walk. This is a safety gate, not a backup mechanism.

**GPG signatures:** Any commit with a signature will have that signature invalidated by the rewrite — the new commit object has a different hash. This is unavoidable and expected behavior. Document it in the confirmation screen.

**Submodules:** Treat as out-of-scope; submodule `.gitmodules` entries and submodule commit pointers in trees are untouched. The rewriter only modifies commit objects (author, committer, message), not tree objects.

**Trailer parsing:** git2 (libgit2) has no built-in trailer-parsing API. Parse commit message text directly: split on `\n\n`, iterate lines of the body, match lines starting with `Co-authored-by:` (case-insensitive). Strip and parse the `Name <email>` portion. This is the same approach used by the git CLI's trailer parser at the text level.

---

## Anti-Patterns to Avoid

**Processing refs before the cascade completes.** Updating a branch ref mid-walk corrupts the OID map if later commits still reference the old tip.

**Using `update_ref` in `repo.commit()` during the cascade.** This moves the ref with each commit, which is wrong — refs must only move after the full `oid_map` is built. Pass `None`.

**Calling git binary at runtime.** The tool must work without git installed. All operations go through git2.

**Async / tokio for the event loop.** The operations are synchronous and short. Async adds runtime complexity with no benefit. Standard crossterm polling is sufficient.

**Separate crates for git/ and tui/.** At this project scope, sub-crates slow compilation feedback and add `Cargo.toml` maintenance overhead. Module hierarchy in a single crate is correct.

---

## Sources

- git2 `Revwalk::set_sorting`, `Sort::TOPOLOGICAL | Sort::REVERSE`: https://docs.rs/git2/0.20.2/src/git2/revwalk.rs (HIGH confidence — Context7 verified)
- git2 `Repository::commit(None, ...)`: https://docs.rs/git2/latest/src/git2/repo.rs (HIGH confidence — Context7 verified)
- git2 `Repository::discover()`: https://docs.rs/git2/latest/src/git2/repo.rs (HIGH confidence — Context7 verified)
- git2 `Repository::set_head()`, `set_head_detached()`: https://docs.rs/git2/0.20.2/git2/struct.Repository (HIGH confidence — Context7 verified)
- git2 `Reference::peel_to_tag()`, `peel_to_commit()`: https://docs.rs/git2/0.20.2/src/git2/reference.rs (HIGH confidence — Context7 verified)
- git2 `Repository::references()`, `references_glob()`: https://docs.rs/git2/latest/src/git2/repo.rs (HIGH confidence — Context7 verified)
- git2 `vendored-libgit2` feature, static linking behavior: https://docs.rs/crate/git2/latest (HIGH confidence — docs.rs direct)
- ratatui `StatefulWidget` / `ListState`: https://docs.rs/ratatui/latest/ratatui/widgets/trait.StatefulWidget.html (HIGH confidence — Context7 verified)
- ratatui `CrosstermBackend` setup: https://docs.rs/ratatui/latest/ratatui/backend/struct.CrosstermBackend.html (HIGH confidence — Context7 verified)
- ratatui multi-screen state enum pattern: https://forum.ratatui.rs/t/multiple-screens-in-ratatui/82 (MEDIUM confidence — official ratatui forum, maintainer-confirmed)
- GitHub Actions macOS runner labels (`macos-latest` = ARM64, `macos-15-intel` for Intel): https://docs.github.com/en/actions/reference/runners/github-hosted-runners (HIGH confidence — official GH docs)
- macOS 13 runner retirement: https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/ (HIGH confidence)
- Linux musl cross-compilation approach: https://blog.ediri.io/how-to-cross-compile-your-rust-applications-using-cross-rs-and-github-actions (MEDIUM confidence — community blog, patterns verified against GH docs)
