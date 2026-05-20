# git-author-reformer

## What This Is

A Rust TUI tool for rewriting git commit author history without external dependencies. It lets developers rename primary commit authors (name + email) across all commits in a repo, and drop co-authors from Co-authored-by trailers. Distributed as a single pre-built binary — download and run with one curl command, no installation required.

## Core Value

Any developer can clean up git author history in seconds — no Python, no git filter-branch complexity, no installation.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] TUI with two top-level operations: "Rename an author" and "Drop a co-author"
- [ ] Rename author: interactive selector showing all primary authors (Name <email> pairs), second selector for target author, rewrites all matching commits
- [ ] Drop co-author: interactive selector showing all Co-authored-by trailer entries, removes selected co-author from every commit it appears in
- [ ] Native git operations via git2 crate (libgit2 statically linked) — no git binary called at runtime
- [ ] Auto-detect repo from current working directory; error if not inside a git repo
- [ ] Pre-rewrite safety: show count of affected commits and ask for confirmation
- [ ] Post-rewrite: show rewrote N commits + force-push reminder with exact command
- [ ] Pre-built binary releases for Linux x86_64, macOS Apple Silicon (aarch64), macOS Intel (x86_64)
- [ ] Single curl command to detect platform, download correct binary, and run it
- [ ] Open source on GitHub with CI for cross-platform binary builds

### Out of Scope

- Windows support — excluded for v1; PowerShell download mechanism is a different UX
- Auto force-push — would require calling git binary, contradicts no-external-tools constraint
- Backup refs (refs/original/) — warn + confirm is sufficient safety for v1
- --dry-run flag — not requested; confirmation prompt serves the same purpose
- Path argument — always operate on current directory

## Context

- Repo name: git-author-reformer
- Language: Rust
- TUI library: ratatui (de facto standard for Rust TUIs)
- Git library: git2 crate (Rust bindings to libgit2, statically linked)
- Distribution: GitHub Releases with binaries built by GitHub Actions CI
- The curl install pattern is: detect OS/arch, download correct binary from GitHub Releases, chmod +x, exec
- Rewriting commits changes their SHA — downstream users of the repo must force-pull after history is rewritten
- Co-authors are stored as free-text trailers in the commit message body: `Co-authored-by: Name <email>`
- Author identity is matched as Name + Email pair (same name with different emails = separate entries)
- All branches in the repo are rewritten, not just current branch (like git filter-branch behavior)

## Constraints

- **Tech stack**: Rust + ratatui + git2 — decided; no alternative considered
- **No external tools**: Binary must work without git installed on the machine
- **Distribution**: Static linking required for the "just curl and run" UX — no dynamic lib dependencies
- **Platforms**: Linux x86_64, macOS aarch64, macOS x86_64 for v1

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| git2 crate over raw .git/ parsing | Raw pack file parsing = weeks of complexity; git2 is battle-tested and purpose-built | — Pending |
| Warn + confirm only (no backup refs) | Simpler UX; user asked for this explicitly | — Pending |
| Name + Email pair for author identity | Prevents accidental merging of distinct identities who share a name | — Pending |
| Rewrite all branches, not just HEAD | Incomplete rewrites leave ghost author in other branches | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-20 after initialization*
