---
phase: 05-hook-engine
plan: "05"
subsystem: hook
tags: [hook-engine, integration-tests, tdd, shell-hook, awk-twin-parser]
dependency_graph:
  requires: [05-04]
  provides: [hook-engine-integration-tests, shell-execution-helper]
  affects: [tests/hook_test.rs, tests/common/mod.rs]
tech_stack:
  added: []
  patterns: [shell-out-via-std-process-command, matches-macro-assertion, struct-initializer-update-syntax]
key_files:
  created: [tests/hook_test.rs]
  modified: [tests/common/mod.rs, src/tui/event.rs, src/hook/parse.rs, src/hook/render.rs, src/hook/write.rs]
decisions:
  - "Used matches!() + destructuring for enum assertions (no Debug/PartialEq derives needed on hook enums) — Karpathy Rule 3: no source-file changes for test ergonomics alone"
  - "Dropped render_hook leg of test #12 — pub(crate) visibility not reachable from integration tests; round-trip is fully proven by install->read_strip_list cycle"
  - "create_stash_ref private helper in hook_test.rs reuses repo.reference pattern from preflight_test.rs — not polluting tests/common/mod.rs with hook-specific setup"
  - "Fixed pre-existing clippy::field_reassign_with_default in src/tui/event.rs as Rule 3 blocking issue (required for --lib --tests clippy gate to pass)"
metrics:
  duration: "~30 minutes"
  completed: "2026-05-21"
  tasks_completed: 2
  files_changed: 6
---

# Phase 05 Plan 05: Hook Engine Integration Test Suite Summary

**One-liner:** 12-test TDD integration suite for the hook engine (HOOK-04/05/06/07/08/10/12/13) with shell-out helper proving awk twin-parser parity against the Rust drop flow.

## What Was Built

### Task 1: RED — Integration tests + shell-out helper (test(05-05))

Extended `tests/common/mod.rs` with `run_hook_on_message(hook_path, msg) -> String` — writes the message to a tempfile, invokes `/bin/sh hook_path MSG_FILE` via `std::process::Command`, asserts exit 0, returns the filtered result.

Created `tests/hook_test.rs` with 12 `#[test]` functions covering every HOOK requirement:

