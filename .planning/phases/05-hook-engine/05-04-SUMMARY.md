---
phase: 05-hook-engine
plan: "04"
subsystem: hook
tags: [hook-engine, atomic-write, public-api, tdd]
dependency_graph:
  requires: [05-01, 05-02, 05-03]
  provides: [hook-engine-public-api, atomic-writer]
  affects: [src/hook/mod.rs, src/hook/write.rs, src/hook/render.rs]
tech_stack:
  added: []
  patterns: [atomic-tmp-rename, permissions-before-rename, state-machine-dispatch]
key_files:
  created: [src/hook/write.rs]
  modified: [src/hook/mod.rs, src/hook/render.rs]
decisions:
  - "Captured original email list length before filter to detect NotFound in remove_strip — avoids needing to borrow after move"
  - "Marker constants consolidated: render.rs imports BEGIN_MARKER/END_MARKER from parse.rs (single source of truth)"
  - "email validation reuses AppError::Io(InvalidInput) per Pitfall §6 — no new error variant"
metrics:
  duration: "~25 minutes"
  completed: "2026-05-21"
  tasks_completed: 2
  files_changed: 3
---

# Phase 05 Plan 04: Hook Engine Integration Summary

**One-liner:** Atomic writer + full public hook API (`install_strip`, `remove_strip`, `read_strip_list`) wiring parser, renderer, and writer into a working hook engine.

## What Was Built

### Task 1: Atomic writer (src/hook/write.rs)

Implemented `atomic_write_executable` and `delete_hook` per RESEARCH §Pattern 1.

Key correctness invariant: `set_permissions` on the tmp file is called at source line 32, `fs::rename` at line 33 — permissions-before-rename ordering enforced (Pitfall §3).

The `#[cfg(unix)]` gate makes `set_mode(0o755)` a no-op on non-Unix targets (Windows is not a v1 target but compiles cleanly).

Seven unit tests verified via TDD (RED commit `test(05-04)`, GREEN commit `feat(05-04)`):
- `atomic_write_executable_creates_file_with_contents`
- `atomic_write_executable_overwrites_existing_file`
- `atomic_write_executable_sets_mode_0755_on_unix` (`#[cfg(unix)]`)
- `atomic_write_executable_cleans_up_tmp_file`
- `atomic_write_executable_emits_lf_line_endings`
- `delete_hook_removes_file`
- `delete_hook_returns_err_on_missing_file`

### Task 2: Public hook-engine API (src/hook/mod.rs)

Replaced three `unimplemented!` stubs with working state-machine implementations:

- `read_strip_list`: path → exists? → read → detect_markers → Absent / NotToolManaged / Managed
- `install_strip`: validate → lowercase → classify → duplicate? → push → atomic_write
- `remove_strip`: lowercase → classify → filter → NotFound / delete_hook / atomic_write

HOOK-12 compliance: zero calls to `check_stash`, `check_worktrees`, or `preflight` anywhere in `src/hook/`.

### Marker constant consolidation (render.rs)

The two `BEGIN_MARKER` / `END_MARKER` string literals that were duplicated in `render.rs` (prod scope + test mod) are now imported from `parse.rs` via `use crate::hook::parse::{BEGIN_MARKER, END_MARKER}`. Single source of truth; the TODO comment is resolved.

## Deviations from Plan

None — plan executed exactly as written. The `emails_len_before_filter` helper I drafted initially was discarded before commit in favor of capturing `original_len = emails.len()` before consuming the vector (cleaner, no helper needed). This is a simplification, not a deviation.

## Commits

| Hash | Message |
|------|---------|
| 9054514 | `test(05-04): add failing tests for atomic writer and delete_hook` |
| 6c9854c | `feat(05-04): implement atomic_write_executable and delete_hook` |
| 2ccba2d | `feat(05-04): implement public hook-engine API and consolidate marker constants` |

## Verification

- `cargo test --lib hook::write` — 7 passed, 0 failed
- `cargo test` (full suite) — 120 passed across 8 suites
- `cargo clippy --lib -- -D warnings` — no issues
- All `must_haves.truths` satisfied:
  - install_strip on absent hook creates file ✓
  - install_strip on tool-managed hook appends email ✓
  - install_strip with duplicate returns AlreadyStripped without writing ✓
  - install_strip on foreign hook returns Err(HookExists) ✓
  - remove_strip on non-last entry rewrites file ✓
  - remove_strip on last entry calls delete_hook ✓
  - read_strip_list returns Absent / Managed / NotToolManaged ✓
  - Permissions set before rename (line 32 < line 33) ✓
  - No preflight calls in hook engine ✓

## Self-Check: PASSED

- `src/hook/write.rs` exists: FOUND
- `src/hook/mod.rs` exists: FOUND
- Marker constants consolidated in `src/hook/render.rs`: FOUND (imports from parse.rs)
- 120 tests pass: VERIFIED
