---
quick_id: 260520-tty
status: complete
date: 2026-05-20
commit: 7174552
---

## Summary

Fixed "Terminal I/O error: Failed to initialize input reader" / "Device not configured"
panic that occurred when the binary was run via `curl ... | sh` on macOS.

**Root cause:** When piped through shell, stdin is the pipe — not a TTY. `ratatui::init()`
calls crossterm which tries to configure raw mode on stdin and panics when it's not a device.

**Fix:**
- Added `AppError::NotATerminal` variant in `src/error.rs` with a clear user message
- Added TTY guard in `src/main.rs` using `std::io::IsTerminal` (stable since Rust 1.70,
  MSRV is 1.74) — placed after preflight checks so those errors still surface correctly,
  but before `ratatui::init()` to prevent the panic
- Added `test_binary_exits_cleanly_when_stdin_is_not_a_tty` integration test that
  simulates `curl|sh` by running binary with `Stdio::null()` as stdin

**Before:** panic with "Device not configured" / "Failed to initialize input reader"
**After:** clean exit with "error: Not an interactive terminal.\ngit-author-reformer is a TUI application — run it directly, not inside a pipe."

Files changed: `src/error.rs`, `src/main.rs`, `tests/main_integration_test.rs`
