---
quick_id: 260520-sfz
slug: fix-not-an-interactive-terminal-error-wh
description: Fix not-an-interactive-terminal error when running git-author-reformer directly
date: 2026-05-20
mode: quick
must_haves:
  truths:
    - install.sh no longer auto-runs the binary after downloading
    - install.sh success message tells users to run ./git-author-reformer themselves
  artifacts:
    - install.sh
    - README.md
---

# Quick Task 260520-sfz: Fix not-an-interactive-terminal error

## Root Cause

`install.sh` line 88 auto-runs the downloaded binary: `"${DEST}" "$@"`. When
the script is invoked via `curl URL | sh`, stdin is a pipe (not a TTY), so the
binary's TTY guard fires with:

```
error: Not an interactive terminal.
git-author-reformer is a TUI application — run it directly, not inside a pipe.
```

## Fix

Remove the auto-run. The install script's job is to download and save the
binary — not to launch it. Users run `./git-author-reformer` themselves.

## Tasks

### Task 1 — Remove auto-run from install.sh

**Files:** `install.sh`
**Action:**
1. Remove line 88: `"${DEST}" "$@"`
2. Update line 4 comment: change "save to the current directory, and run" → "save to the current directory"
3. Update the success printf message (line 83) to include a "Run with:" hint

**Verify:** `install.sh` has no line that executes `${DEST}` or `$DEST`
**Done:** install.sh no longer invokes the binary

### Task 2 — Update README.md if it references auto-run

**Files:** `README.md`
**Action:** Check if README mentions the install script running the binary automatically. If so, update to clarify the script only downloads, and the user runs the binary themselves.
**Verify:** README correctly describes the two-step workflow: download, then run
**Done:** README is consistent with the new behavior
