---
quick_id: 260524-cbk
status: complete
date: 2026-05-24
---

# Quick Task 260524-cbk: Summary

## What changed

- **`src/tui/event.rs`** — `handle_key` now translates Ctrl+U → clear-line and
  Ctrl+W → delete-word into the equivalent modified `Backspace` events, routing
  them through the existing `backspace_edit` path for all five text fields. Added
  two tests (`test_rename_form_ctrl_u_clears_focused_field`,
  `test_rename_form_ctrl_w_deletes_last_word`).
- **`src/main.rs`** — enables `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`
  after init (gated on terminal support), pops it before restore. Lets
  Cmd/Option+Backspace deliver modifiers in protocol-capable terminals.

## Why the original feature failed

`backspace_edit` was correct, but the enhancement protocol was never enabled, so
no modifiers ever reached it. Cmd is additionally reserved by macOS terminals
(Terminal.app, Warp) and cannot be relied on — hence the Ctrl+U/Ctrl+W fallback.

## Result

- 110 lib tests pass (2 new); `cargo clippy` clean.
- Ctrl+U / Ctrl+W work in every terminal including Warp.
- Cmd/Option+Backspace work in terminals supporting the Kitty protocol
  (kitty, WezTerm, Ghostty, iTerm2 with CSI-u). They remain non-functional in
  Warp / Terminal.app, which do not forward those modifiers.
