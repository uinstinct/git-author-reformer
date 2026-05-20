---
phase: 04-ci-distribution
plan: "02"
subsystem: infra
tags: [shell, install-script, posix-sh, sha256, github-releases, curl]

# Dependency graph
requires: []
provides:
  - POSIX sh install script that detects platform, downloads binary from GitHub Releases, verifies SHA256, and executes
  - Shell test harness for platform detection and checksum verification (8 tests, no bats dependency)
affects: [distribution, release, curl-install]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "INSTALL_TEST_MODE=1 guard: set env before sourcing shell script to skip main body in tests"
    - "Foreground child (not exec) for trap-based TMPDIR cleanup"
    - "shasum -a 256 (not sha256sum) for cross-platform checksum in POSIX sh"
    - "set -eu (not set -euo pipefail) for POSIX sh compatibility on macOS"

key-files:
  created:
    - install.sh
    - tests/install_script_test.sh
  modified: []

key-decisions:
  - "Foreground child (not exec) so EXIT trap fires and TMPDIR_WORK is cleaned after binary exits"
  - "verify_checksum called before chmod +x (fail closed — corrupted download never executed)"
  - "shasum -a 256 used throughout; sha256sum absent (Linux-only, fails on macOS)"
  - "INSTALL_TEST_MODE guard via [ ${INSTALL_TEST_MODE:-0} = 1 ] && return 0 (not case $0 pattern)"
  - "TMPDIR_WORK named to avoid shadowing the TMPDIR reserved env var"

patterns-established:
  - "Shell scripts in this project use #!/usr/bin/env sh and set -eu (no pipefail)"
  - "Test harnesses for shell scripts source the script with INSTALL_TEST_MODE=1 to load functions without side effects"

requirements-completed:
  - DIST-04

# Metrics
duration: 2min
completed: 2026-05-20
---

# Phase 4 Plan 02: Install Script Summary

**POSIX sh one-shot install script with platform detection, SHA256 verification before chmod, foreground child execution, and an 8-test harness covering all platform and checksum cases**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-05-20T13:10:56Z
- **Completed:** 2026-05-20T13:12:51Z
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files modified:** 2

## Accomplishments

- `install.sh`: detect_platform(os, arch) handles Linux x86_64, Darwin arm64/aarch64, Darwin x86_64; exits non-zero with stderr "Unsupported" for other combos
- `install.sh`: verify_checksum() reads expected hash from .sha256 sidecar, computes actual with `shasum -a 256`, exits 1 with "Checksum verification failed" on mismatch
- `install.sh`: verify_checksum called before chmod +x; binary runs as foreground child (not exec) so EXIT trap fires and TMPDIR_WORK is deleted after the binary exits
- `tests/install_script_test.sh`: 8-test plain POSIX sh harness — all 8 pass (exit 0), no network calls in test mode
- All structural constraints verified: zero sha256sum, zero pipefail, zero exec occurrences; shasum -a 256, chmod +x, trap EXIT, "Checksum verification failed", "Unsupported" all present

## Task Commits

1. **Task 1 RED: test harness** - `6f0d2e0` (test)
2. **Task 1 GREEN: install.sh** - `545092f` (feat)

## Files Created/Modified

- `install.sh` — one-shot POSIX sh install script: platform detection, SHA256 verify, foreground binary execution
- `tests/install_script_test.sh` — 8-test harness: 4 platform detection cases, 2 unsupported platform cases, 2 checksum cases

## Decisions Made

- `shasum -a 256` throughout — `sha256sum` is Linux-only and fails on macOS; `shasum` is available on both
- Foreground child (`"${TMPDIR_WORK}/git-author-reformer" "$@"`) instead of `exec` so the EXIT trap fires after the binary finishes and TMPDIR_WORK is removed
- `INSTALL_TEST_MODE=1` guard with `return 0` (not a `case "$0"` pattern) — cleaner and the test harness simply sets the env var before sourcing
- `TMPDIR_WORK` (not `TMPDIR`) for the mktemp directory — `TMPDIR` is a reserved env var; overwriting it breaks subsequent `mktemp` calls
- `set -eu` without `pipefail` — `pipefail` is bash-only; macOS `/bin/sh` rejects it at startup

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## Known Stubs

None — install.sh is complete and functional. The REPO value (`uinstinct/git-author-reformer`) and binary names are real and will work once GitHub Releases are populated by the CI workflow (Plan 04-01).

## Threat Flags

No new threat surface beyond what the plan's threat model already covers. All T-04-05 through T-04-09 mitigations are implemented:
- T-04-05: SHA256 verified before execution
- T-04-07: verify_checksum before chmod +x, tested by Test 7
- T-04-09: Foreground child + EXIT trap; no exec

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- DIST-04 satisfied: install.sh is ready for distribution alongside GitHub Releases
- Depends on Phase 04-01 (release.yml) producing binaries with .sha256 sidecars
- No blockers

## Self-Check

- `install.sh` exists: YES
- `tests/install_script_test.sh` exists: YES
- RED commit `6f0d2e0` exists: YES
- GREEN commit `545092f` exists: YES
- All 8 tests pass: YES (exit 0)
- sha256sum occurrences: 0
- pipefail occurrences: 0
- exec occurrences: 0

## Self-Check: PASSED

---
*Phase: 04-ci-distribution*
*Completed: 2026-05-20*
