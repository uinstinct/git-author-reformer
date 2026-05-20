# Requirements: git-author-reformer

**Defined:** 2026-05-20
**Core Value:** Any developer can clean up git author history in seconds — no Python, no git filter-branch complexity, no installation.

## v1 Requirements

### Core

- [ ] **CORE-01**: User is presented with a two-option main menu on launch: "Rename an author" and "Drop a co-author"
- [ ] **CORE-02**: Tool auto-detects the git repo from the current working directory; shows a clear error and exits if not inside a git repo
- [ ] **CORE-03**: All git operations use the git2 crate (libgit2, vendored, no SSH/HTTPS features); no git binary is called at runtime

### Rename Author

- [ ] **RENAME-01**: User sees a list of all primary commit authors (Name + Email pairs) with commit count per identity, filterable with fuzzy search
- [ ] **RENAME-02**: After selecting the source author, user enters the new name and new email via a two-field free-text form (not a second list picker)
- [ ] **RENAME-03**: Tool rewrites all matching commits across all branches, updating both the author and committer fields when the committer matches the old author identity
- [ ] **RENAME-04**: Annotated tag objects pointing at rewritten commits are recreated (not just the ref pointer — the tag object itself is updated with the new target SHA)
- [ ] **RENAME-05**: Before rewriting, tool shows the exact count of affected commits and prompts for confirmation (Y/n) that the user must explicitly answer

### Drop Co-author

- [ ] **DROP-01**: User sees a list of all unique co-authors from Co-authored-by trailers across all commits, with commit count per identity, filterable with fuzzy search
- [ ] **DROP-02**: Tool removes the selected co-author from all Co-authored-by trailers using case-insensitive key matching; removes all occurrences within a single commit if duplicated
- [ ] **DROP-03**: All other Co-authored-by entries and other commit metadata (tree, timestamps, other trailers, commit message body) are preserved byte-for-byte
- [ ] **DROP-04**: Before rewriting, tool shows the exact count of affected commits and prompts for confirmation (Y/n) that the user must explicitly answer

### Safety

- [ ] **SAFE-01**: Tool blocks the operation (with a descriptive error) if stash entries are detected — stash references commits that will be orphaned after rewrite
- [ ] **SAFE-02**: Tool blocks the operation (with a descriptive error) if linked worktrees are detected — locked branches cannot be updated
- [ ] **SAFE-03**: Tool shows a non-blocking warning if any affected commits have GPG or SSH signatures — rewriting invalidates them; user can still proceed
- [ ] **SAFE-04**: Tool shows a non-blocking warning if annotated tags point at commits being rewritten — the tag objects will be recreated; user can still proceed
- [ ] **SAFE-05**: Tool shows a non-blocking warning if refs/notes/commits exists — notes reference old SHAs and will be orphaned; user can still proceed

### Post-Rewrite Output

- [ ] **OUT-01**: After a successful rewrite, tool shows the count of rewritten commits and a force-push reminder using the detected remote name and `git push --force-with-lease --all`

### Distribution

- [x] **DIST-01**: Pre-built static binary for Linux x86_64 (musl target — genuinely no dynamic dependencies)
- [x] **DIST-02**: Pre-built binary for macOS Apple Silicon / aarch64 (built on native macOS runner, not cross-compiled)
- [x] **DIST-03**: Pre-built binary for macOS Intel / x86_64 (built on native macOS Intel runner, not cross-compiled)
- [x] **DIST-04**: Single curl command detects OS/arch, downloads the correct binary from GitHub Releases, verifies SHA256 checksum, and runs the tool
- [x] **DIST-05**: GitHub Actions CI builds and uploads release binaries on git tag push

## v2 Requirements

### Extended Operations

- **EXT-01**: Rename a co-author (distinct from drop; requires new name+email input after selection)
- **EXT-02**: `--path` flag to specify a repo directory instead of using CWD

### Extended Safety

- **EXT-03**: `--dry-run` flag to preview what would be rewritten without modifying anything

### Extended Platform Support

- **EXT-04**: Windows binary support with PowerShell download command

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

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CORE-01 | Phase 3 | Pending |
| CORE-02 | Phase 1 | Pending |
| CORE-03 | Phase 1 | Pending |
| RENAME-01 | Phase 3 | Pending |
| RENAME-02 | Phase 3 | Pending |
| RENAME-03 | Phase 2 | Pending |
| RENAME-04 | Phase 2 | Pending |
| RENAME-05 | Phase 3 | Pending |
| DROP-01 | Phase 3 | Pending |
| DROP-02 | Phase 2 | Pending |
| DROP-03 | Phase 2 | Pending |
| DROP-04 | Phase 3 | Pending |
| SAFE-01 | Phase 1 | Pending |
| SAFE-02 | Phase 1 | Pending |
| SAFE-03 | Phase 3 | Pending |
| SAFE-04 | Phase 3 | Pending |
| SAFE-05 | Phase 3 | Pending |
| OUT-01 | Phase 3 | Pending |
| DIST-01 | Phase 4 | Complete |
| DIST-02 | Phase 4 | Complete |
| DIST-03 | Phase 4 | Complete |
| DIST-04 | Phase 4 | Complete |
| DIST-05 | Phase 4 | Complete |

**Coverage:**
- v1 requirements: 23 total
- Mapped to phases: 23
- Unmapped: 0 ✓

---
*Requirements defined: 2026-05-20*
*Last updated: 2026-05-20 after initial definition*
