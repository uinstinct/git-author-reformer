---
quick_id: 260520-scm
slug: when-the-following-prompt-occurs-run-the
description: "when the following prompt occurs \"run the following to update the remote ...\", add a `c` button shortcut to copy the command"
date: 2026-05-20
must_haves:
  truths:
    - "Pressing 'c' on Screen::Success copies the git push command to the clipboard"
    - "The success screen shows a 'c to copy' hint and 'Copied!' feedback after pressing c"
    - "All other keys on Screen::Success still exit the app"
  artifacts:
    - src/tui/event.rs
    - src/tui/render.rs
    - src/tui/app.rs
---

# Quick Task 260520-scm: Add 'c' copy shortcut to Success screen

## Context

The Success screen (shown after a rewrite) displays:
  "Run the following to update the remote:\n\n  git push --force-with-lease --all {remote}"

Currently any key exits. Task: pressing 'c' copies the push command via OSC 52, shows "Copied!" feedback; all other keys still exit.

No new dependencies — OSC 52 clipboard + inline base64 encoder.

## Tasks

### Task 1: Add `copied` flag to Screen::Success and implement copy logic

**Files:** src/tui/app.rs, src/tui/event.rs

**Action:**
1. Add `copied: bool` field to `Screen::Success` in `app.rs`
2. Update all construction sites: add `copied: false`
3. Update all pattern matches: add `copied: _` or `..`
4. In `event.rs`, split the `Screen::Success { .. } | Screen::Err` arm:
   - `Screen::Success`: handle `KeyCode::Char('c')` → copy via OSC 52, set `*copied = true`; other keys → `app.should_exit = true`
   - `Screen::Err`: `app.should_exit = true` (unchanged)
5. Add `base64_encode` and `copy_via_osc52` helpers in `event.rs`

**Verify:** `cargo test` passes

### Task 2: Update render to show copy hint

**Files:** src/tui/render.rs

**Action:**
1. Update `Screen::Success { rewritten, remote_name }` destructure to include `copied`
2. Update `render_success` signature to accept `copied: bool`
3. Update text: replace "Press any key to exit." with:
   - When not copied: "Press 'c' to copy  |  Any key to exit"
   - When copied: "Copied!  |  Any key to exit"

**Verify:** `cargo test` passes, `cargo build` succeeds
