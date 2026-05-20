---
phase: 03-tui-integration
reviewed: 2026-05-20T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - src/git/scan.rs
  - src/tui/app.rs
  - src/tui/event.rs
  - src/tui/render.rs
  - src/tui/mod.rs
  - src/main.rs
  - src/error.rs
  - Cargo.toml
  - tests/scan_test.rs
findings:
  critical: 0
  warning: 4
  info: 2
  total: 6
status: clean
fixes_applied: 5
fixes_skipped: 0
---

# Phase 03: TUI + Integration — Code Review Report

**Reviewed:** 2026-05-20  
**Depth:** standard  
**Files Reviewed:** 9  
**Status:** clean (all findings fixed)

## Summary

Nine files reviewed covering the scan layer (`scan.rs`), full TUI stack (`app.rs`, `event.rs`, `render.rs`, `mod.rs`), entry point (`main.rs`), error type (`error.rs`), manifest (`Cargo.toml`), and integration tests (`tests/scan_test.rs`).

The cascade logic in `scan.rs` faithfully replicates `rewrite.rs` for UTF-8 commits. The SIGTERM flag is correctly registered before `ratatui::init()`. Scan is invoked on the Preview transition only, never in the render path. Nucleo filter usage (`reparse` + `tick(10)`) is consistent. The Success and Error screens both show `rewritten` count and `remote_name` from `RewritePreview`.

Four warnings were found: a scan-count vs rewrite-count divergence for non-UTF-8 commit messages; a latent MainMenu navigation bug whose tests pass by numerical coincidence; a broken "press any key" exit on Success/Err; and an absent force-push warning at the confirmation screen (explicit brief requirement).

## Warnings

### WR-01: `scan_drop` silently ignores non-UTF-8 commit messages; `drop_coauthor` fails on them — cascade-count invariant broken

**File:** `src/git/scan.rs:79`

**Issue:** `scan_drop` reads the commit message with `commit.message_raw().unwrap_or("")`. When the message is not valid UTF-8, `unwrap_or("")` returns an empty string, so `message_would_change` is false; the commit is still added to `would_remap` if a parent was remapped (`any_parent_remapped` path). In `drop_coauthor` (rewrite.rs:233-235), the same commit fails with `NonUtf8Message` before any refs are touched.

