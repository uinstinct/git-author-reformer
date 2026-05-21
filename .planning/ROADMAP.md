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

- [x] **Phase 5: Hook Engine** - Pure-Rust module owning the commit-msg hook file format: parse, serialize, ownership detection, idempotent install/extend/remove (completed 2026-05-21)
- [ ] **Phase 6: Hook TUI Integration** - Two new main-menu flows (Add, Manage) wired to the hook engine, with success screens

## Phase Details

> Phases 1–4 shipped in milestone v1.0 (archived 2026-05-21). Details below cover only the active v1.1 phases.

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
**Plans**: 5 plans
- [x] 05-01-PLAN.md — Scaffold src/hook/ module skeleton + AppError::HookExists + lib.rs wiring
- [x] 05-02-PLAN.md — TDD parser: marker-pair detection + strip-list extraction (src/hook/parse.rs)
- [x] 05-03-PLAN.md — TDD renderer: POSIX sh hook template + twin awk filter + email sanitization (src/hook/render.rs)
- [x] 05-04-PLAN.md — Atomic writer + public install_strip/remove_strip/read_strip_list API (src/hook/write.rs, src/hook/mod.rs)
- [x] 05-05-PLAN.md — Integration tests for HOOK-04/05/06/07/08/10/12/13 (tests/hook_test.rs + run_hook_on_message helper)
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
**Plans**: 5 plans
- [x] 06-01-PLAN.md — TDD: Move preflight from main.rs into event.rs Rename/Drop branches (HOOK-12 structural fix)
- [x] 06-02-PLAN.md — Extend MenuChoice (4 options), add 4 Screen variants + stubs, fix modulus (HOOK-01, HOOK-02)
- [ ] 06-03-PLAN.md — TDD: Add flow — HookAddList, install_strip wiring, HookSuccess, HookAlreadyStripped (HOOK-03, HOOK-11)
- [ ] 06-04-PLAN.md — TDD: Manage flow — HookManageList, remove_strip wiring, empty state (HOOK-02, HOOK-09, HOOK-11)
- [ ] 06-05-PLAN.md — HOOK-14 stash-bypass tests + final phase gate (clippy/fmt) (HOOK-14)
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
| 5. Hook Engine | 5/5 | Complete   | 2026-05-21 |
| 6. Hook TUI Integration | 2/5 | In Progress|  |