| Test | HOOK ID | What It Proves |
|------|---------|----------------|
| `test_install_fresh_writes_file_with_markers_and_email` | HOOK-04 | Fresh install creates file with both markers and email |
| `test_install_appends_to_existing_tool_managed_hook` | HOOK-04 | Second install appends; count=2 |
| `test_install_duplicate_email_is_noop_file_bytes_identical` | HOOK-05 | Duplicate (same + mixed-case) returns AlreadyStripped; bytes unchanged |
| `test_install_refuses_to_overwrite_non_tool_managed_hook` | HOOK-06 | Foreign hook returns Err(HookExists); file unchanged |
| `test_install_sets_mode_0755_on_unix` | HOOK-07 | File mode is 0755 after install (#[cfg(unix)]) |
| `test_generated_hook_has_posix_shebang_and_markers` | HOOK-07 | Starts with #!/bin/sh\n, both markers, no CRLF |
| `test_shell_hook_strips_case_insensitive_matches` | HOOK-08 | awk strips all case variants of target email; preserves others |
| `test_shell_hook_preserves_when_email_only_in_name_slot` | HOOK-08 | **Load-bearing**: awk structural extraction preserves lines where target is in name slot only |
| `test_remove_single_entry_rewrites_file` | HOOK-10 | Remove non-last entry; Updated{remaining:1} |
| `test_remove_last_entry_deletes_file` | HOOK-10 | Remove last entry; HookDeleted; file absent |
| `test_install_does_not_trigger_preflight_with_stash_present` | HOOK-12 | Engine bypasses check_stash — succeeds with refs/stash present |
| `test_read_strip_list_round_trips_through_render` | HOOK-13 | Three-email write→read round-trip preserves insertion order |

All 12 passed on first run — acceptable per TDD reference doc (Plan 04 implemented the engine; Plan 05 adds integration tests after the fact).

### Task 2: GREEN — Final phase verification (feat(05-05))

Full `cargo test`: 132 passed across 9 suites (including 12 hook_test + all pre-existing suites).

Fixed two pre-existing blocking issues (Rule 3) required for the clippy gate:
- `clippy::field_reassign_with_default` in `src/tui/event.rs` lines 459-461 and 490-492 — rewritten as struct initializer update syntax (`RenameDraft { field: val, ..RenameDraft::default() }`)
- Pre-existing `cargo fmt` drift in `src/hook/parse.rs`, `src/hook/render.rs`, `src/hook/write.rs`, `src/tui/*.rs` — applied `cargo fmt` in one pass

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Pre-existing clippy::field_reassign_with_default in src/tui/event.rs**
- **Found during:** Task 2 clippy gate
- **Issue:** Two test functions built `RenameDraft` with `default()` then reassigned fields — clippy -D warnings rejects this
- **Fix:** Rewrote to use `RenameDraft { field: val, ..RenameDraft::default() }` struct initializer syntax
- **Files modified:** `src/tui/event.rs`
- **Commit:** 185f41d

**2. [Rule 3 - Blocking] Pre-existing cargo fmt drift in src/hook/*.rs and src/tui/*.rs**
- **Found during:** Task 2 fmt gate
- **Issue:** Multiple files had formatting not matching rustfmt style (long assert! calls, format! trailing commas, match arm bracing)
- **Fix:** `cargo fmt` applied in one pass
- **Files modified:** `src/hook/parse.rs`, `src/hook/render.rs`, `src/hook/write.rs`, `src/main.rs`, `src/tui/app.rs`, `src/tui/event.rs`, `src/tui/mod.rs`, `src/tui/render.rs`
- **Commit:** 185f41d

### Design Simplifications (not deviations)

- **render_hook leg of test #12 dropped:** The plan explicitly pre-approved this: "if not accessible from integration tests, drop this leg." `pub(crate)` functions are unreachable from `tests/`. The round-trip is fully proven by the install→read_strip_list disk cycle.
- **No Debug/PartialEq derives added to hook enums:** Used `matches!()` + destructuring throughout — zero source file changes for test ergonomics. Karpathy Rule 3 compliant.
- **Karpathy Rule 9 (stale branches):** This agent runs inside `worktree-agent-ab5075b8e8d58f678` — worktree teardown is orchestrator-owned after the wave. No agent-created branches to clean up.

## Commits

| Hash | Message |
|------|---------|
| 3b78e2c | `test(05-05): add integration tests for hook engine (HOOK-04/05/06/07/08/10/12/13)` |
| 185f41d | `feat(05-05): green hook-engine integration tests (HOOK-04/05/06/07/08/10/12/13)` |

## Verification

- `cargo test --test hook_test` — 12 passed, 0 failed
- `cargo test` (full suite) — 132 passed across 9 suites
- `cargo clippy --lib --tests -- -D warnings` — no issues
- `cargo fmt --check` — clean
- All HOOK requirements in frontmatter have at least one test: HOOK-04(2), HOOK-05(1), HOOK-06(1), HOOK-07(2), HOOK-08(2), HOOK-10(2), HOOK-12(1), HOOK-13(1)
- Pitfall §1 counterexample test (`test_shell_hook_preserves_when_email_only_in_name_slot`) passes — awk structural extraction mirrors `src/git/reader.rs:84-107`

## Known Stubs

None — all tests exercise real engine behavior; no placeholder or mock data.

## Threat Flags

None — integration tests are test-only code; no new production API surface introduced.

## Self-Check: PASSED

- `tests/hook_test.rs` exists: FOUND
- `tests/common/mod.rs` contains `run_hook_on_message`: FOUND
- Commit 3b78e2c exists: VERIFIED
- Commit 185f41d exists: VERIFIED
- 132 tests pass: VERIFIED
