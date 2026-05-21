# Requirements: git-author-reformer

**Defined:** 2026-05-20 (v1.0), 2026-05-21 (v1.1)
**Core Value:** Any developer can clean up git author history in seconds — no Python, no git filter-branch complexity, no installation.

## v1.1 Requirements

### Auto-Strip Hook

- [ ] **HOOK-01**: TUI main menu shows a third option **"Add co-author auto-strip hook"** alongside the existing rename and drop options
- [ ] **HOOK-02**: TUI main menu shows a fourth option **"Manage auto-strip hook"** alongside the other three options; this option is always visible (works even when no hook is installed, showing an empty state)
- [ ] **HOOK-03**: When the user picks "Add", the tool shows the list of currently-stripped emails (or "no entries yet" if none), then presents a fuzzy-filterable co-author list reusing the same enumeration as the existing drop flow
- [ ] **HOOK-04**: When the user selects a co-author to add, the tool writes `.git/hooks/commit-msg` if absent, or rewrites it with the email appended to the embedded strip list if present and tool-managed
- [ ] **HOOK-05**: Adding an email already present in the strip list is a no-op shown to the user as "already stripped: <email>" — the hook file is not rewritten
- [ ] **HOOK-06**: If `.git/hooks/commit-msg` exists and does NOT contain the tool's marker comment, the tool refuses to overwrite, displays a clear error naming the file and instructing the user to remove or rename it, and exits the flow without writing anything
- [ ] **HOOK-07**: The generated hook file is POSIX `sh` (shebang `#!/bin/sh`, no bash-isms) and is created with executable permissions (mode 0755 on Unix); the strip list lives between two distinctive marker comments inside the file
- [ ] **HOOK-08**: The hook strips lines from the commit message using the SAME case-insensitive `Co-authored-by:` matching semantics as the existing drop flow — trailing whitespace and trailer key case must match the parser's behavior
- [ ] **HOOK-09**: When the user picks "Manage", the tool shows a fuzzy-filterable list of configured strip emails; selecting an entry removes it from the list and rewrites the hook file
- [ ] **HOOK-10**: Removing the last entry from the strip list deletes the hook file entirely — no empty marker file is left behind
- [ ] **HOOK-11**: Both "Add" and "Manage" flows end on a success screen showing the resulting strip-list state (e.g. "Hook installed — stripping 2 emails: a@x.com, b@y.com" or "Hook removed — no entries remain")
- [ ] **HOOK-12**: Hook install/manage operations do NOT trigger the existing stash/worktree pre-flight blockers (SAFE-01, SAFE-02) — installing a hook does not rewrite history
- [ ] **HOOK-13**: Hook engine has automated Rust tests covering every code path: fresh install on a repo with no hook, append-to-existing tool-managed hook, no-op when adding a duplicate email, refuse-to-overwrite a non-tool-managed hook, remove a single entry, remove the last entry (hook file deleted), parse a tool-managed hook back into its strip list, mode 0755 is set, and the generated shell script correctly strips matching `Co-authored-by:` lines case-insensitively when executed against fixture commit messages
- [ ] **HOOK-14**: Hook TUI flows have automated tests covering every user path: main menu shows all four options and routes each correctly, "Add" happy path lands on success screen with updated list, "Add" with already-stripped email lands on no-op screen, "Manage" empty state renders when no hook installed, "Manage" remove-single-entry lands on success screen with remaining list, "Manage" remove-last-entry lands on "hook removed" screen, and neither flow invokes the SAFE-01/SAFE-02 preflight on a repo with stash entries

## v1.0 Requirements — Shipped 2026-05-20

### Core

- [x] **CORE-01**: User is presented with a two-option main menu on launch: "Rename an author" and "Drop a co-author"
- [x] **CORE-02**: Tool auto-detects the git repo from the current working directory; shows a clear error and exits if not inside a git repo
- [x] **CORE-03**: All git operations use the git2 crate (libgit2, vendored, no SSH/HTTPS features); no git binary is called at runtime

### Rename Author

- [x] **RENAME-01**: User sees a list of all primary commit authors (Name + Email pairs) with commit count per identity, filterable with fuzzy search
- [x] **RENAME-02**: After selecting the source author, user enters the new name and new email via a two-field free-text form (not a second list picker)
- [x] **RENAME-03**: Tool rewrites all matching commits across all branches, updating both the author and committer fields when the committer matches the old author identity
- [x] **RENAME-04**: Annotated tag objects pointing at rewritten commits are recreated (not just the ref pointer — the tag object itself is updated with the new target SHA)
- [x] **RENAME-05**: Before rewriting, tool shows the exact count of affected commits and prompts for confirmation (Y/n) that the user must explicitly answer

### Drop Co-author

- [x] **DROP-01**: User sees a list of all unique co-authors from Co-authored-by trailers across all commits, with commit count per identity, filterable with fuzzy search
- [x] **DROP-02**: Tool removes the selected co-author from all Co-authored-by trailers using case-insensitive key matching; removes all occurrences within a single commit if duplicated
- [x] **DROP-03**: All other Co-authored-by entries and other commit metadata (tree, timestamps, other trailers, commit message body) are preserved byte-for-byte
- [x] **DROP-04**: Before rewriting, tool shows the exact count of affected commits and prompts for confirmation (Y/n) that the user must explicitly answer

### Safety

