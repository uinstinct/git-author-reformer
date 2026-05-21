---
phase: 05-hook-engine
verified: 2026-05-21T07:30:00Z
status: human_needed
score: 7/7 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run `git commit` in a real repo after installing a hook with install_strip, with a Co-authored-by trailer for the configured email"
    expected: "The commit-msg hook fires automatically via git (mode bit 0755 recognized, .git/hooks/commit-msg location discovered, $1 argument = commit message file path) and the trailer is stripped from the recorded commit message"
    why_human: "cargo test exercises /bin/sh hook_path msg_file directly — it proves the awk script works but does NOT prove git itself invokes the hook. Git must discover hooks/commit-msg, honor its mode bit, and pass the correct temp-file path as $1. These are git-side behaviors that only a real git commit can exercise."
---

# Phase 05: Hook Engine Verification Report

**Phase Goal:** Pure-Rust module that owns the `commit-msg` hook file end-to-end — read, parse, serialize, install, extend, remove — with no TUI dependencies
**Verified:** 2026-05-21
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

All 7 must-haves from ROADMAP.md Success Criteria are verified. Scores are drawn from direct code and test inspection, not from SUMMARY.md claims.

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Fresh install on a repo with no existing `commit-msg` hook writes a POSIX `sh` script (shebang `#!/bin/sh`) at `.git/hooks/commit-msg` with mode 0755 and the email listed between marker comments | VERIFIED | `test_install_fresh_writes_file_with_markers_and_email` + `test_install_sets_mode_0755_on_unix` pass. `write.rs:31` sets `0o755`. `render.rs:30` emits `#!/bin/sh\n`. |
| 2 | Installing a strip entry on a tool-managed hook appends the email and rewrites; duplicate is reported as "already stripped" with file bytes unchanged | VERIFIED | `test_install_appends_to_existing_tool_managed_hook` + `test_install_duplicate_email_is_noop_file_bytes_identical`. `mod.rs:67-69` checks `eq_ignore_ascii_case` before writing. |
| 3 | Installing on a non-tool-managed hook returns a refuse-to-overwrite error naming the file — no file written | VERIFIED | `test_install_refuses_to_overwrite_non_tool_managed_hook` asserts `Err(AppError::HookExists(_))` and `pre_bytes == post_bytes`. `mod.rs:63` returns `Err(AppError::HookExists(p))` for `NotToolManaged`. `error.rs:27` has the error message. |
| 4 | Removing the last entry from a tool-managed hook deletes the `.git/hooks/commit-msg` file entirely | VERIFIED | `test_remove_last_entry_deletes_file` asserts `HookDeleted` and `!hook_path.exists()`. `mod.rs:102-104` calls `write::delete_hook`. |
| 5 | Executing the generated hook against a sample commit message strips `Co-authored-by:` lines case-insensitively for any email in the list, using the same matching semantics as the existing drop flow | VERIFIED | `test_shell_hook_strips_case_insensitive_matches` and `test_shell_hook_preserves_when_email_only_in_name_slot` both pass. Rust side: `reader.rs:84-107` uses `rfind('<')` / `rfind('>')` with `.trim()`. Awk side: `render.rs:49-57` uses backwards substr loop (same rfind semantics) with `gsub` trim. Twin-parity documented in render.rs comment lines 34-37. |
| 6 | Calling any hook engine operation does not invoke the SAFE-01/SAFE-02 preflight blockers — a repo with stash entries can still have hooks installed | VERIFIED | `grep check_stash src/hook/` — zero matches (exit 1 = no output). `test_install_does_not_trigger_preflight_with_stash_present` passes with `refs/stash` present. |
| 7 | Automated Rust tests cover every engine code path per HOOK-13 matrix — `cargo test` exercises each path | VERIFIED | `cargo test --test hook_test` → 12 passed. `cargo test` (full suite) → 132 passed. All 8 HOOK IDs in phase scope (04/05/06/07/08/10/12/13) have at least one test. |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/hook/mod.rs` | Public API: `install_strip`, `remove_strip`, `read_strip_list` + result enums | VERIFIED | All three functions present (lines 28, 50, 83). `HookState`, `AddResult`, `RemoveResult` declared (lines 10-25). |
| `src/hook/path.rs` | `commit_msg_hook_path(repo) -> PathBuf` | VERIFIED | `repo.path().join("hooks").join("commit-msg")` at line 4. |
| `src/hook/parse.rs` | Marker detection + strip-list extraction | VERIFIED | `detect_markers` (line 14), `extract_strip_list` (line 26). 9 unit tests in-file. |
| `src/hook/render.rs` | POSIX sh hook template with awk filter | VERIFIED | `render_hook` (line 17), `validate_email_for_embedding` (line 73). 14 unit tests in-file. |
| `src/hook/write.rs` | Atomic write with mode 0755 + delete | VERIFIED | `atomic_write_executable` (line 22), `delete_hook` (line 38). 7 unit tests in-file. |
| `src/error.rs` | `AppError::HookExists(PathBuf)` variant | VERIFIED | Line 27. `#[error]` message names the file and gives remediation instructions. |
| `src/lib.rs` | `pub mod hook;` wired into crate root | VERIFIED | Line 3. Alphabetical between `git` and `tui`. |
| `tests/hook_test.rs` | 12+ integration tests covering full HOOK-13 matrix | VERIFIED | 12 test functions, 331 lines. Imports `git_author_reformer::hook` public API. |
| `tests/common/mod.rs` | `run_hook_on_message` shell-execution helper | VERIFIED | Lines 71-82. Uses `Command::new("/bin/sh")`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/hook/mod.rs` | `src/hook/{parse,path,render,write}.rs` | `pub mod` declarations | WIRED | Lines 1-4: `pub mod parse; pub mod path; pub mod render; pub mod write;` |
| `src/hook/mod.rs` | `src/hook/path.rs::commit_msg_hook_path` | `path::commit_msg_hook_path(repo)` | WIRED | Called at mod.rs lines 29, 61, 87 |
| `src/hook/mod.rs` | `src/hook/parse.rs` | `parse::detect_markers` + `parse::extract_strip_list` | WIRED | mod.rs lines 34, 38 |
| `src/hook/mod.rs` | `src/hook/render.rs` | `render::render_hook` + `render::validate_email_for_embedding` | WIRED | mod.rs lines 54, 71 |
| `src/hook/mod.rs` | `src/hook/write.rs` | `write::atomic_write_executable` + `write::delete_hook` | WIRED | mod.rs lines 71, 103, 106 |
| `src/hook/render.rs` | `src/hook/parse.rs` | `use crate::hook::parse::{BEGIN_MARKER, END_MARKER}` | WIRED | render.rs line 9 |
| `src/lib.rs` | `src/hook/mod.rs` | `pub mod hook;` | WIRED | lib.rs line 3 |
| `tests/hook_test.rs` | `git_author_reformer::hook` | `use git_author_reformer::hook::{...}` | WIRED | hook_test.rs lines 3-5 |
| `tests/common/mod.rs::run_hook_on_message` | `/bin/sh` | `Command::new("/bin/sh")` | WIRED | common/mod.rs line 75 |
| `src/hook/` | `crate::git::preflight` | NOT called | VERIFIED ABSENT | `grep check_stash src/hook/` → exit 1 (no matches). HOOK-12 satisfied. |

### Data-Flow Trace (Level 4)

Hook module is not a TUI component — it performs file I/O, not dynamic rendering. Level 4 data-flow trace focuses on the install→read round-trip:

| Operation | Data Variable | Source | Produces Real Data | Status |
|-----------|---------------|--------|--------------------|--------|
| `install_strip` | `emails` Vec | `read_strip_list` → `parse::extract_strip_list` → disk read | Yes — reads actual hook file via `fs::read_to_string` (mod.rs:33) | FLOWING |
| `read_strip_list` | hook contents | `fs::read_to_string(&hook_path)` (mod.rs:33) | Yes — real filesystem read | FLOWING |
| `atomic_write_executable` | `contents` string | `render::render_hook(&emails)` (mod.rs:71) | Yes — deterministic from email list | FLOWING |

Round-trip verified by `test_read_strip_list_round_trips_through_render`: three emails installed, all three recovered in insertion order via `read_strip_list`.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full test suite (132 tests, 9 suites) | `cargo test` | 132 passed, 0 failed, 1.05s | PASS |
| Hook integration suite (12 tests) | `cargo test --test hook_test` | 12 passed, 0 failed, 0.12s | PASS |
| No TUI imports in hook module | `grep -rn "use.*tui\|use.*ratatui\|use.*crossterm" src/hook/` | exit 1 (no output) | PASS |
| No preflight calls in hook module | `grep -rn "check_stash\|check_worktrees\|preflight" src/hook/` | exit 1 (no output) | PASS |
| `commit_msg_hook_path` correct implementation | `grep "repo.path().join" src/hook/path.rs` | line 4 matches | PASS |
| `HookExists` variant present | `grep HookExists src/error.rs` | line 27 found | PASS |
| `pub mod hook` in crate root | `grep "pub mod hook" src/lib.rs` | line 3 found | PASS |

### TDD Audit (workflow.tdd_mode=true — advisory, not blocking)

| Plan | Type | RED Commit | GREEN Commit | TDD Discipline |
|------|------|-----------|--------------|----------------|
| 05-01 | `execute` (scaffold) | N/A — scaffold plan, not TDD | `2c2cf55` feat(05-01): scaffold hook module skeleton | N/A — scaffold; TDD not applicable |
| 05-02 | `tdd` | `b9768bc` test(05-02): add failing parser tests (RED — stubs) | `bd09e95` feat(05-02): implement hook-file parser | Clean RED→GREEN |
| 05-03 | `tdd` | `00cef53` test(05-03): add failing renderer tests | `0395a8a` feat(05-03): implement POSIX sh hook renderer | Clean RED→GREEN |
| 05-04 | `tdd` | `9054514` test(05-04): add failing tests for atomic writer | `6c9854c` + `2ccba2d` feat(05-04): implement write + public API | Clean RED→GREEN (two GREEN commits for two tasks) |
| 05-05 | `tdd` | `3b78e2c` test(05-05): add integration tests | No separate GREEN commit needed | **Advisory deviation: Plan 04 implemented the engine before Plan 05 wrote integration tests. `3b78e2c` was simultaneously RED (new file) and GREEN (all 12 passed on first run). Per objective: noted as advisory, not blocking.** |

**Process deviation (non-blocking):** `05-05-SUMMARY.md` frontmatter lists commit `185f41d` as the GREEN commit, but the actual git log shows this was split into `a554fca` (chore: cargo fmt + clippy) and the GREEN behavior is implicit in `3b78e2c` passing immediately. The SUMMARY's `key_files.modified` also includes `src/tui/event.rs` and `src/hook/*.rs` (from the bundled cleanup) — these are accurate to what was touched but create an inflated footprint relative to the test-only scope of Plan 05. Audit trail discrepancy is cosmetic; no code defect.

### Requirements Coverage

| Requirement | Phase | Description | Status | Evidence |
|-------------|-------|-------------|--------|----------|
| HOOK-04 | Phase 5 | Write/append `commit-msg` hook with email | SATISFIED | `install_strip` in `mod.rs:50-75`. Tests: `test_install_fresh_*`, `test_install_appends_*`. |
| HOOK-05 | Phase 5 | Duplicate add is no-op returning "already stripped" | SATISFIED | `mod.rs:67-69` case-insensitive check → `Ok(AddResult::AlreadyStripped)`. Test: `test_install_duplicate_email_is_noop_file_bytes_identical`. |
| HOOK-06 | Phase 5 | Refuse to overwrite non-tool-managed hook | SATISFIED | `mod.rs:63` returns `Err(AppError::HookExists(p))`. `error.rs:27` names the file. Test: `test_install_refuses_to_overwrite_non_tool_managed_hook`. |
| HOOK-07 | Phase 5 | POSIX sh shebang + mode 0755 | SATISFIED | `render.rs:30` `#!/bin/sh\n`. `write.rs:31` `set_mode(0o755)`. Tests: `test_install_sets_mode_0755_on_unix`, `test_generated_hook_has_posix_shebang_and_markers`. |
| HOOK-08 | Phase 5 | Same case-insensitive matching semantics as drop flow | SATISFIED | `reader.rs:84-107` rfind+trim ↔ `render.rs:49-57` backwards substr loop + gsub trim. Tests: `test_shell_hook_strips_case_insensitive_matches`, `test_shell_hook_preserves_when_email_only_in_name_slot`. |
| HOOK-10 | Phase 5 | Remove last entry deletes file entirely | SATISFIED | `mod.rs:102-104` calls `write::delete_hook`. Tests: `test_remove_single_entry_rewrites_file`, `test_remove_last_entry_deletes_file`. |
| HOOK-12 | Phase 5 | No preflight call for hook ops | SATISFIED | Zero matches for `check_stash`/`check_worktrees` in `src/hook/`. Test: `test_install_does_not_trigger_preflight_with_stash_present`. |
| HOOK-13 | Phase 5 | Full automated test coverage of every engine code path | SATISFIED | 12 integration tests in `tests/hook_test.rs`. 9 unit tests in `parse.rs`, 14 in `render.rs`, 7 in `write.rs`. `cargo test` → 132 passed. |

**Requirements NOT in Phase 5 scope (Phase 6):** HOOK-01, HOOK-02, HOOK-03, HOOK-09, HOOK-11, HOOK-14 — correctly deferred.

### Anti-Patterns Found

Scan of files modified by this phase (src/hook/*.rs, src/error.rs, src/lib.rs, tests/hook_test.rs, tests/common/mod.rs):

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None found | — | — | — | — |

No `TBD`, `FIXME`, `XXX`, placeholder patterns, or empty implementations found in phase files. Stub functions from Plan 01 were replaced with real implementations in Plans 02-04. `unimplemented!()` macros used during TDD RED phases are gone.

### Human Verification Required

**1. Git-invoked hook execution in a real repository**

**Test:** In an actual git repository (not a tempdir fake), run `git commit` with a commit message that includes a `Co-authored-by:` trailer for an email configured in the strip list. The hook must have been installed via `install_strip`.

**Expected:** Git discovers `.git/hooks/commit-msg`, respects its 0755 mode bit, invokes `/bin/sh .git/hooks/commit-msg <tempfile>` with the commit message file as `$1`, and the recorded commit object has the matching trailer stripped.

**Why human:** `cargo test` calls `/bin/sh hook_path msg_file` directly — bypassing git entirely. The integration tests prove the awk script is correct in isolation, but do NOT prove: (a) git's hook discovery mechanism finds the file at the correct path, (b) the 0755 mode bit is honored by the git binary in the user's environment, (c) the `$1` semantics (git passes a temp file, not stdin) match what the script expects. Only a real `git commit` invocation can confirm the end-to-end path. This is a shell-invocation/OS-behavior check, not a Rust code check.

---

## Gaps Summary

No gaps found. All 7 observable truths are VERIFIED against the actual codebase. The `human_needed` status reflects one item requiring real-world git invocation to confirm end-to-end hook discovery and execution, which cannot be verified via `cargo test` alone.

---

_Verified: 2026-05-21_
_Verifier: Claude (gsd-verifier)_
