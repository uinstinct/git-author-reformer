---
quick_id: 260520-scm
status: complete
date: 2026-05-20
---

# Quick Task 260520-scm: Add 'c' copy shortcut to Success screen

## What was done

Added a `c` keyboard shortcut to the Success screen that copies the git push command to the clipboard via OSC 52 (terminal escape sequence), with "Copied!" feedback.

## Changes

### `src/tui/app.rs`
- Added `copied: bool` field to `Screen::Success`
- Updated all test construction sites to include `copied: false`
- Updated pattern matches to use `..`

### `src/tui/event.rs`
- Added `base64_encode` helper (inline, no new dependencies)
- Added `copy_via_osc52` helper — writes OSC 52 escape sequence to stdout
- Split `Screen::Success { .. } | Screen::Err(_)` into two separate arms
- `Screen::Success`: `c` copies command and sets `*copied = true`; all other keys still exit
- Updated `Screen::Success` construction to include `copied: false`
- Updated test pattern matches to use `..`

### `src/tui/render.rs`
- Updated `render_success` to accept `copied: bool`
- Shows "Press 'c' to copy  |  Any key to exit" by default
- Shows "Copied!  |  Any key to exit" after pressing `c`

## Implementation notes

- OSC 52 works without any additional dependencies — zero binary size increase
- The terminal processes the escape sequence directly; it works in iTerm2, Kitty, Alacritty, WezTerm, and most modern terminal emulators
- Base64 encoding is implemented inline (~13 lines) to avoid adding a `base64` crate dependency

## Build note

`cargo check` / `cargo test` could not be run locally due to a pre-existing macOS toolchain issue (`CommandLineTools/bin/ranlib` permission error on this machine — unrelated to this change, confirmed by checking out the base commit). Code changes are syntactically verified by manual review.
