# Roadmap: git-author-reformer

## Overview

git-author-reformer is built in four phases on a strict dependency chain. Phase 1 lays the foundation: repo detection, pre-flight safety blockers, and a fully-tested read layer that enumerates authors and co-authors with commit counts. Phase 2 builds the rewrite engine in complete isolation from any UI — topological walk, OID map, branch ref updates, and annotated tag object recreation. Phase 3 wires a full ratatui TUI shell to the engine, delivering both operations end-to-end. Phase 4 ships pre-built static binaries via GitHub Actions and a single curl install command.

Milestone v1.1 (Auto-Strip Co-Author Hook) extends the tool with two additional main-menu operations. It follows the same engine-then-TUI split: Phase 5 builds a pure-Rust hook engine (file format, parser, serializer, ownership detection), then Phase 6 wires two new TUI flows on top.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

### Milestone v1.0 — Shipped 2026-05-20

- [x] **Phase 1: Foundation + Read Layer** - Repo detection, pre-flight safety blockers, and read-only author/co-author enumeration
- [x] **Phase 2: Rewrite Engine** - Commit cascade engine across all branches with annotated tag recreation and correct merge parent ordering
- [x] **Phase 3: TUI + Integration** - Full ratatui TUI wired to the rewrite engine — both operations end-to-end (completed 2026-05-20)
- [x] **Phase 4: CI + Distribution** - Pre-built static binaries on GitHub Releases with a single curl install command (completed 2026-05-20)

### Milestone v1.1 — Auto-Strip Co-Author Hook

- [ ] **Phase 5: Hook Engine** - Pure-Rust module owning the commit-msg hook file format: parse, serialize, ownership detection, idempotent install/extend/remove
- [ ] **Phase 6: Hook TUI Integration** - Two new main-menu flows (Add, Manage) wired to the hook engine, with success screens

## Phase Details

### Phase 1: Foundation + Read Layer
**Goal**: Solid repo detection, author enumeration, and pre-flight safety checks with no writes
**Depends on**: Nothing (first phase)
**Requirements**: CORE-02, CORE-03, SAFE-01, SAFE-02
**Success Criteria** (what must be TRUE):
  1. Running the binary outside a git repository exits immediately with a descriptive error message and a non-zero exit code
  2. A repo containing stash entries or linked worktrees is detected at startup and blocked with a clear message — no rewrite proceeds
  3. Enumerating authors on a fixture repo returns the correct Name+Email pairs with accurate per-identity commit counts, sorted by count descending
  4. Enumerating co-authors parses Co-authored-by trailers case-insensitively and returns unique identities with accurate commit counts
**Plans**: 4 plans
  - [ ] 01-01-PLAN.md — Cargo manifest, AppError, types, module scaffolding, shared test fixtures
  - [ ] 01-02-PLAN.md — TDD: preflight gates (check_stash, check_worktrees)
  - [ ] 01-03-PLAN.md — TDD: read layer (enumerate_authors, enumerate_coauthors)
  - [ ] 01-04-PLAN.md — main.rs wiring + end-to-end CLI tests

### Phase 2: Rewrite Engine
**Goal**: The commit cascade engine — rewrite commits across all branches with correct parent mapping, handle annotated tags, no TUI
**Depends on**: Phase 1
**Requirements**: RENAME-03, RENAME-04, DROP-02, DROP-03
**Success Criteria** (what must be TRUE):
  1. After a rename operation on a fixture repo, `git log --all` shows zero occurrences of the old author identity across all branches
  2. Annotated tag objects pointing at rewritten commits are recreated (not just the ref pointer), verified via `git cat-file tag <tag>` showing the new target SHA
  3. Merge commit parent order is preserved byte-for-byte — `git log --first-parent` and `git bisect` produce identical results before and after rewrite
  4. After a co-author drop, all other trailers, commit message bodies, trees, and timestamps are byte-identical to the originals
**Plans**: 3 plans
  - [ ] 02-01-PLAN.md — Scaffolding: pub(crate) visibility on reader trailer helpers, empty rewrite module, fixture helpers (create_branch, add_merge_commit, create_annotated_tag)
  - [ ] 02-02-PLAN.md — TDD: rewrite_author (RENAME-03, RENAME-04) with merge parent order, annotated tag recreation, conditional committer rewrite, detached HEAD
  - [ ] 02-03-PLAN.md — TDD: drop_coauthor + drop_coauthor_from_message (DROP-02, DROP-03) with case-insensitive match, duplicates, byte-identity preservation
**Key constraints**:
- Annotated tag object recreation must occur in the same phase as branch ref updating — do not defer the tag object rewrite to Phase 3
- Merge commit parent order must be preserved by index (`commit.parent_id(i)` in 0..N order, mapped through OID table) — never use an unordered structure

