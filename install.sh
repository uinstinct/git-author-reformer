#!/usr/bin/env sh
# install.sh — One-shot: detect platform, download binary from GitHub Releases,
# verify SHA256 checksum, execute with user args, clean up temp dir on exit.
# The binary is NOT installed permanently — it runs once and is deleted.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/uinstinct/git-author-reformer/main/install.sh | sh
#   VERSION=v1.2.3 curl -fsSL ... | sh   # pin a specific version
set -eu

REPO="uinstinct/git-author-reformer"
VERSION="${VERSION:-latest}"

# detect_platform <os> <arch>
# Outputs the platform string (e.g., "linux-x86_64") to stdout.
# Exits non-zero with a message on stderr for unsupported combinations.
detect_platform() {
  _os="$1"
  _arch="$2"
  case "${_os}" in
    Linux)
      case "${_arch}" in
        x86_64) echo "linux-x86_64" ;;
        *) printf 'Unsupported Linux arch: %s\n' "${_arch}" >&2; return 1 ;;
      esac ;;
    Darwin)
      case "${_arch}" in
        arm64|aarch64) echo "macos-aarch64" ;;
        x86_64)        echo "macos-x86_64" ;;
        *) printf 'Unsupported macOS arch: %s\n' "${_arch}" >&2; return 1 ;;
      esac ;;
    *)
      printf 'Unsupported OS: %s\n' "${_os}" >&2; return 1 ;;
  esac
}

# verify_checksum <binary_path> <checksum_file>
# Reads expected hash (first field) from checksum_file, computes actual hash of binary_path.
# Exits non-zero with stderr message on mismatch.
verify_checksum() {
  _bin="$1"
  _sha_file="$2"
  _expected="$(awk '{print $1; exit}' "${_sha_file}")"
  _actual="$(shasum -a 256 "${_bin}" | awk '{print $1}')"
  if [ "${_expected}" != "${_actual}" ]; then
    printf 'Checksum verification failed!\nExpected: %s\nActual:   %s\n' \
      "${_expected}" "${_actual}" >&2
    return 1
  fi
}

# INSTALL_TEST_MODE=1 is set by the test harness before sourcing this file.
# When set, return immediately after loading functions so no network calls or
# side effects occur. When executed via 'curl URL | sh', INSTALL_TEST_MODE is
# unset, so the main body runs normally.
[ "${INSTALL_TEST_MODE:-0}" = "1" ] && return 0

PLATFORM="$(detect_platform "$(uname -s)" "$(uname -m)")"

if [ "${VERSION}" = "latest" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -1 \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
fi

BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
BINARY_NAME="git-author-reformer-${PLATFORM}"
TMPDIR_WORK="$(mktemp -d)"
trap 'rm -rf "${TMPDIR_WORK}"' EXIT

printf 'Downloading %s %s...\n' "${BINARY_NAME}" "${VERSION}" >&2
curl -fsSL "${BASE_URL}/${BINARY_NAME}" -o "${TMPDIR_WORK}/git-author-reformer"
curl -fsSL "${BASE_URL}/${BINARY_NAME}.sha256" -o "${TMPDIR_WORK}/git-author-reformer.sha256"

verify_checksum "${TMPDIR_WORK}/git-author-reformer" "${TMPDIR_WORK}/git-author-reformer.sha256"

chmod +x "${TMPDIR_WORK}/git-author-reformer"
printf 'Checksum verified. Running git-author-reformer...\n' >&2
# Run as a foreground child, NOT exec. The shell process remains alive after the
# binary exits so the EXIT trap fires and TMPDIR_WORK is removed. Using exec would
# replace the shell process and the trap would never fire, leaking the temp directory.
"${TMPDIR_WORK}/git-author-reformer" "$@"
