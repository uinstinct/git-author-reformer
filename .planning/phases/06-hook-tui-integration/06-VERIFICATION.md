---
phase: 06-hook-tui-integration
verified: 2026-05-21T00:00:00Z
status: gaps_found
score: 5/6 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Picking 'Manage' displays a fuzzy-filterable list of configured strip emails; selecting an entry removes it via the hook engine and lands on a success screen showing the resulting strip-list state (or 'hook removed — no entries remain' when the last entry was removed)"
    status: failed
    reason: "SC4 requires a distinct 'hook removed — no entries remain' message when the last entry is removed. The implementation routes HookDeleted to HookSuccess { state: Absent }, which renders 'No hook installed — no emails configured.' — the same message shown when no hook was ever installed. The two distinct user situations (just removed last entry vs. never had a hook) are rendered identically. The ROADMAP SC4 parenthetical is a behavioral contract, not paraphrase."
    artifacts:
      - path: "src/tui/render.rs"
        issue: "render_hook_success Absent arm (line 500-501) renders 'No hook installed — no emails configured.' for both the never-had-a-hook case and the post-remove-last-entry case. No branch distinguishes them."
      - path: "src/tui/event.rs"
        issue: "HookManageList Enter, HookDeleted path (line ~398): routes to HookSuccess { state: HookState::Absent } directly, which reuses the Absent render arm. A dedicated 'hook removed' state or a disambiguating field on HookSuccess is needed."
    missing:
      - "Either: (a) add a new Screen variant (e.g. HookRemoved) that renders 'Hook removed — no entries remain.' and route HookDeleted there instead of HookSuccess; OR (b) add a boolean/enum field to HookSuccess (e.g. just_removed: bool) and branch on it in render_hook_success to emit the SC4-specified message when the hook was actively removed by the user."
      - "Add a state-machine test that asserts the post-remove-last-entry path produces a distinct screen (or distinct message content) from the never-installed path."
---

# Phase 6: Hook TUI Integration Verification Report

**Phase Goal:** Two new main-menu flows (Add, Manage) wired to the hook engine, with fuzzy-filterable selectors and success screens
**Verified:** 2026-05-21
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Launching the tool presents a four-option main menu — "Rename an author", "Drop a co-author", "Add co-author auto-strip hook", "Manage auto-strip hook" — and responds to keyboard navigation | VERIFIED | `src/tui/app.rs`: MenuChoice enum has 4 variants with correct label strings; `event.rs` lines 41-42: navigation uses `% 4` modulus; `test_main_menu_shows_four_options` and `test_menu_choice_all_has_four_items` pass |
| 2 | The "Manage auto-strip hook" option is always visible and selectable, even when no hook is installed; in that empty state it shows a clear "no entries configured" screen | VERIFIED | `event.rs` ManageHook branch (line 135): `Absent` variant routes to `Screen::HookSuccess { state: HookState::Absent }` without error; `render_hook_success` matches `Absent` -> "No hook installed — no emails configured."; `test_main_menu_routes_manage_hook_empty` passes |
| 3 | Picking "Add" displays the currently-configured strip list then a fuzzy-filterable co-author selector reusing the same enumeration as the existing drop flow; selecting an entry hands off to the hook engine and lands on a success screen showing the resulting strip-list state | VERIFIED | `event.rs` AddHook branch (line 100): calls `read_strip_list` then `enumerate_coauthors` (HOOK-03 reuse at line 117, same as Drop branch at 83); HookAddList Enter: calls `install_strip` then `read_strip_list` re-read -> `HookSuccess` (HOOK-11); `test_main_menu_routes_add_hook`, `test_add_hook_happy_path` pass |
| 4 | Picking "Manage" displays a fuzzy-filterable list of configured strip emails; selecting an entry removes it via the hook engine and lands on a success screen showing the resulting strip-list state (or "hook removed — no entries remain" when the last entry was removed) | FAILED | `event.rs` HookManageList Enter HookDeleted path routes to `HookSuccess { state: HookState::Absent }`, which reuses the never-installed render arm. `render_hook_success` Absent arm (line 500-501) renders `"No hook installed — no emails configured."` for both post-removal and never-installed. SC4 requires a distinct "hook removed — no entries remain" message. The two user situations are indistinguishable in the rendered output. |
| 5 | Neither Add nor Manage triggers the stash/worktree pre-flight blockers — both flows reach their selectors on a repo with stash entries | VERIFIED | `main.rs`: zero calls to `check_stash` or `check_worktrees` (preflight removed from startup per 06-01); `event.rs`: `check_stash` at lines 48 and 75 only (Rename and Drop branches); AddHook (line 100) and ManageHook (line 135) have no preflight calls; `test_add_hook_no_preflight_with_stash`, `test_manage_no_preflight_with_stash` pass using `make_test_app_with_stash()` helper |
| 6 | Automated TUI/state-machine tests cover every user path: main menu routes each of the four options, Add happy path → success screen, Add duplicate → already-stripped screen, Manage empty state, Manage remove single → updated list, Manage remove last → "hook removed" screen, and a regression test verifies Add/Manage on a repo with stash entries does NOT hit the SAFE-01/SAFE-02 preflight | VERIFIED | 152 tests pass; all listed transitions have dedicated tests; however, `test_manage_remove_last_entry` asserts routing to `HookSuccess { state: Absent }` — it does not assert a distinct "hook removed" screen because no such screen exists |

