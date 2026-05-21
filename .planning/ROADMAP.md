# Roadmap: git-author-reformer

## Overview

git-author-reformer is built in four phases on a strict dependency chain. Phase 1 lays the foundation: repo detection, pre-flight safety blockers, and a fully-tested read layer that enumerates authors and co-authors with commit counts. Phase 2 builds the rewrite engine in complete isolation from any UI — topological walk, OID map, branch ref updates, and annotated tag object recreation. Phase 3 wires a full ratatui TUI shell to the engine, delivering both operations end-to-end. Phase 4 ships pre-built static binaries via GitHub Actions and a single curl install command.

Milestone v1.1 (shipped 2026-05-21) added two new main-menu operations following the same engine-then-TUI split: Phase 5 built a pure-Rust hook engine (file format, parser, serializer, ownership detection), then Phase 6 wired two new TUI flows on top.

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

### Milestone v1.1 — Shipped 2026-05-21

- [x] **Phase 5: Hook Engine** - Pure-Rust module owning the commit-msg hook file format: parse, serialize, ownership detection, idempotent install/extend/remove (completed 2026-05-21)
- [x] **Phase 6: Hook TUI Integration** - Two new main-menu flows (Add, Manage) wired to the hook engine, with success screens (completed 2026-05-21)

> Phase details for v1.0 (Phases 1–4) and v1.1 (Phases 5–6) are archived in `.planning/milestones/`.

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
| 6. Hook TUI Integration | 6/6 | Complete   | 2026-05-21 |