### Phase 3: TUI + Integration
**Goal**: Full ratatui TUI shell wired to the git layer — both rename and drop operations end-to-end
**Depends on**: Phase 2
**Requirements**: CORE-01, RENAME-01, RENAME-02, RENAME-05, DROP-01, DROP-04, SAFE-03, SAFE-04, SAFE-05, OUT-01
**Success Criteria** (what must be TRUE):
  1. Launching the tool presents a two-option main menu ("Rename an author" / "Drop a co-author") and responds to keyboard navigation
  2. The rename flow shows a fuzzy-filterable author list, then a two-field free-text form (new name + new email), then a confirmation prompt showing exact affected commit count before any write
  3. The drop flow shows a fuzzy-filterable co-author list, then a confirmation prompt showing exact affected commit count before any write
  4. Non-blocking warnings for GPG/SSH signatures, annotated tags, and refs/notes/commits are displayed before the confirmation prompt — user can still proceed
  5. After a successful rewrite, the tool shows the count of rewritten commits and a force-push reminder using the detected remote name
**Plans**: 5 plans
  - [x] 03-01-PLAN.md — TDD: git::scan module (RewritePreview, scan_rename, scan_drop) + Cargo deps + empty tui module skeleton
  - [x] 03-02-PLAN.md — TUI shell: SIGTERM-aware ratatui init/restore in main.rs + App state machine + main menu (CORE-01)
  - [x] 03-03-PLAN.md — Rename flow: fuzzy-filterable author list + two-field rename form (RENAME-01, RENAME-02)
  - [x] 03-04-PLAN.md — Drop flow: fuzzy-filterable co-author list to Preview placeholder (DROP-01)
  - [x] 03-05-PLAN.md — Preview + warnings + execute + result: scan integration, Y/N confirm, success screen with force-push reminder (RENAME-05, DROP-04, SAFE-03, SAFE-04, SAFE-05, OUT-01)
**Key constraints**:
- `ratatui::init()` and a SIGTERM handler (calling `ratatui::restore()`) must be the first code written — before any app logic — to prevent terminal stuck in raw mode on panic or signal
- Target author entry is a free-text two-field form (new name + new email), not a second list picker
**UI hint**: yes

### Phase 4: CI + Distribution
**Goal**: Pre-built static binaries on GitHub Releases, curl install command
**Depends on**: Phase 3
**Requirements**: DIST-01, DIST-02, DIST-03, DIST-04, DIST-05
**Success Criteria** (what must be TRUE):
  1. Running the curl install command on Linux x86_64 downloads the correct binary, verifies its SHA256 checksum, and executes the tool
  2. Running the curl install command on macOS Apple Silicon (aarch64) and macOS Intel (x86_64) each downloads the correct binary, verifies its checksum, and executes the tool
  3. Pushing a git tag triggers the GitHub Actions CI workflow, which builds and uploads all three release binaries automatically
  4. The Linux binary has no dynamic library dependencies (verified with `ldd` showing "not a dynamic executable")
**Plans**: 2 plans
  - [x] 04-01-PLAN.md — GitHub Actions release workflow: 3-platform matrix (linux-musl, macos-aarch64, macos-x86_64-intel), SHA256 upload, ldd verification
  - [x] 04-02-PLAN.md — POSIX sh install script: platform detection, checksum-before-chmod, trap cleanup + test harness
**Key constraints**:
- Linux target must be `x86_64-unknown-linux-musl` (musl, not glibc) to guarantee genuinely no dynamic dependencies — glibc build produces `undefined reference to 'dlopen'` from libcrypto.a
- macOS aarch64 and x86_64 binaries must be built on native macOS runners, not cross-compiled — Apple SDK licensing blocks Docker-based cross-compilation
- Never use `actions-rs/*` GitHub Actions (archived) — use `dtolnay/rust-toolchain` or shell commands directly

---

## Milestone v1.1: Auto-Strip Co-Author Hook

