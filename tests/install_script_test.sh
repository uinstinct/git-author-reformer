#!/usr/bin/env sh
# Tests for install.sh — platform detection and checksum verification.
# Runs with plain POSIX sh (no bats dependency).
# Sources install.sh with INSTALL_TEST_MODE=1 so the main download body is skipped.
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_TEST_MODE=1
export INSTALL_TEST_MODE
# shellcheck source=../install.sh
. "${SCRIPT_DIR}/install.sh"

PASS=0; FAIL=0
pass() { echo "PASS: $1"; PASS=$((PASS+1)); }
fail() { echo "FAIL: $1" >&2; FAIL=$((FAIL+1)); }

# ---------------------------------------------------------------------------
# Platform detection tests (Tests 1-6)
# ---------------------------------------------------------------------------

# Test 1: Linux x86_64 -> linux-x86_64
if [ "$(detect_platform 'Linux' 'x86_64')" = "linux-x86_64" ]; then
  pass "detect_platform Linux x86_64 -> linux-x86_64"
else
  fail "detect_platform Linux x86_64 -> linux-x86_64"
fi

# Test 2: Darwin arm64 -> macos-aarch64
if [ "$(detect_platform 'Darwin' 'arm64')" = "macos-aarch64" ]; then
  pass "detect_platform Darwin arm64 -> macos-aarch64"
else
  fail "detect_platform Darwin arm64 -> macos-aarch64"
fi

# Test 3: Darwin aarch64 -> macos-aarch64 (alternate arm64 spelling)
if [ "$(detect_platform 'Darwin' 'aarch64')" = "macos-aarch64" ]; then
  pass "detect_platform Darwin aarch64 -> macos-aarch64"
else
  fail "detect_platform Darwin aarch64 -> macos-aarch64"
fi

# Test 4: Darwin x86_64 -> macos-x86_64
if [ "$(detect_platform 'Darwin' 'x86_64')" = "macos-x86_64" ]; then
  pass "detect_platform Darwin x86_64 -> macos-x86_64"
else
  fail "detect_platform Darwin x86_64 -> macos-x86_64"
fi

# Test 5: Unsupported Linux arch (aarch64) exits non-zero
if ! detect_platform "Linux" "aarch64" >/dev/null 2>&1; then
  pass "detect_platform Linux aarch64 exits non-zero (unsupported arch)"
else
  fail "detect_platform Linux aarch64 should exit non-zero but did not"
fi

# Test 6: Unsupported OS (Windows_NT) exits non-zero
if ! detect_platform "Windows_NT" "x86_64" >/dev/null 2>&1; then
  pass "detect_platform Windows_NT x86_64 exits non-zero (unsupported OS)"
else
  fail "detect_platform Windows_NT x86_64 should exit non-zero but did not"
fi

# ---------------------------------------------------------------------------
# Checksum verification tests (Tests 7-8)
# ---------------------------------------------------------------------------

# Test 7: Checksum mismatch exits 1 before chmod
T7_DIR="$(mktemp -d)"
printf 'FAKE_BINARY_CONTENT' > "${T7_DIR}/git-author-reformer"
# Write a deliberately wrong hash (64 zeros — valid hex length but wrong value)
printf '%s  git-author-reformer\n' \
  "0000000000000000000000000000000000000000000000000000000000000000" \
  > "${T7_DIR}/git-author-reformer.sha256"
if ( verify_checksum "${T7_DIR}/git-author-reformer" "${T7_DIR}/git-author-reformer.sha256" ) 2>/dev/null; then
  fail "checksum mismatch should exit non-zero but exited 0"
else
  # Confirm executable bit was NOT set (verify_checksum must not chmod)
  if [ -x "${T7_DIR}/git-author-reformer" ]; then
    fail "checksum mismatch: executable bit should NOT be set but it is"
  else
    pass "checksum mismatch exits non-zero and file has no executable bit"
  fi
fi
rm -rf "${T7_DIR}"

# Test 8: Checksum match exits 0
T8_DIR="$(mktemp -d)"
printf 'FAKE_BINARY_CONTENT' > "${T8_DIR}/git-author-reformer"
# Compute the real hash and write the .sha256 sidecar
shasum -a 256 "${T8_DIR}/git-author-reformer" > "${T8_DIR}/git-author-reformer.sha256"
if verify_checksum "${T8_DIR}/git-author-reformer" "${T8_DIR}/git-author-reformer.sha256" 2>/dev/null; then
  pass "checksum match exits 0"
else
  fail "checksum match should exit 0 but did not"
fi
rm -rf "${T8_DIR}"

# ---------------------------------------------------------------------------
# Results
# ---------------------------------------------------------------------------
echo "Results: ${PASS} passed, ${FAIL} failed"
exit $FAIL
