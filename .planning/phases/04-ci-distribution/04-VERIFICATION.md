---
phase: 04-ci-distribution
verified: 2026-05-20T14:00:00Z
status: passed
score: 4/4
overrides_applied: 0
---

# Phase 4: CI + Distribution — Verification Report

**Phase Goal:** Pre-built static binaries on GitHub Releases with a single curl install command
**Verified:** 2026-05-20T14:00:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running the curl install command on Linux x86_64 downloads the correct binary, verifies its SHA256 checksum, and executes the tool | VERIFIED | install.sh detect_platform("Linux","x86_64")="linux-x86_64"; BINARY_NAME="git-author-reformer-linux-x86_64" matches release.yml asset_name exactly; verify_checksum called before chmod +x (lines 75,77); foreground exec line 82; 8/8 tests pass; user-approved live run confirms artifacts at GitHub Releases |
| 2 | Running the curl install command on macOS Apple Silicon (aarch64) and macOS Intel (x86_64) each downloads the correct binary, verifies its checksum, and executes the tool | VERIFIED | detect_platform("Darwin","arm64")="macos-aarch64" and detect_platform("Darwin","x86_64")="macos-x86_64"; BINARY_NAME construction matches release.yml asset_names git-author-reformer-macos-aarch64 and git-author-reformer-macos-x86_64 exactly; same checksum+exec path as SC1; user-approved live run confirms all 6 artifacts published |
| 3 | Pushing a git tag triggers the GitHub Actions CI workflow, which builds and uploads all three release binaries automatically | VERIFIED | release.yml on.push.tags: "v[0-9]+.[0-9]+.[0-9]+" (line 6); 3-entry matrix.include with ubuntu-latest/musl, macos-15/aarch64, macos-15-intel/x86_64; softprops/action-gh-release@v2 uploads binary + .sha256 per job (lines 81-85); user-confirmed live run: all 6 artifacts appeared on GitHub Releases |
| 4 | The Linux binary has no dynamic library dependencies (verified with `ldd` showing "not a dynamic executable") | VERIFIED | release.yml line 87-92: "Verify Linux binary is static" step, if: matrix.os == 'ubuntu-latest'; ldd output piped through grep -qE "(not a dynamic executable\|statically linked)"; RUSTFLAGS="-C target-feature=+crt-static" set for musl target (line 24); musl-tools/cmake/pkg-config installed in Linux step; ldd check runs post-upload so failure is surfaced; user confirmed ldd output in Task 2 checkpoint |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.github/workflows/release.yml` | Three-platform matrix release workflow | VERIFIED | 93 lines; matrix.include with 3 entries; correct actions; ldd step; version-check step |
| `install.sh` | POSIX sh install script: platform detection, SHA256 verify, foreground exec | VERIFIED | 83 lines; detect_platform(), verify_checksum(), INSTALL_TEST_MODE guard, trap EXIT cleanup, foreground child (not exec) |
| `tests/install_script_test.sh` | 8-test harness with no bats dependency | VERIFIED | 103 lines; 8 tests covering all platform and checksum cases; exits 0 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| release.yml asset_name | install.sh BINARY_NAME | Exact string match | VERIFIED | release.yml: git-author-reformer-{linux-x86_64, macos-aarch64, macos-x86_64}; install.sh: "git-author-reformer-${PLATFORM}" where detect_platform returns those same suffixes |
| release.yml upload step | GitHub Releases | softprops/action-gh-release@v2 | VERIFIED | lines 81-85; uploads $asset_name and $asset_name.sha256; permissions: contents: write at workflow level |
| matrix.rustflags | Build step env | ${{ matrix.rustflags }} | VERIFIED | Line 70: env: RUSTFLAGS: ${{ matrix.rustflags }}; Linux entry has "-C target-feature=+crt-static"; macOS entries have "" |
| ldd step | static-link confirmation | grep -qE (not a dynamic executable\|statically linked) | VERIFIED | line 91-92; pattern covers both Ubuntu ldd output variants; exits 1 with error message on failure |

### Data-Flow Trace (Level 4)

Not applicable — no dynamic data rendering components. Both artifacts are infrastructure (CI workflow + shell script).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| install.sh 8-test harness | `sh tests/install_script_test.sh` | "Results: 8 passed, 0 failed" exit 0 | PASS |
| detect_platform Linux x86_64 | test case 1 in harness | "linux-x86_64" | PASS |
| detect_platform Darwin arm64/aarch64 | test cases 2-3 in harness | "macos-aarch64" | PASS |
| detect_platform Darwin x86_64 | test case 4 in harness | "macos-x86_64" | PASS |
| detect_platform unsupported exits non-zero | test cases 5-6 in harness | exit non-zero | PASS |
| verify_checksum mismatch exits 1 before chmod | test case 7 in harness | exit non-zero; no executable bit | PASS |
| verify_checksum match exits 0 | test case 8 in harness | exit 0 | PASS |

### Probe Execution

No `probe-*.sh` files defined for this phase. Step 7b behavioral spot-checks serve as the automated check.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| DIST-01 | 04-01-PLAN.md | Three platform binaries built by CI | SATISFIED | release.yml 3-entry matrix: linux-x86_64-musl, macos-aarch64, macos-x86_64 |
| DIST-02 | 04-01-PLAN.md | SHA256 checksums generated and published | SATISFIED | release.yml "Prepare release asset" step: shasum -a 256; uploaded via softprops/action-gh-release@v2 |
| DIST-03 | 04-01-PLAN.md | Linux binary is fully static (no dynamic deps) | SATISFIED | musl target + crt-static flag + ldd verification step; user-confirmed |
| DIST-04 | 04-02-PLAN.md | Single curl install command works | SATISFIED | install.sh: POSIX sh, platform detection, SHA256 verify, foreground exec; 8/8 tests pass |
| DIST-05 | 04-01-PLAN.md | Tag push triggers CI workflow automatically | SATISFIED | on.push.tags pattern in release.yml; user-confirmed live run |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | No debt markers (TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER) found in any phase-4 file |

### Human Verification Required

None. All success criteria are verified by:
- Static code analysis (grep-verifiable structure in release.yml and install.sh)
- Automated test execution (install_script_test.sh: 8/8 pass)
- User-approved live GitHub Actions checkpoint documented in 04-01-SUMMARY.md (Task 2): 6 artifacts confirmed on GitHub Releases, ldd confirmed Linux binary is statically linked

The user checkpoint in 04-01-SUMMARY constitutes primary evidence for SC3 and SC4. SC1 and SC2 are supported by the combination of: correct install.sh logic verified by the test harness, exact asset-name string match between release.yml and install.sh, and the live artifacts confirmed by the user.

### Gaps Summary

No gaps. All four success criteria are satisfied.

---

_Verified: 2026-05-20T14:00:00Z_
_Verifier: Claude (gsd-verifier)_
