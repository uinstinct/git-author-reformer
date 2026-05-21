---
phase: 06-hook-tui-integration
reviewed: 2026-05-21
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src/main.rs
  - src/tui/app.rs
  - src/tui/event.rs
  - src/tui/render.rs
findings:
  critical: 0
  warning: 0
  info: 2
  total: 2
status: clean
---

# Phase 06: Hook TUI Integration — Code Review Report

**Reviewed:** 2026-05-21
**Depth:** standard
**Files Reviewed:** 4
**Status:** clean (zero Critical, zero Warning)

## Summary

Four files reviewed against six focus areas. The implementation is structurally sound. HOOK-11 (engine-truth invariant), HOOK-12 (preflight bypass), MenuChoice navigation, HookState exhaustiveness, and error-path routing are all correctly implemented. Two informational observations are noted; neither rises to WARNING because neither causes incorrect behavior or data loss in any realistic scenario.

## Critical Issues

None.

## Warnings

None.

## Info

### IN-01: `render_hook_success` NotToolManaged fallback emits a self-referential message

**File:** `src/tui/render.rs:516–518`

The third arm of `render_hook_success` outputs `"Error: foreign hook (should not reach this screen)."` and discards the `PathBuf`. In the nominal path this arm is unreachable because the entry handlers in `event.rs` (lines 105–110 and 152–156) filter `NotToolManaged` to `Screen::Err` before `HookSuccess` is ever set. However, there is one TOCTOU window: after `install_strip` or `remove_strip` succeeds, the post-mutation `read_strip_list` call (lines 343–346 and 387–390) re-reads the hook file; if a foreign hook races in between those two operations, `read_strip_list` returns `HookState::NotToolManaged`, the `Ok(state)` arm sets `Screen::HookSuccess { state: NotToolManaged(...) }`, and `render_hook_success` emits the unhelpful self-referential string. The entry handlers produce a proper actionable message (`"Foreign hook at {path} — remove or rename it first."`); the render fallback is inconsistent with that.

**Fix:** Change the arm to format the path matching the entry-handler message, or `unreachable!("NotToolManaged must never reach HookSuccess")` to make the invariant explicit.

### IN-02: `HookSuccess` empty-state exits the app; `HookAlreadyStripped` returns to menu

**File:** `src/tui/event.rs:424–429`

When the user selects Manage on a repo with no hook, the app routes to `Screen::HookSuccess { state: Absent }` (line 136–139). Any key on `HookSuccess` sets `should_exit = true` (lines 424–426). By contrast, `HookAlreadyStripped` (also a no-op outcome) on any key returns to `Screen::MainMenu { selected: 2 }` (lines 427–429) rather than exiting. The inconsistency means a user who checks Manage out of curiosity cannot return to the menu without restarting the binary.

**Fix (if behavioral symmetry desired):** Match on `state` in the `HookSuccess` arm and return to `MainMenu { selected: 3 }` when `Absent`, exit only on real-success (`Managed`/`NotToolManaged`).

## Focus-Area Verdicts

| Area | Verdict | Evidence |
|------|---------|----------|
| HOOK-11 engine-truth invariant | PASS | `install_strip → read_strip_list` at line 343; `remove_strip Updated → read_strip_list` at line 387; `HookDeleted → HookRemoved` direct at line 392 (justified: hook file is gone) |
| HOOK-12 preflight bypass | PASS | `check_stash` appears only at lines 48 and 75 (Rename and Drop branches); `main.rs` has zero preflight calls |
| MenuChoice ↔ usize navigation | PASS | `% 4` on lines 41–42; `from_index` covers 0/1/2/_; `all()` returns `[Self; 4]`; 4 exhaustive Enter arms |
| HookState rendering coverage | PASS | `render_hook_success` matches all three variants; entry handlers cover all three with no silent wildcards |
| Error path handling | PASS | `NotToolManaged` → `Screen::Err`; `install_strip`/`remove_strip` errors → `Screen::Err`; `RemoveResult::NotFound` → `Screen::Err` defensive |
| Test quality | PASS | Tests assert data content (emails present/absent), not just variant discriminants; stash-bypass tests verify absence of preflight block |
| Karpathy adherence | PASS | `src/main.rs` diff removes exactly the two preflight lines; no adjacent refactoring; `items` field follows existing `CoAuthorList` convention |

---

_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
