---
quick_id: 260524-cbk
description: Fix cmd+backspace and option+backspace not working in TUI text fields
date: 2026-05-24
---

# Quick Task 260524-cbk: Fix cmd+backspace / option+backspace in TUI text fields

## Problem

Commit `5937b8f` added `backspace_edit()` which reads `KeyModifiers::SUPER` (Cmd)
and `KeyModifiers::ALT` (Option) to clear a line / delete a word. Neither worked
when tested on the second screen (rename form).

## Root cause

Two issues:
1. `ratatui::init()` never enabled the keyboard enhancement (Kitty CSI-u)
   protocol, so terminals deliver a plain `Backspace` with `KeyModifiers::NONE` —
   `backspace_edit` always fell through to `s.pop()`.
2. Cmd is reserved by most macOS terminals (Terminal.app, Warp) and is never
   forwarded to the program even with the protocol enabled, so Cmd+Backspace is
   not reliably deliverable.

## Approach (user-selected)

Add the universal readline bindings **and** enable the protocol:

1. `src/tui/event.rs` — at the top of `handle_key`, translate Ctrl+U → clear line
   and Ctrl+W → delete word by mapping them to modified `Backspace` events that
   the existing per-screen `backspace_edit` path already handles. Control chars
   arrive in every terminal, so this works regardless of protocol support.
2. `src/main.rs` — push `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`
   after `ratatui::init()` (gated on `supports_keyboard_enhancement()`), pop
   before `ratatui::restore()`. Makes Cmd/Option modifiers work where supported.

## Verify

- `cargo test --lib` passes (incl. new Ctrl+U / Ctrl+W tests on rename form)
- `cargo clippy` clean