### Phase 5: Hook Engine
**Goal**: Pure-Rust module that owns the `commit-msg` hook file end-to-end — read, parse, serialize, install, extend, remove — with no TUI dependencies
**Depends on**: Phase 2 (reuses the case-insensitive `Co-authored-by:` matching semantics from the drop flow; same parser code path embedded in the hook script's filter logic)
**Requirements**: HOOK-04, HOOK-05, HOOK-06, HOOK-07, HOOK-08, HOOK-10, HOOK-12, HOOK-13
**Success Criteria** (what must be TRUE):
  1. Installing a strip entry on a repo with no existing `commit-msg` hook writes a POSIX `sh` script (shebang `#!/bin/sh`) at `.git/hooks/commit-msg` with mode 0755 and the email listed between marker comments
  2. Installing a strip entry on a repo whose `commit-msg` hook already carries the tool's marker comment appends the new email to the embedded list and rewrites the file; installing an email already in the list is reported as "already stripped" and the file's bytes are unchanged
  3. Installing a strip entry on a repo whose `commit-msg` hook exists but lacks the tool's marker comment returns a refuse-to-overwrite error naming the file and instructing the user to remove or rename it — no file is written
  4. Removing the last entry from a tool-managed hook deletes the `.git/hooks/commit-msg` file entirely (no empty marker-only file is left behind)
  5. Executing the generated hook against a sample commit message strips lines matching `Co-authored-by:` case-insensitively for any email in the embedded list, using the same matching semantics as the existing drop flow (verified by running the script with `sh` on fixture inputs)
  6. Calling any hook engine operation does not invoke the SAFE-01/SAFE-02 preflight blockers — a repo with stash entries or linked worktrees can still have hooks installed or managed
  7. Automated Rust tests cover every engine code path: fresh install, append-to-existing, no-op duplicate, refuse-to-overwrite, parse tool-managed hook back into strip list, remove single entry, remove last entry (file deleted), mode 0755 verified on Unix, and shell-script execution against fixture commit messages — `cargo test` exercises each path with a dedicated test
**Plans**: TBD
**Key constraints**:
- The hook's runtime filter (the `sh` script that strips lines) must use the SAME case-insensitive `Co-authored-by:` matching semantics as the Rust drop flow (HOOK-08). The Rust drop parser is the source of truth; the shell filter is its faithful POSIX reimplementation. Document this twin-source explicitly in plans.
- The Rust side writes a fixed shell script template with the strip list embedded between two distinctive marker comments. The script itself contains no Rust — it is plain POSIX `sh` using `grep -i` / `sed` to filter `Co-authored-by:` lines whose emails match the embedded list.
- Hook engine entry points must NOT call the existing preflight gates (`check_stash`, `check_worktrees`); installing a hook is a non-history-rewriting operation (HOOK-12). The TUI in Phase 6 routes hook flows around the preflight, not through it.
- File mode 0755 must be set on Unix; on Windows (not a v1 target) this is a no-op. The Rust file write path should use `std::os::unix::fs::PermissionsExt`.

### Phase 6: Hook TUI Integration
**Goal**: Two new main-menu flows (Add, Manage) wired to the hook engine, with fuzzy-filterable selectors and success screens
**Depends on**: Phase 5
**Requirements**: HOOK-01, HOOK-02, HOOK-03, HOOK-09, HOOK-11, HOOK-14
**Success Criteria** (what must be TRUE):
  1. Launching the tool presents a four-option main menu — "Rename an author", "Drop a co-author", "Add co-author auto-strip hook", "Manage auto-strip hook" — and responds to keyboard navigation
  2. The "Manage auto-strip hook" option is always visible and selectable, even when no hook is installed; in that empty state it shows a clear "no entries configured" screen
  3. Picking "Add" displays the currently-configured strip list (or "no entries yet"), then a fuzzy-filterable co-author selector reusing the same enumeration as the existing drop flow; selecting an entry hands off to the hook engine and lands on a success screen showing the resulting strip-list state
  4. Picking "Manage" displays a fuzzy-filterable list of configured strip emails; selecting an entry removes it via the hook engine and lands on a success screen showing the resulting strip-list state (or "hook removed — no entries remain" when the last entry was removed)
  5. Neither Add nor Manage triggers the stash/worktree pre-flight blockers — both flows reach their selectors on a repo with stash entries
  6. Automated TUI/state-machine tests cover every user path: main menu routes each of the four options, Add happy path → success screen, Add duplicate → already-stripped screen, Manage empty state, Manage remove single → updated list, Manage remove last → "hook removed" screen, and a regression test verifies Add/Manage on a repo with stash entries does NOT hit the SAFE-01/SAFE-02 preflight
**Plans**: TBD
**Key constraints**:
- The co-author enumeration in the Add flow must reuse the existing `enumerate_coauthors` from Phase 1, not a parallel implementation (HOOK-03).
- The Add and Manage flows must dispatch to the hook engine on a code path that bypasses the SAFE-01/SAFE-02 preflight (HOOK-12). Audit the App state machine for any unconditional preflight call before adding the new transitions.
- Success screens for both flows render the final strip-list state from the hook engine, not from a cached TUI value — the engine is the source of truth (HOOK-11).
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation + Read Layer | 4/4 | Complete    | 2026-05-20 |
| 2. Rewrite Engine | 3/3 | Complete    | 2026-05-20 |
| 3. TUI + Integration | 5/5 | Complete   | 2026-05-20 |
| 4. CI + Distribution | 2/2 | Complete   | 2026-05-20 |
| 5. Hook Engine | 0/TBD | Not started | - |
| 6. Hook TUI Integration | 0/TBD | Not started | - |