- [x] **SAFE-01**: Tool blocks the operation (with a descriptive error) if stash entries are detected — stash references commits that will be orphaned after rewrite
- [x] **SAFE-02**: Tool blocks the operation (with a descriptive error) if linked worktrees are detected — locked branches cannot be updated
- [x] **SAFE-03**: Tool shows a non-blocking warning if any affected commits have GPG or SSH signatures — rewriting invalidates them; user can still proceed
- [x] **SAFE-04**: Tool shows a non-blocking warning if annotated tags point at commits being rewritten — the tag objects will be recreated; user can still proceed
- [x] **SAFE-05**: Tool shows a non-blocking warning if refs/notes/commits exists — notes reference old SHAs and will be orphaned; user can still proceed

### Post-Rewrite Output

- [x] **OUT-01**: After a successful rewrite, tool shows the count of rewritten commits and a force-push reminder using the detected remote name and `git push --force-with-lease --all`

### Distribution

- [x] **DIST-01**: Pre-built static binary for Linux x86_64 (musl target — genuinely no dynamic dependencies)
- [x] **DIST-02**: Pre-built binary for macOS Apple Silicon / aarch64 (built on native macOS runner, not cross-compiled)
- [x] **DIST-03**: Pre-built binary for macOS Intel / x86_64 (built on native macOS Intel runner, not cross-compiled)
- [x] **DIST-04**: Single curl command detects OS/arch, downloads the correct binary from GitHub Releases, verifies SHA256 checksum, and runs the tool
- [x] **DIST-05**: GitHub Actions CI builds and uploads release binaries on git tag push

## Future Requirements (deferred)

### Extended Operations

- **EXT-01**: Rename a co-author (distinct from drop; requires new name+email input after selection)
- **EXT-02**: `--path` flag to specify a repo directory instead of using CWD

### Extended Safety

- **EXT-03**: `--dry-run` flag to preview what would be rewritten without modifying anything

### Extended Platform Support

- **EXT-04**: Windows binary support with PowerShell download command

### Extended Hook Support

- **EXT-05**: Global hook installation via `core.hooksPath` (repo-local only in v1.1)
- **EXT-06**: Built-in AI author pattern list (Claude, Copilot, Cursor, GPT…) for one-click "strip all known AI co-authors"
- **EXT-07**: Append the tool's strip block to a pre-existing non-tool-written `commit-msg` hook (refuse-to-overwrite in v1.1)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Calling the git binary | Contradicts no-external-tools constraint; git2 covers all needed operations |
| Auto force-push after rewrite | Would require calling git binary; user can copy-paste the shown command |
| Backup refs (refs/original/) | Warn + confirm is sufficient safety; git reflog provides 90-day local recovery |
| Windows v1 support | Different download mechanism (PowerShell), adds CI complexity before core is validated |
| Dry-run flag | Confirmation prompt + commit count serves the same purpose for v1 |
| Mailmap integration | Different tool — mailmap is display-layer only; this tool modifies commit objects |
| Undo/rollback command | git reflog provides recovery; document this in post-rewrite output |
| post-commit hook for stripping (v1.1) | Would force a `git commit --amend` and change SHA after the fact; `commit-msg` does it cleanly before the commit object is created |
| Built-in AI author list (v1.1) | User picks from observed co-authors only; avoids stale curated list and false positives |
| Global hooks via `core.hooksPath` (v1.1) | Repo-local hooks are simpler and match the tool's per-repo operation model |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CORE-01 | Phase 3 | Complete |
| CORE-02 | Phase 1 | Complete |
| CORE-03 | Phase 1 | Complete |
| RENAME-01 | Phase 3 | Complete |
| RENAME-02 | Phase 3 | Complete |
| RENAME-03 | Phase 2 | Complete |
| RENAME-04 | Phase 2 | Complete |
| RENAME-05 | Phase 3 | Complete |
| DROP-01 | Phase 3 | Complete |
| DROP-02 | Phase 2 | Complete |
| DROP-03 | Phase 2 | Complete |
| DROP-04 | Phase 3 | Complete |
| SAFE-01 | Phase 1 | Complete |
| SAFE-02 | Phase 1 | Complete |
| SAFE-03 | Phase 3 | Complete |
| SAFE-04 | Phase 3 | Complete |
| SAFE-05 | Phase 3 | Complete |
| OUT-01 | Phase 3 | Complete |
| DIST-01 | Phase 4 | Complete |
| DIST-02 | Phase 4 | Complete |
| DIST-03 | Phase 4 | Complete |
| DIST-04 | Phase 4 | Complete |
| DIST-05 | Phase 4 | Complete |
| HOOK-01 | Phase 6 | Pending |
| HOOK-02 | Phase 6 | Pending |
| HOOK-03 | Phase 6 | Pending |
| HOOK-04 | Phase 5 | Pending |
| HOOK-05 | Phase 5 | Pending |
| HOOK-06 | Phase 5 | Pending |
| HOOK-07 | Phase 5 | Pending |
| HOOK-08 | Phase 5 | Pending |
| HOOK-09 | Phase 6 | Pending |
| HOOK-10 | Phase 5 | Pending |
| HOOK-11 | Phase 6 | Pending |
| HOOK-12 | Phase 5 | Pending |
| HOOK-13 | Phase 5 | Pending |
| HOOK-14 | Phase 6 | Pending |

**Coverage:**
- v1.0 requirements: 23 total (mapped, shipped)
- v1.1 requirements: 14 total (mapped: 8 → Phase 5, 6 → Phase 6)

---
*Requirements defined: 2026-05-20*
*Last updated: 2026-05-21 after v1.1 roadmap mapping*