Concrete scenario: commit A has a matching `Co-authored-by` trailer (UTF-8). Commit B descends from A with a non-UTF-8 message. `scan_drop` adds both A and B to `would_remap`. Preview shows "2 commits". User presses Y. `drop_coauthor` fails at B with `NonUtf8Message(B)` — no refs written — but the user was told 2 commits would be rewritten, not that the operation would abort. No data is lost, but the scan-count-equals-rewrite-count invariant the brief (Key check #8) explicitly requires is broken.

`scan_rename` is not affected — it reads only `commit.author()`, never the message body.

**Fix:** Apply the same handling as `drop_coauthor` — match on the result and include the commit in `would_remap` when the message is non-UTF-8 and a parent was remapped:
```rust
// scan.rs — scan_drop revwalk loop
let raw_msg = match commit.message_raw() {
    Ok(m) => m,
    Err(_) => {
        // Non-UTF-8: drop_coauthor will also fail here.
        // Cascade applies identically; message_would_change is impossible.
        let any_parent_remapped = (0..commit.parent_count())
            .any(|i| would_remap.contains(&commit.parent_id(i).unwrap()));
        if any_parent_remapped {
            would_remap.insert(old_oid);
        }
        continue;
    }
};
let message_would_change = message_has_matching_coauthor(raw_msg, target_email);
```

---

### WR-02: MainMenu `Up`/`k` arm is a copy-paste of `Down`/`j` — wrong logic masked by numerical coincidence

**File:** `src/tui/event.rs:8`

**Issue:** Both navigation arms execute the identical expression `(*selected + 1) % 2`:
```rust
KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1) % 2,
KeyCode::Up   | KeyCode::Char('k') => *selected = (*selected + 1) % 2,  // copy-paste bug
```
Up and Down are the same operation. The three test cases that cover this (`test_main_menu_down_increments_selected_mod_2`, `test_main_menu_up_decrements_with_wrap`, `test_main_menu_j_k_same_as_down_up`) all pass because, with exactly 2 items, `(x + 1) % 2` and decrement-with-wrap (`(x + 2 - 1) % 2`) produce the same cycle `0→1→0`. The tests never assert that Up and Down move in *opposite* directions — they only assert wrapping.

Compare to `AuthorList` and `CoAuthorList` which correctly implement Up as `(*selected + matched.len() - 1) % matched.len()` (lines 65, 178). If the menu ever grows to 3 items, Up navigation breaks without any code change.

**Fix:** Use the generalised decrement formula with the menu length as N:
```rust
const MENU_LEN: usize = 2; // or MenuChoice::all().len()
KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1) % MENU_LEN,
KeyCode::Up   | KeyCode::Char('k') => *selected = (*selected + MENU_LEN - 1) % MENU_LEN,
```
Also add a test that asserts Down from 0 reaches 1 while Up from 0 reaches `MENU_LEN - 1`, verifying they move in opposite directions.

---

### WR-03: Success/Err "press any key" promise is broken — non-char keys are silently swallowed

**File:** `src/tui/event.rs:206-211`

**Issue:**
```rust
Screen::Success { .. } | Screen::Err(_) => match key {
    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char(_) => {
        app.should_exit = true;
    }
    _ => {}
},
```
The `_` arm catches arrow keys, Tab, F-keys, Backspace, and all other non-char key codes and does nothing. The render strings at render.rs:299 and render.rs:312 both say "Press any key to exit". Pressing Down or Backspace after a rewrite leaves the screen frozen with no feedback.

The `Char('q')` alternative is also unreachable dead code — it is shadowed by the earlier `Char(_)` arm.

**Fix:** Remove the inner match entirely; all key presses should exit:
```rust
Screen::Success { .. } | Screen::Err(_) => {
    app.should_exit = true;
}
```

---

### WR-04: Preview screen omits force-push reminder before the user confirms

**File:** `src/tui/render.rs:253-283`

**Issue:** The brief lists "force-push reminder" as an explicit required element of the Preview screen (Key check #5). `render_preview` shows the affected count, GPG/SSH warning, annotated-tag warning, and notes warning, but never mentions that the rewrite will require `git push --force-with-lease` and will break anyone else with a local copy of the affected branches. The force-push reminder only appears on the Success screen (render.rs:299), after the rewrite is already done and irreversible.

**Fix:** Add the reminder to the body of `render_preview` before the "Proceed?" line:
```rust
lines.push(
    "\u{26a0} This rewrites history. Collaborators will need to re-clone or \
     force-reset. Push with: git push --force-with-lease --all <remote>"
        .to_string(),
);
```

## Info

### IN-01: `render_preview` hardcodes `refs/notes/commits` in the warning regardless of the actual detected ref

**File:** `src/tui/render.rs:271`

**Issue:** The warning text always says `refs/notes/commits exists` even when `check_has_notes_ref` matched `repo.note_default_ref()` — which may be a user-configured ref such as `refs/notes/review`. The user sees the wrong ref name and cannot verify or act on it.

**Fix:** Thread the actual matched ref name through `RewritePreview` as `notes_ref_name: Option<String>`, or change the message to a generic form: "A notes ref exists — notes reference old SHAs and will be orphaned by the rewrite."

---

### IN-02: SIGINT and SIGHUP not registered — only SIGTERM restores the terminal

**File:** `src/main.rs:13`

**Issue:** Only `SIGTERM` is registered with `term_flag`. An out-of-band `SIGINT` (`kill -INT`) or `SIGHUP` (terminal emulator closed while the process is alive) does not set the flag, so `ratatui::restore()` is not called on those paths. In interactive use, Ctrl-C arrives as a raw keystroke because the terminal is in raw mode, so the common case is unaffected. This matters only when a signal is sent externally while the TUI is active.

**Fix:**
```rust
signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term_flag))?;
signal_hook::flag::register(signal_hook::consts::SIGINT,  Arc::clone(&term_flag))?;
signal_hook::flag::register(signal_hook::consts::SIGHUP,  Arc::clone(&term_flag))?;
```

---

_Reviewed: 2026-05-20_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