**Score:** 5/6 truths verified

### Deferred Items

None.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/tui/app.rs` | MenuChoice with 4 variants, 4 new Screen variants, strip fuzzy helpers | VERIFIED | MenuChoice: Rename/Drop/AddHook/ManageHook; Screen variants: HookAddList, HookManageList, HookSuccess, HookAlreadyStripped; helpers: build_strip_nucleo, apply_strip_filter |
| `src/tui/event.rs` | AddHook and ManageHook branches wired to hook engine; all 4 new Screen arms; stash-bypass confirmed; 49 tui::event tests | VERIFIED | 1489 lines; all branches implemented; 49 tests in tui::event module; 152 total tests pass |
| `src/tui/render.rs` | render_hook_add_list, render_hook_manage_list, render_hook_success, render_hook_already_stripped — all real implementations | PARTIAL | All 4 functions are substantive implementations; dispatch covers all 11 Screen variants; however render_hook_success Absent arm (line 500-501) does not distinguish post-removal from never-installed |
| `src/main.rs` | Preflight removed from startup (HOOK-12) | VERIFIED | Only call is `git::open_repo()`; no check_stash or check_worktrees calls at startup |
| `tests/main_integration_test.rs` | Updated integration test names to reflect new startup behavior | VERIFIED | Test name changed to `test_binary_reaches_tty_guard_when_stash_ref_exists`; documents that preflight no longer fires at startup |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| MainMenu Enter | AddHook branch | `MenuChoice::from_index(sel)` match | WIRED | event.rs line 100; dispatches on index 2 |
| MainMenu Enter | ManageHook branch | `MenuChoice::from_index(sel)` match | WIRED | event.rs line 135; dispatches on index 3 |
| AddHook branch | `enumerate_coauthors` | HOOK-03 reuse mandate | WIRED | event.rs line 117; same function as Drop branch at line 83 |
| AddHook branch | `read_strip_list` | strip list header population | WIRED | event.rs line 101 (entry read) |
| HookAddList Enter | `install_strip` | hook engine call | WIRED | event.rs line 338 |
| HookAddList Enter (Installed) | `read_strip_list` re-read | HOOK-11 engine-truth | WIRED | event.rs line 343 |
| HookManageList Enter | `remove_strip` | hook engine call | WIRED | event.rs line 387 |
| HookManageList Enter (Updated) | `read_strip_list` re-read | HOOK-11 engine-truth | WIRED | event.rs line 393 |
| HookManageList Enter (HookDeleted) | `HookSuccess { state: Absent }` | direct construct | PARTIAL | event.rs line ~398; hook file deleted path reuses Absent render arm; SC4 requires distinct "hook removed" message — not rendered |
| AddHook / ManageHook | zero preflight calls | HOOK-12 stash bypass | WIRED | grep shows check_stash only at lines 48 (Rename) and 75 (Drop) |
| render() dispatch | all 11 Screen variants | exhaustive match | WIRED | render.rs; no Screen variant left as todo!/unreachable! |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| render_hook_add_list | `current_strip`, `matched` | read_strip_list + enumerate_coauthors | Yes — live hook file read + git log traversal | FLOWING |
| render_hook_manage_list | `matched` | read_strip_list → build_strip_nucleo | Yes — live hook file read | FLOWING |
| render_hook_success (Updated path) | `state: HookState` | read_strip_list re-read post-mutation | Yes — re-read after install_strip/remove_strip (HOOK-11) | FLOWING |
| render_hook_success (HookDeleted path) | `state: HookState::Absent` | constructed directly | N/A — no data to flow; but renders wrong message for context | STATIC (renders Absent message regardless of user action) |
| render_hook_already_stripped | `email: String` | NLL-cloned from selected coauthor | Yes — from enumerate_coauthors result | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All 152 tests pass | `PATH=/tmp/ar-intercept:$PATH cargo test` | `test result: ok. 152 passed` | PASS |
| clippy clean | `cargo clippy -- -D warnings` | 0 errors | PASS |
| fmt clean | `cargo fmt --check` | no changes needed | PASS |
| enumerate_coauthors reuse (HOOK-03) | `grep -n "enumerate_coauthors" src/tui/event.rs` | Lines 83 (Drop) and 117 (AddHook) — 2 callsites | PASS |
| Preflight not called in AddHook/ManageHook | `grep -n "check_stash" src/tui/event.rs` | Lines 48 and 75 only (Rename, Drop branches) | PASS |
| read_strip_list re-read post-install (HOOK-11) | `grep -n "read_strip_list" src/tui/event.rs` | Lines 101, 318, 343, 387 — entry reads + post-mutation re-reads | PASS |
| % 4 modulus in navigation | `grep -n "% 4" src/tui/event.rs` | Lines 41-42 (Down and Up wrap at 4) | PASS |
| render_hook_success Absent text (post-removal) | `grep -n "No hook installed\|hook removed" src/tui/render.rs` | Line 501: "No hook installed — no emails configured." only — no "hook removed" branch | FAIL — SC4 violated |
| No debt markers in modified files | `grep -rn "TBD\|FIXME\|XXX" src/tui/event.rs src/tui/render.rs src/tui/app.rs src/main.rs` | 0 matches | PASS |

### Probe Execution

| Probe | Command | Result | Status |
|-------|---------|--------|--------|
| cargo test (full suite) | `PATH=/tmp/ar-intercept:$PATH cargo test` | 152 passed, 0 failed | PASS |

No `scripts/*/tests/probe-*.sh` probes declared or found for this phase.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| HOOK-01 | 06-02 | Four-option main menu visible and navigable | SATISFIED | MenuChoice enum 4 variants; % 4 navigation; test_main_menu_shows_four_options |
| HOOK-02 | 06-02, 06-04 | Manage option always visible; empty state clearly shown | SATISFIED | ManageHook Absent path -> HookSuccess(Absent); render_hook_success Absent arm |
| HOOK-03 | 06-03 | AddHook reuses enumerate_coauthors from Phase 1, not a parallel impl | SATISFIED | event.rs line 117 calls reader::enumerate_coauthors; same function as Drop branch |
| HOOK-09 | 06-04 | Manage flow shows fuzzy-filterable strip list | SATISFIED | HookManageList with build_strip_nucleo + apply_strip_filter wiring; render_hook_manage_list three-zone layout |
| HOOK-11 | 06-03, 06-04 | Success screens populated from engine re-read, not cached TUI state | SATISFIED | install_strip -> read_strip_list at line 343; remove_strip Updated -> read_strip_list at line 393; HookDeleted exception routed to Absent (hook is gone, re-read would return Absent anyway) |
| HOOK-14 | 06-05 | Add and Manage bypass stash/worktree preflight | SATISFIED | check_stash/check_worktrees absent from AddHook and ManageHook branches; test_add_hook_no_preflight_with_stash and test_manage_no_preflight_with_stash pass |

**Note:** All HOOK-01 through HOOK-14 requirements in `.planning/REQUIREMENTS.md` remain marked "Pending" — the executor did not update the traceability table. Informational gap only.

### TDD Cadence Audit (workflow.tdd_mode = true)

| Plan | RED Commit | GREEN Commit | Gate | Notes |
|------|-----------|-------------|------|-------|
| 06-01 | 76192f1 `test(06-01): add failing preflight-in-branch tests` | 4f3af85 `feat(06-01): move preflight into Rename/Drop branches` | PASS | Both commits verified via `git show` |
| 06-02 | N/A | N/A | SKIP | Plan type: execute (not TDD); scaffold/wiring only — stubs documented, no RED gate required |
| 06-03 | 285b954 `test(06-03): add failing Add flow tests` | 185cbfe `feat(06-03): implement Add flow (HookAddList, HookSuccess, HookAlreadyStripped)` | PASS | Both commits verified |
| 06-04 | 238bc26 `test(06-04): add failing Manage flow tests` | 19583d0 `feat(06-04): implement Manage flow (HookManageList, remove_strip wiring)` | PASS | Both commits verified |
| 06-05 | N/A (tests embedded) | N/A | SKIP | Plan type: execute/auto; stash-bypass tests added + clippy/fmt gate; no new production features requiring TDD RED gate |

### Karpathy Surgical-Change Audit (Rule 3)

| Plan | Declared files_modified | Actually Modified | Verdict | Notes |
|------|------------------------|------------------|---------|-------|
| 06-01 | src/main.rs, src/tui/event.rs | src/main.rs, src/tui/event.rs, tests/main_integration_test.rs | INFO | Integration test names changed to reflect new startup behavior; documented in 06-01-SUMMARY.md deviations with sound rationale (test assertions had to match the behavior this plan explicitly removed). Not a blocker. |
| 06-02 | src/tui/app.rs, src/tui/event.rs, src/tui/render.rs | src/tui/app.rs, src/tui/event.rs, src/tui/render.rs | CLEAN | Exact match |
| 06-03 | src/tui/event.rs, src/tui/render.rs | src/tui/event.rs, src/tui/render.rs | CLEAN | Exact match |
| 06-04 | src/tui/event.rs, src/tui/render.rs | src/tui/event.rs, src/tui/render.rs | CLEAN | Exact match |
| 06-05 | src/tui/event.rs | src/tui/event.rs | CLEAN | Exact match |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src/tui/render.rs | 500-501 | render_hook_success Absent arm renders identical text for post-remove-last-entry and never-installed cases | BLOCKER | SC4 specifies a distinct "hook removed — no entries remain" message for the post-removal path; current code uses "No hook installed — no emails configured." for both — user cannot distinguish why they are seeing this screen |
| src/tui/render.rs | (all hook render fns) | render_hook_* functions exist but are never called by any automated test | INFO | State-machine tests verify all transitions; visual layout accuracy requires human UAT |

No `TBD`, `FIXME`, or `XXX` markers found in any file modified by this phase.

### Gaps Summary

**1 blocker gap.** SC4 requires that removing the last strip entry shows a contextually distinct message — specifically "hook removed — no entries remain" (ROADMAP line 65). The implementation routes HookDeleted to `HookSuccess { state: HookState::Absent }` and reuses the Absent render arm, which produces "No hook installed — no emails configured." A user who just removed their last entry sees the same screen as a user who never installed a hook. The ROADMAP parenthetical "(or 'hook removed — no entries remain' when the last entry was removed)" is a behavioral contract.

**Fix options:**
- Add a new Screen variant (e.g. `HookRemoved`) that renders "Hook removed — no entries remain." and route the HookDeleted path there.
- Or add a field to HookSuccess (e.g. `context: SuccessContext`) and branch in render_hook_success on whether the hook was just removed vs. was already absent.

Either fix requires a new test that asserts the post-remove-last-entry path produces distinct output from the never-installed path.

---

_Verified: 2026-05-21_
_Verifier: Claude (gsd-verifier)_
