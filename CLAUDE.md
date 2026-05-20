# Karpathy Guidelines

Behavioral guidelines to reduce common LLM coding mistakes, derived from [Andrej Karpathy's observations](https://x.com/karpathy/status/2015883857489522876) on LLM coding pitfalls.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

## Rule 5 — Use the model only for judgment calls

Use for: classification, drafting, summarization, extraction.
Do NOT use for: routing, retries, status-code handling, deterministic transforms.
If code can answer, code answers.

## Rule 6 — Token budgets are not advisory

Per-task: 4,000 tokens. Per-session: 30,000 tokens.
If approaching budget, summarize and start fresh.
Surface the breach. Do not silently overrun.

## Rule 7 — Surface conflicts, don't average them

If two patterns contradict, pick one (more recent / more tested).
Explain why. Flag the other for cleanup.
Don't blend conflicting patterns.

## Rule 8 — Read before you write

Before adding code, read exports, immediate callers, shared utilities.
If unsure why existing code is structured a certain way, ask.

## Rule 9 — Tests verify intent, not just behavior

Tests must encode WHY behavior matters, not just WHAT it does.
A test that can't fail when business logic changes is wrong.

<!-- GSD:project-start source:PROJECT.md -->
## Project

**git-author-reformer**

A Rust TUI tool for rewriting git commit author history without external dependencies. It lets developers rename primary commit authors (name + email) across all commits in a repo, and drop co-authors from Co-authored-by trailers. Distributed as a single pre-built binary — download and run with one curl command, no installation required.

**Core Value:** Any developer can clean up git author history in seconds — no Python, no git filter-branch complexity, no installation.

### Constraints

- **Tech stack**: Rust + ratatui + git2 — decided; no alternative considered
- **No external tools**: Binary must work without git installed on the machine
- **Distribution**: Static linking required for the "just curl and run" UX — no dynamic lib dependencies
- **Platforms**: Linux x86_64, macOS aarch64, macOS x86_64 for v1
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommended Stack
### Core
| Library | Version | Role | Confidence |
|---------|---------|------|------------|
| `ratatui` | `0.30.0` | TUI rendering and widget layout | HIGH — verified via `cargo search` |
| `crossterm` | `0.29.0` | Terminal backend for ratatui (cross-platform) | HIGH — verified via `cargo search` |
| `git2` | `0.21.0` | libgit2 bindings: repo open, revwalk, commit creation, ref update | HIGH — verified via `cargo search` |
| `nucleo` | `0.5.0` | Fuzzy matching for author selector (same engine as Helix editor) | HIGH — verified via `cargo search` |
| `tui-textarea` | `0.7.0` | Optional: inline text-edit widget if commit message editing is added | MEDIUM — exists, confirmed ratatui 0.29+ compatible |
| `clap` | `4.6.1` | CLI entry point (`--version`, `--help`); derive macro preferred | HIGH — verified via `cargo search` |
### Rationale
- `Repository::open_from_env()` — detect repo from CWD by walking up (same semantics as git)
- `Revwalk` + `push_glob("*")` — walk ALL commits reachable from ALL refs (all branches, tags)
- `Sort::TOPOLOGICAL | Sort::REVERSE` — walk oldest-first so parent OIDs are always known before children when building the new graph
- `Commit::author()`, `Commit::committer()`, `Commit::time()`, `Commit::message()`, `Commit::tree()`, `Commit::parent_ids()` — read everything needed to reconstruct a commit
- `Repository::commit()` — create a new commit with modified author/message, existing tree, and remapped parent OIDs
- `Repository::reference()` with `force: true` — update each branch/tag ref to point at the new tip after the rewrite
- `Commit::amend()` — available for single-commit amend; for a full graph rewrite, the `Repository::commit()` + ref-update loop is the right pattern (amend only works on the tip of a ref, not arbitrary ancestors)
### What NOT to Use
| Library | Why Not |
|---------|---------|
| `actions-rs/toolchain` | **Archived and unmaintained.** Uses deprecated `node12` and `set-output` GHA features. Every major Rust project has migrated away. Use `dtolnay/rust-toolchain@stable` instead. |
| `cross` (cross-rs) | Docker-based; cannot cross-compile to macOS from Linux without the Apple SDK, which Docker images cannot legally include. Adds complexity with no benefit given the native-runner strategy below. |
| `termion` | Linux/macOS only. crossterm is strictly better for a tool distributed to both platforms. |
| `fuzzy-matcher` | Older, slower than nucleo. No longer maintained as actively. |
| `tokio` / `async-std` | No async I/O needed. TUI event loops are synchronous by nature (crossterm's event polling is blocking with a timeout). Adding an async runtime increases binary size and complexity for zero benefit. |
| Raw `.git/` pack-file parsing | Weeks of complexity to handle all edge cases (thin packs, delta chains, alternates). git2/libgit2 is battle-tested and purpose-built. |
## Cargo.toml Feature Flags for Static Linking
## Cross-Compilation Strategy
### Platform Constraints
| Target | Triplet | Strategy | Notes |
|--------|---------|----------|-------|
| Linux x86_64 | `x86_64-unknown-linux-musl` | musl static | Fully static binary; no glibc dependency, runs on any Linux kernel ≥ 3.2 |
| macOS Apple Silicon | `aarch64-apple-darwin` | Native build on `macos-14` runner | macOS cannot produce fully static binaries; libSystem is always dynamic (OS constraint, not a build tool limitation) |
| macOS Intel | `x86_64-apple-darwin` | Native build on `macos-13` runner | Same macOS constraint; `macos-13` is the last GHA-hosted Intel runner |
### Linux musl — Additional Build Flag
## Build / CI Setup
### GitHub Actions Workflow Structure
### Required Actions (no deprecated tools)
- `dtolnay/rust-toolchain@stable` — maintained by dtolnay (prolific Rust ecosystem contributor), actively updated. Replaces the archived `actions-rs/toolchain`.
- `Swatinem/rust-cache@v2` — the standard Rust CI cache action; caches registry and incremental build artifacts per toolchain version and target.
- `actions-rs/*` — DO NOT USE. Entire organization is archived; uses deprecated GHA features.
### Release Workflow
- `git-author-reformer-linux-x86_64` (musl binary, strip symbols for size)
- `git-author-reformer-macos-aarch64`
- `git-author-reformer-macos-x86_64`
## Minimum Rust Version
## Dependency Tree Risk Assessment
| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| libgit2 C compilation fails on musl | Low | `vendored-libgit2` compiles from source; musl-tools provides the needed headers |
| macOS runner `macos-13` is retired | Medium (Intel Macs deprecated) | Monitor GHA runner announcements; migrate to cross-compile from `macos-14` via `x86_64` cross when needed |
| ratatui breaking API changes | Low | 0.30 is a stable release; pin with `=0.30.0` if needed |
| nucleo API churn | Low | `nucleo` 0.5 is stable; the lower-level `nucleo-matcher` crate is even more stable if needed |
## Sources
- ratatui version: `cargo search ratatui` — `0.30.0`
- git2 version: `cargo search git2` — `0.21.0`
- crossterm version: `cargo search crossterm` — `0.29.0`
- nucleo version: `cargo search nucleo` — `0.5.0`
- clap version: `cargo search clap` — `4.6.1`
- git2 feature flags: [git2-rs Cargo.toml on GitHub](https://github.com/rust-lang/git2-rs/blob/master/Cargo.toml)
- git2 Repository API: [docs.rs/git2/latest/git2/struct.Repository.html](https://docs.rs/git2/latest/git2/struct.Repository.html)
- git2 Commit API: [docs.rs/git2/latest/git2/struct.Commit.html](https://docs.rs/git2/latest/git2/struct.Commit.html)
- git2 Revwalk API: [docs.rs/git2/latest/git2/struct.Revwalk.html](https://docs.rs/git2/latest/git2/struct.Revwalk.html)
- ratatui widgets: [docs.rs/ratatui/latest/ratatui/widgets/index.html](https://docs.rs/ratatui/latest/ratatui/widgets/index.html)
- ratatui event model: no built-in input handling; use crossterm::event directly (official docs)
- actions-rs deprecated: [tauri-apps/tauri issue #8078](https://github.com/tauri-apps/tauri/issues/8078)
- macos-14 = ARM64 only: [actions/runner-images issue #9741](https://github.com/actions/runner-images/issues/9741)
- Swatinem/rust-cache: [GitHub Marketplace](https://github.com/marketplace/actions/rust-cache)
- cargo-zigbuild version: [rust-cross/cargo-zigbuild releases](https://github.com/rust-cross/cargo-zigbuild/releases) — v0.22.3
- macOS static binary constraint: Apple OS design; libSystem always dynamic
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
