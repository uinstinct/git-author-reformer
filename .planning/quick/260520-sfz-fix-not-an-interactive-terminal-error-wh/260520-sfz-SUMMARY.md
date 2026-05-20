---
quick_id: 260520-sfz
status: complete
date: 2026-05-20
---

# Quick Task 260520-sfz: Fix not-an-interactive-terminal error

## What Changed

**Root cause:** `install.sh` line 88 ran the binary immediately after downloading
(`"${DEST}" "$@"`). When invoked via `curl URL | sh`, stdin is a pipe (not a TTY),
so the binary's TTY guard fired with "Not an interactive terminal."

**Fix — install.sh:**
- Removed the auto-run line (`"${DEST}" "$@"`)
- Updated header comment: removed "and run" from "save to the current directory, and run"
- Added `Run with: ./git-author-reformer` hint to the success message (both new-download and reuse paths)

**Fix — README.md:**
- Quick Start: added explicit `./git-author-reformer` step after the curl command
- Usage section: replaced `curl | sh` + "The TUI launches automatically" with explicit two-step (`curl | sh` then `./git-author-reformer`)
- Line 5: updated "download and run with one command" → "download with one command, then run directly"

## Result

Users running `curl ... | sh` now see:
```
Checksum verified. Binary saved as ./git-author-reformer
Run with: ./git-author-reformer
```

Running `./git-author-reformer` directly opens the TUI as expected.
