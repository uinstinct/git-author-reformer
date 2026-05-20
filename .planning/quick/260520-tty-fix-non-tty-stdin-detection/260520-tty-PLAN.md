---
quick_id: 260520-tty
slug: fix-non-tty-stdin-detection
description: Fix Terminal I/O error when binary run via curl|sh pipe on macOS
date: 2026-05-20
---

## Task

Fix "Terminal I/O error: Failed to initialize input reader" on macOS when binary
is run via `curl ... | sh`. Add a test that detects this before ratatui::init().

## Plan

### Task 1: Add NotATerminal error variant
- File: `src/error.rs`
- Add `NotATerminal` variant with user-friendly message

### Task 2: Add TTY guard in main.rs
- File: `src/main.rs`
- Check `std::io::stdin().is_terminal()` after preflight, before `ratatui::init()`
- Return `AppError::NotATerminal` if stdin is not a TTY

### Task 3: Add integration test
- File: `tests/main_integration_test.rs`
- New test: run binary with `Stdio::null()` stdin, assert clean exit + correct message
- Update existing clean-repo test comment to reflect new TTY guard behavior
