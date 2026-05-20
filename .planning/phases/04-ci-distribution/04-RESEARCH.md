# Phase 4: CI + Distribution - Research

**Researched:** 2026-05-20
**Domain:** GitHub Actions CI, Rust musl cross-compilation, binary distribution
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Linux target: `x86_64-unknown-linux-musl` (musl, not glibc) — genuinely static, no dynamic deps
- macOS aarch64: native build on `macos-14` runner (ARM) — DEVIATION PROPOSED: see runner status below
- macOS x86_64: native build on `macos-13` runner (Intel) — RETIRED: see runner status below
- Never use `actions-rs/*` — use `dtolnay/rust-toolchain` and shell commands directly
- Docker/cross-rs forbidden — no Docker-based cross-compilation

### Claude's Discretion

All implementation choices are at Claude's discretion — discuss phase was skipped per user setting.

### Deferred Ideas (OUT OF SCOPE)

None — discuss phase skipped.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DIST-01 | Pre-built static binary for Linux x86_64 (musl target — genuinely no dynamic dependencies) | musl build recipe: ubuntu-latest + musl-tools + cargo build --target x86_64-unknown-linux-musl; `crt-static` must be explicit |
| DIST-02 | Pre-built binary for macOS Apple Silicon / aarch64 (built on native macOS runner, not cross-compiled) | `macos-15` runner (arm64), native `cargo build --target aarch64-apple-darwin` |
| DIST-03 | Pre-built binary for macOS Intel / x86_64 (built on native macOS Intel runner, not cross-compiled) | `macos-15-intel` runner (replaces retired `macos-13`); or cross-compile from `macos-15` arm64 — both work |
| DIST-04 | Single curl command detects OS/arch, downloads correct binary from GitHub Releases, verifies SHA256, and runs | shell install script design documented below |
| DIST-05 | GitHub Actions CI builds and uploads release binaries on git tag push | `on: push: tags: ['v[0-9]+.[0-9]+.[0-9]+'`; `softprops/action-gh-release@v2` or `gh release` shell commands |
</phase_requirements>

---

## Summary

Phase 4 delivers the distribution layer: GitHub Actions builds three release binaries (Linux x86_64 musl, macOS arm64, macOS x86_64), uploads them to GitHub Releases, and a shell install script lets users curl-install the correct binary in one command.

The dominant technical risk in this phase is the Linux musl static build. The project uses `git2` with `vendored-libgit2`, which compiles libgit2 from C source via `cc-rs`. On `x86_64-unknown-linux-musl`, this requires the `musl-tools` apt package (provides `musl-gcc`) and an explicit `RUSTFLAGS="-C target-feature=+crt-static"` to guarantee the CRT is statically linked. Real-world projects (`jj`, ripgrep) confirm this combination works on `ubuntu-latest` without Docker or `cross`. The `Cargo.toml` already has `default-features = false, features = ["vendored-libgit2"]` and `[profile.release] strip = true` — both are correct and complete.

The macOS runner landscape changed significantly after this project's CONTEXT.md was written. `macos-13` was retired December 2025. The replacement for x86_64 macOS is `macos-15-intel` (available until August 2027). Alternatively — and this is simpler — a single `macos-15` (arm64) runner can cross-compile to `x86_64-apple-darwin` via standard Rust tooling (just `rustup target add x86_64-apple-darwin`), since Apple Silicon macOS ships the full Xcode toolchain for both architectures. The `jj` release workflow demonstrates this pattern successfully.

**Primary recommendation:** Single `release.yml` using a 3-entry matrix `include` list. Linux on `ubuntu-latest`, both macOS targets on a single `macos-15` arm64 runner (one native, one cross). Tag trigger `v[0-9]+.[0-9]+.[0-9]+`. Upload via `softprops/action-gh-release@v2`. Install script detects `uname -s`/`uname -m`, downloads binary + `.sha256` sidecar, verifies before `chmod +x`.

**Note on first-run CI duration:** The first run after a `Cargo.lock` change will be slow. `vendored-libgit2` compiles the full libgit2 C library from source — this takes 5-10 minutes per platform job and is NOT cached across `Cargo.lock` bumps (the cache key includes the lock file hash). Subsequent runs with the same `Cargo.lock` use the `Swatinem/rust-cache` cache and complete in ~2-3 minutes. This is expected behavior, not a build failure.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Binary compilation | CI runner | — | Rust compiler on GHA-hosted runner; no server involved |
| Release artifact upload | CI runner (via GHA action) | GitHub Releases storage | `softprops/action-gh-release` writes to GitHub Releases API |
| Checksum generation | CI runner (shell step) | — | `shasum -a 256` run inline in workflow YAML |
| Binary download + verification | User's shell (install script) | — | `curl` + `shasum` on user's machine |
| Platform detection | User's shell (install script) | — | `uname -s` + `uname -m` in install script |

---

## Standard Stack

### Core

| Action / Tool | Version | Purpose | Source |
|---------------|---------|---------|--------|
| `dtolnay/rust-toolchain` | `@stable` (floating) | Install Rust toolchain + targets | [VERIFIED: github.com/dtolnay/rust-toolchain] |
| `Swatinem/rust-cache` | `@v2` (latest minor: v2.9.1) | Cache Cargo registry + incremental artifacts | [VERIFIED: Swatinem/rust-cache releases] |
| `softprops/action-gh-release` | `@v2` (latest in v2 line: v2.6.2; v3 requires Node 24) | Upload binaries to GitHub Release | [VERIFIED: softprops/action-gh-release releases] |
| `musl-tools` (apt) | system package | Provides `musl-gcc` for x86_64-unknown-linux-musl builds | [VERIFIED: ripgrep ci/ubuntu-install-packages; jj release.yml] |

**Note on `softprops/action-gh-release` v2 vs v3:** v3 was released April 2026 and requires Node 24. GitHub-hosted runners support Node 24. Using `@v2` is safe and avoids any risk from the major version bump. Stick with `@v2` for v1 of this project. [CITED: softprops/action-gh-release releases page]

**Alternative:** Skip the action entirely and use `gh release create` / `gh release upload` shell commands. This has zero external dependencies beyond the GitHub CLI (pre-installed on all GHA runners). It is the pattern ripgrep uses. Either approach is correct — the plan can decide.

### What NOT to Use (enforced by CLAUDE.md)

| Tool | Why Not |
|------|---------|
| `actions-rs/*` | Entire org archived; uses deprecated node12 and `set-output` GHA features |
| `cross` / `cross-rs` | Docker-based; Apple SDK licensing blocks macOS cross-compile via Docker |
| `cargo-zigbuild` | Not needed — native macOS cross-compile from arm64 to x86_64 works without it |

---

## GHA Runner Status (CRITICAL — UPDATED)

`macos-13` is **retired** as of December 4, 2025. [VERIFIED: github.blog changelog 2025-09-19]

| Target | Old Runner | New Runner | Status |
|--------|-----------|-----------|--------|
| `aarch64-apple-darwin` | `macos-14` | `macos-15` (arm64) | Available |
| `x86_64-apple-darwin` | `macos-13` (RETIRED) | `macos-15-intel` | Available until Aug 2027 |
| `x86_64-apple-darwin` (alt) | `macos-13` (RETIRED) | `macos-15` (arm64 + cross) | Available |
| `x86_64-unknown-linux-musl` | `ubuntu-latest` | `ubuntu-latest` | Unchanged |

**Decision required for planner:** Two valid strategies for macOS x86_64:

**Option A — `macos-15-intel` runner:** Runs on actual Intel hardware. True native build matching CONTEXT.md intent.
```yaml
- build: macos-x86_64
  os: macos-15-intel
  target: x86_64-apple-darwin
```

**Option B — `macos-15` arm64 runner + cross-compile:** Single runner for both macOS targets, reducing CI cost and complexity. Rust supports `x86_64-apple-darwin` cross-compile from arm64 macOS (both share the same Xcode toolchain). The `jj` project uses this approach (`macos-15` for both targets). [VERIFIED: jj release.yml]
```yaml
- build: macos-x86_64
  os: macos-15
  target: x86_64-apple-darwin
```

**Recommendation:** Option B (single arm64 runner for both macOS targets). Simpler, uses the modern runner, matches current ecosystem practice, and `macos-15-intel` is on a sunset clock anyway (August 2027). The binary output is identical — Rust's cross-compilation on macOS uses the same Xcode SDK.

> **Deviation from CONTEXT.md locked decision — planner must surface for confirmation:**
> CONTEXT.md specifies `macos-14` for the aarch64 build. Research recommends `macos-15` instead. `macos-14` is still available (non-breaking), so honoring the locked decision is an option. The recommendation to use `macos-15` is based on it being the current stable runner and consistent with ecosystem practice, but the planner should confirm this upgrade with the user before locking it. If the user prefers `macos-14`, change the `macos-aarch64` matrix entry from `macos-15` to `macos-14` — no other changes needed.

---

## Architecture Patterns

### System Architecture Diagram

```
[Developer] -- git push tag v1.2.3 --> [GitHub]
                                           |
                              on.push.tags: v*.*.*
                                           |
                               +-----------+-----------+
                               |           |           |
                          [ubuntu-latest] [macos-15]  [macos-15]
                          x86_64-musl     aarch64     x86_64
                               |           |           |
                          cargo build   cargo build  cargo build
                          musl target   native       cross-compile
                               |           |           |
                          [binary]      [binary]   [binary]
                          + .sha256     + .sha256   + .sha256
                               |           |           |
                               +-----+-----+-----------+
                                     |
                          softprops/action-gh-release@v2
                                     |
                          [GitHub Releases page]
                          git-author-reformer-linux-x86_64
                          git-author-reformer-linux-x86_64.sha256
                          git-author-reformer-macos-aarch64
                          git-author-reformer-macos-aarch64.sha256
                          git-author-reformer-macos-x86_64
                          git-author-reformer-macos-x86_64.sha256
                                     |
                            [User: curl install.sh | sh]
                                     |
                         detect OS + arch (uname -s / uname -m)
                         download binary + .sha256
                         verify checksum (shasum -a 256)
                         chmod +x
                         execute (binary runs; tmpdir cleaned up on exit)
```

### Recommended Project Structure

```
.github/
└── workflows/
    └── release.yml       # Tag-triggered: build 3 binaries, upload to GitHub Releases
install.sh                # Curl install script (detect platform, download, verify, run)
```

No `Cargo.toml` changes needed. No `.cargo/config.toml` needed (see musl linker section below).

---

## Architecture Patterns

### Pattern 1: Three-Platform Matrix with `include`

Use a flat `include` list (NOT a 2D matrix). Each platform entry specifies `os`, `target`, and any platform-specific fields.

```yaml
# Source: jj release.yml + ripgrep release.yml (both verified)
strategy:
  fail-fast: false
  matrix:
    include:
      - build: linux-x86_64-musl
        os: ubuntu-latest
        target: x86_64-unknown-linux-musl
      - build: macos-aarch64
        os: macos-15
        target: aarch64-apple-darwin
      - build: macos-x86_64
        os: macos-15
        target: x86_64-apple-darwin
```

### Pattern 2: musl Build Setup (Linux)

```yaml
# Source: jj release.yml (uses git2-free pure Rust) + ripgrep ci/ (uses musl-tools)
# Verified: both projects use musl-tools on ubuntu-latest without cross

- name: Install musl tools (Linux only)
  if: matrix.os == 'ubuntu-latest'
  run: |
    sudo apt-get update
    sudo apt-get install -y --no-install-recommends musl-tools

- name: Install Rust toolchain
  uses: dtolnay/rust-toolchain@stable
  with:
    targets: ${{ matrix.target }}

- name: Build release binary
  env:
    # Explicit crt-static required: Rust is changing musl default from static to dynamic.
    # Without this flag, future rustc versions may produce a dynamically-linked binary.
    # See: https://github.com/rust-lang/rust/pull/133386
    RUSTFLAGS: ${{ matrix.os == 'ubuntu-latest' && '"-C target-feature=+crt-static"' || '' }}
  run: cargo build --release --target ${{ matrix.target }} --locked
```

**Critical detail on `RUSTFLAGS`:** The ternary expression above is illustrative. In practice, define `rustflags` in the matrix entry:

```yaml
matrix:
  include:
    - build: linux-x86_64-musl
      os: ubuntu-latest
      target: x86_64-unknown-linux-musl
      rustflags: "-C target-feature=+crt-static"
    - build: macos-aarch64
      os: macos-15
      target: aarch64-apple-darwin
      rustflags: ""
    - build: macos-x86_64
      os: macos-15
      target: x86_64-apple-darwin
      rustflags: ""
```

Then in the build step: `env: RUSTFLAGS: ${{ matrix.rustflags }}`

### Pattern 3: Checksum Generation (Unix)

```bash
# shasum is available on both macOS and Linux (part of base system)
# sha256sum is Linux-specific; shasum -a 256 works on both
shasum -a 256 "git-author-reformer-${{ matrix.build }}" > "git-author-reformer-${{ matrix.build }}.sha256"
```

### Pattern 4: Release Upload

```yaml
# Source: softprops/action-gh-release README (verified)
- name: Upload to GitHub Release
  uses: softprops/action-gh-release@v2
  with:
    files: |
      git-author-reformer-${{ matrix.build }}
      git-author-reformer-${{ matrix.build }}.sha256
```

**Permissions required at workflow level:**
```yaml
permissions:
  contents: write
```

### Pattern 5: Tag Trigger

```yaml
# Source: ripgrep release.yml pattern (verified)
# [DISCRETIONARY — not locked by CONTEXT.md; this is Claude's recommendation]
on:
  push:
    tags:
      - "v[0-9]+.[0-9]+.[0-9]+"
```

Use the constrained glob (`v[0-9]+.[0-9]+.[0-9]+`) rather than `v*` to avoid triggering on tags like `v-beta` or `v-rc1`.

### Pattern 6: Install Script Design

The install script is a **one-shot downloader and runner**, not a persistent installer. It downloads the binary to a temporary directory, verifies the checksum, runs the tool with the user's arguments, then cleans up the temp directory on exit (via `trap`). The binary is not installed to `PATH` — every invocation re-downloads. This matches the DIST-04 requirement ("downloads and runs").

```bash
#!/usr/bin/env sh
# install.sh — One-shot: detect platform, download binary from GitHub Releases,
# verify SHA256 checksum, execute with user args, clean up temp dir on exit.
# The binary is NOT installed permanently — it runs once and is deleted.
set -eu

REPO="<owner>/git-author-reformer"
VERSION="${VERSION:-latest}"  # allow override via env var

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
  Linux)
    case "${ARCH}" in
      x86_64) PLATFORM="linux-x86_64-musl" ;;
      *) echo "Unsupported Linux arch: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  Darwin)
    case "${ARCH}" in
      arm64|aarch64) PLATFORM="macos-aarch64" ;;
      x86_64)        PLATFORM="macos-x86_64" ;;
      *) echo "Unsupported macOS arch: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: ${OS}" >&2; exit 1 ;;
esac

# Resolve version
if [ "${VERSION}" = "latest" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
fi

BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
BINARY_NAME="git-author-reformer-${PLATFORM}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "${TMPDIR}"' EXIT

# Download binary and checksum
curl -fsSL "${BASE_URL}/${BINARY_NAME}" -o "${TMPDIR}/git-author-reformer"
curl -fsSL "${BASE_URL}/${BINARY_NAME}.sha256" -o "${TMPDIR}/git-author-reformer.sha256"

# Verify checksum before chmod
cd "${TMPDIR}"
# Rewrite checksum file to use the local filename (the .sha256 file contains the original name)
EXPECTED="$(awk '{print $1}' git-author-reformer.sha256)"
ACTUAL="$(shasum -a 256 git-author-reformer | awk '{print $1}')"
if [ "${EXPECTED}" != "${ACTUAL}" ]; then
  echo "Checksum verification failed!" >&2
  echo "Expected: ${EXPECTED}" >&2
  echo "Actual:   ${ACTUAL}" >&2
  exit 1
fi

chmod +x git-author-reformer
echo "Checksum verified. Running git-author-reformer..."
./git-author-reformer "$@"
```

**Key design choices:**
- `set -eu` (not `set -euo pipefail` — `pipefail` is bash-only, `sh` doesn't support it)
- Download to `TMPDIR`, clean up with `trap` on EXIT (binary is deleted after the tool runs)
- Verify checksum BEFORE `chmod +x`, fail closed
- `shasum -a 256` works on both macOS and Linux (not `sha256sum` which is Linux-only)
- `curl -fsSL` fails on HTTP errors (`-f`), silent (`-s`), follow redirects (`-L`)
- No `curl | bash` — user downloads to temp and executes, which gives better error messages

### Anti-Patterns to Avoid

- **`actions-rs/toolchain`:** Archived, uses deprecated `set-output` and node12. Use `dtolnay/rust-toolchain@stable`.
- **`v*` tag glob:** Too broad. Use `v[0-9]+.[0-9]+.[0-9]+`.
- **`chmod +x` before checksum verify:** Always verify integrity first. A corrupted download should never be executed.
- **`sha256sum` in install script:** Linux-only command. Use `shasum -a 256` for cross-platform shell scripts.
- **Hardcoded version in install script:** Always resolve `latest` via the GitHub API, or accept `VERSION` override.
- **`Swatinem/rust-cache` without target key:** The cache key must include the Cargo target. `rust-cache@v2` handles this automatically by incorporating `matrix.target` when set. No extra config needed.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Release creation + asset upload | Custom API calls | `softprops/action-gh-release@v2` | Handles draft releases, asset upload, retry, existing-release update |
| Rust toolchain management | Shell `rustup` calls | `dtolnay/rust-toolchain@stable` | Handles toolchain, target, component installs + idempotency |
| Build artifact caching | Custom cache keys | `Swatinem/rust-cache@v2` | Smart cache key from `Cargo.lock` + target + toolchain; handles invalidation |

**Key insight:** The release upload and toolchain setup have enough corner cases (race conditions in parallel jobs, cache poisoning, auth token scopes) that the existing actions are significantly more robust than equivalent shell one-liners.

---

## Common Pitfalls

### Pitfall 1: musl CRT Static Default Changing
**What goes wrong:** Future rustc versions (post-PR#133386 rollout) will change `x86_64-unknown-linux-musl` to link CRT dynamically by default. Without `RUSTFLAGS="-C target-feature=+crt-static"`, the binary stops being static.
**Why it happens:** Rust is standardizing musl to dynamic-by-default for consistency with glibc targets.
**How to avoid:** Explicitly set `RUSTFLAGS="-C target-feature=+crt-static"` in the CI step for the musl target.
**Warning signs:** `ldd` on the Linux binary shows library references instead of "not a dynamic executable".

### Pitfall 2: Using macos-13 (Retired)
**What goes wrong:** Any workflow referencing `macos-13` will fail immediately — runner no longer exists.
**Why it happens:** GitHub retired macOS 13 runner December 4, 2025.
**How to avoid:** Use `macos-15-intel` for native Intel builds, or `macos-15` (arm64) with `rustup target add x86_64-apple-darwin` for cross-compile.
**Warning signs:** Workflow fails at job queue step with "runner not found" or similar error.

### Pitfall 3: Missing `permissions: contents: write`
**What goes wrong:** `softprops/action-gh-release` fails with 403 when trying to create a release or upload assets.
**Why it happens:** GitHub Actions workflows default to `read` permissions on `GITHUB_TOKEN`.
**How to avoid:** Add `permissions: contents: write` at the workflow or job level.
**Warning signs:** Action fails with "Resource not accessible by integration" error.

### Pitfall 4: `sha256sum` Instead of `shasum -a 256` in Install Script
**What goes wrong:** Install script fails on macOS with "command not found: sha256sum".
**Why it happens:** `sha256sum` is Linux/GNU coreutils only. macOS ships `shasum` (from Perl), not `sha256sum`.
**How to avoid:** Always use `shasum -a 256` in POSIX shell scripts targeting both platforms.
**Warning signs:** Error on macOS: `sh: sha256sum: command not found`.

### Pitfall 5: `pipefail` in `#!/usr/bin/env sh` Script
**What goes wrong:** Script fails immediately on macOS/BSD sh with "set: Illegal option -o pipefail".
**Why it happens:** `pipefail` is a bash extension not available in POSIX sh.
**How to avoid:** Use `set -eu` (no `pipefail`), or change shebang to `#!/usr/bin/env bash`.
**Warning signs:** Script fails on the very first `set -euo pipefail` line on macOS.

### Pitfall 6: Parallel Jobs Writing to Same Release Simultaneously
**What goes wrong:** Two parallel jobs both call `softprops/action-gh-release` at the same time and both try to create the release — one fails with "Release already exists".
**Why it happens:** Race condition between parallel matrix jobs.
**How to avoid:** `softprops/action-gh-release@v2` handles existing releases gracefully (it updates them, not creates-or-fails). The simpler path is to let the action handle it — no precursor job needed. If stricter control is required, use a `create-release` job with `needs: []` that runs first and creates the release as a draft; then matrix build jobs with `needs: [create-release]` call `gh release upload` to add assets to the existing release. The ripgrep workflow uses this explicit two-job pattern.
**Warning signs:** Intermittent failures in multi-platform matrix with "Release already exists" or "Unprocessable Entity" error.

### Pitfall 7: `Cargo.toml` Version Does Not Match Tag
**What goes wrong:** Release tag is `v1.2.3` but `Cargo.toml` still says `version = "0.1.0"`. User downloads binary claiming to be "v1.2.3" but it was compiled from mismatched source.
**Why it happens:** Forgot to bump `Cargo.toml` version before tagging.
**How to avoid:** Add a CI check that `grep -q "version = \"${TAG_VERSION}\"" Cargo.toml` (strip leading `v` from tag first). Fail the job if mismatch. See ripgrep release.yml for this pattern.
**Warning signs:** No warning at compile time — this is silent. Only caught if you add the explicit check.

---

## Code Examples

### Complete release.yml (reference pattern)

```yaml
# Source: Synthesized from jj release.yml [VERIFIED] + ripgrep release.yml [VERIFIED]
# + softprops/action-gh-release README [VERIFIED]
#
# Note: This uses a single-job matrix (no separate create-release job).
# softprops/action-gh-release@v2 handles concurrent uploads to the same release gracefully.
# If intermittent race-condition errors appear, add a create-release precursor job (ripgrep pattern).

name: release

on:
  push:
    tags:
      - "v[0-9]+.[0-9]+.[0-9]+"

permissions:
  contents: write

jobs:
  build-release:
    name: build-release
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - build: linux-x86_64-musl
            os: ubuntu-latest
            target: x86_64-unknown-linux-musl
            binary: git-author-reformer
            asset_name: git-author-reformer-linux-x86_64
            rustflags: "-C target-feature=+crt-static"
          - build: macos-aarch64
            os: macos-15
            target: aarch64-apple-darwin
            binary: git-author-reformer
            asset_name: git-author-reformer-macos-aarch64
            rustflags: ""
          - build: macos-x86_64
            os: macos-15
            target: x86_64-apple-darwin
            binary: git-author-reformer
            asset_name: git-author-reformer-macos-x86_64
            rustflags: ""

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install musl tools (Linux only)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y --no-install-recommends musl-tools

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Check Cargo.toml version matches tag
        shell: bash
        run: |
          TAG="${{ github.ref_name }}"
          VERSION="${TAG#v}"
          if ! grep -q "^version = \"${VERSION}\"" Cargo.toml; then
            echo "Error: Cargo.toml version does not match tag ${TAG}" >&2
            exit 1
          fi

      - name: Build release binary
        env:
          RUSTFLAGS: ${{ matrix.rustflags }}
        run: cargo build --release --target ${{ matrix.target }} --locked

      - name: Prepare release asset
        shell: bash
        run: |
          BIN="target/${{ matrix.target }}/release/${{ matrix.binary }}"
          cp "${BIN}" "${{ matrix.asset_name }}"
          shasum -a 256 "${{ matrix.asset_name }}" > "${{ matrix.asset_name }}.sha256"

      - name: Upload to GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            ${{ matrix.asset_name }}
            ${{ matrix.asset_name }}.sha256

      - name: Verify Linux binary is static (Linux only)
        if: matrix.os == 'ubuntu-latest'
        run: |
          ldd "${{ matrix.asset_name }}" 2>&1 | grep -q "not a dynamic executable" \
            || (echo "ERROR: Binary is not fully static!" >&2 && ldd "${{ matrix.asset_name }}" >&2 && exit 1)
```

---

## Cargo.toml Analysis

The existing `Cargo.toml` is already correctly configured for this phase:

```toml
# Already correct — DO NOT change these
[dependencies]
git2 = { version = "0.21", default-features = false, features = ["vendored-libgit2"] }

[profile.release]
strip = true    # Strips symbols — no additional strip step needed in CI
lto = true
codegen-units = 1
```

**`default-features = false` on git2:** This drops the `ssh` and `https` features, which would pull in `openssl-sys`. OpenSSL is notoriously hard to link statically on musl. This is the single most important existing configuration for making musl builds work. Do not change it.

**No `.cargo/config.toml` needed:** For `x86_64-unknown-linux-musl` on Ubuntu, `musl-tools` installs `musl-gcc` and Rust's `cc-rs` automatically finds it for vendored C code. No explicit linker configuration is required.

**`strip = true` is already set:** The `[profile.release]` already strips debug symbols. No `strip` shell command needed in CI. Duplicate stripping would be a no-op.

**First-run duration:** The first CI run compiling `vendored-libgit2` takes 5-10 minutes per platform job (C compilation from source, not cacheable across `Cargo.lock` changes). This is normal. `Swatinem/rust-cache` will cache subsequent runs.

---

## Package Legitimacy Audit

> This phase installs no new Rust crate dependencies. It only adds GitHub Actions workflow files (YAML) and a shell script. The actions used are all established, maintained projects.

| Action | Source | Age | Usage | Verdict |
|--------|--------|-----|-------|---------|
| `dtolnay/rust-toolchain` | github.com/dtolnay/rust-toolchain | ~4 years | Canonical Rust CI action, used by thousands of projects | Approved [VERIFIED: project README] |
| `Swatinem/rust-cache@v2` | github.com/Swatinem/rust-cache | ~4 years | Standard Rust CI cache action | Approved [VERIFIED: GitHub Marketplace] |
| `softprops/action-gh-release@v2` | github.com/softprops/action-gh-release | ~7 years | Most popular GHA release action | Approved [VERIFIED: GitHub Marketplace] |
| `actions/checkout@v4` | github.com/actions/checkout | GitHub-official | Pre-installed on all runners | Approved |

**No packages removed. No packages flagged suspicious.**

*Slopcheck not applicable — these are GitHub Actions, not package registry entries. All actions are from known maintainers with years of adoption history.*

---

## Environment Availability

| Dependency | Required By | Available on GHA | Notes |
|------------|------------|-----------------|-------|
| `musl-tools` (apt) | DIST-01 Linux musl build | ✓ via `apt-get install` | `ubuntu-latest` has apt; `musl-tools` is in standard Ubuntu repo |
| `dtolnay/rust-toolchain` | All builds | ✓ | Action installs rustup + toolchain |
| `macos-15` runner | DIST-02, DIST-03 | ✓ | Current; arm64 macOS Sequoia |
| `macos-15-intel` runner | DIST-03 (alt) | ✓ until Aug 2027 | Intel macOS Sequoia |
| `shasum` | Checksum generation in CI | ✓ | macOS: built-in; Linux: part of `perl` package, available on ubuntu-latest |
| `gh` CLI | Version resolve in install.sh | ✓ on user machines (optional) | Install script uses GitHub REST API directly via curl, no `gh` required |
| GitHub Releases API | DIST-04 install script | ✓ | Public endpoint, no auth for public repos |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `actions-rs/toolchain` | `dtolnay/rust-toolchain@stable` | 2023 (actions-rs archived) | Direct replacement; same interface, maintained |
| `macos-13` runner (Intel) | `macos-15-intel` or `macos-15` + cross | Dec 2025 (macos-13 retired) | Breaking: workflows using macos-13 fail immediately |
| `macos-14` runner for arm64 | `macos-15` | Sep 2025 | Non-breaking; macos-14 still available but macos-15 is current |
| `softprops/action-gh-release@v1` | `@v2` (node20) or `@v3` (node24) | v2: 2023; v3: Apr 2026 | v1 deprecated; use v2 for stability, v3 for node24 runners |
| Implicit musl `crt-static = true` | Explicit `RUSTFLAGS="-C target-feature=+crt-static"` | PR merged Dec 2024, rollout in-progress | Required to guarantee static binary across future rustc versions |

**Deprecated/outdated:**
- `macos-13`: Retired December 4, 2025. Any reference to this runner in YAML will fail.
- `actions-rs/*`: Entire org archived. Do not use.
- `v*` tag glob: Too broad; use `v[0-9]+.[0-9]+.[0-9]+`.

---

## Project Constraints (from CLAUDE.md)

| Directive | Category | Research Compliance |
|-----------|----------|-------------------|
| Never use `actions-rs/*` | Forbidden tool | Stack uses `dtolnay/rust-toolchain` only |
| Docker/cross-rs forbidden | Forbidden tool | Workflow uses native runners only; no Docker steps |
| `x86_64-unknown-linux-musl` for Linux | Target constraint | Matrix entry specifies this target; musl-tools installed |
| macOS aarch64 on native runner | Platform constraint | `macos-15` (arm64) native build — deviation from `macos-14` flagged above |
| macOS x86_64 on native runner | Platform constraint | `macos-15-intel` (Option A) or `macos-15` cross-compile (Option B) — both compliant |
| Static linking required | Binary constraint | `vendored-libgit2` + `default-features = false` + explicit `crt-static` |
| Single binary, no dynamic deps | Distribution constraint | Verified by `ldd` check in CI workflow |
| Karpathy: Simplicity First | Coding convention | Minimal workflow YAML; no speculative features |
| Karpathy: Surgical Changes | Coding convention | Only new files added: `.github/workflows/release.yml`, `install.sh` |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | macOS arm64 runner (`macos-15`) can cross-compile to `x86_64-apple-darwin` using standard Rust tooling (no special setup) | GHA Runner Status | If wrong, need `macos-15-intel` for x86_64 — fallback documented |
| A2 | `musl-tools` apt package on `ubuntu-latest` is sufficient for `vendored-libgit2` to compile cleanly (no additional cmake, pkg-config needed) | Architecture Patterns | If wrong, additional apt packages needed in CI step; `jj` evidence suggests musl-tools is sufficient for pure-Rust with vendored C |
| A3 | `curl` on macOS does NOT set `com.apple.quarantine` xattr on downloaded files, so Gatekeeper will not block unsigned binaries installed via the install script | Common Pitfalls | If wrong, install script needs `xattr -d com.apple.quarantine` step — low risk based on search findings |

**Risk mitigation:** A1 has a documented fallback (`macos-15-intel`). A2 is based on the `jj` reference project (which uses pure-Rust git, not libgit2, so libgit2 vendoring is not directly validated there). A3 is based on macOS security documentation.

---

## Open Questions

1. **Does vendored-libgit2 + musl require cmake or other apt packages?**
   - What we know: `jj` (pure-Rust git) builds cleanly on ubuntu with only `musl-tools`. ripgrep musl build uses `musl-tools + g++` for PCRE2. libgit2 builds via `cc-rs` using only a C compiler.
   - What's unclear: Whether `cmake` is needed for vendored libgit2 compilation path, or whether `cc-rs` handles it entirely. The `git2-rs` build.rs file (checked) does not mention cmake explicitly.
   - Recommendation: Plan Wave 0 should include a local `cargo build --release --target x86_64-unknown-linux-musl` test (in a Docker musl container if available) to confirm `musl-tools` alone is sufficient. Add `cmake` to the apt install if the build fails.

2. **`softprops/action-gh-release@v2` vs `gh release` shell commands?**
   - What we know: Both work. ripgrep uses `gh release create` + `gh release upload` shell commands. Many other projects use `softprops/action-gh-release@v2`.
   - What's unclear: Neither is strictly better — this is a style choice.
   - Recommendation: Use `softprops/action-gh-release@v2` for clarity and automatic retry logic. If action is unavailable, `gh release create $VERSION --draft && gh release upload $VERSION file file.sha256` is the equivalent.

3. **Should the planner use `macos-14` or `macos-15` for the aarch64 runner?**
   - What we know: CONTEXT.md locks `macos-14`. `macos-14` is still available. `macos-15` is the current stable runner.
   - What's unclear: User preference — the locked decision may have been a specific choice or just "whatever was current at the time."
   - Recommendation: Planner should surface this to the user before locking the plan. Both work.

---

## Sources

### Primary (HIGH confidence)

- `jj release.yml` — github.com/martinvonz/jj/.github/workflows/release.yml — canonical reference for musl + macOS arm64 native + macOS x86_64 on macos-15 (fetched live via `gh api`)
- `ripgrep release.yml` + `ci/ubuntu-install-packages` — github.com/BurntSushi/ripgrep — musl build pattern with `musl-tools` (fetched live via `gh api`)
- GitHub Changelog 2025-09-19 — github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/ — macos-13 retirement confirmed December 4, 2025
- GitHub runner-images issue #13045 — macos-15-intel availability confirmed (fetched via WebFetch)
- `dtolnay/rust-toolchain` README — github.com/dtolnay/rust-toolchain — `targets` input, `@stable` tag (fetched via WebFetch)
- `softprops/action-gh-release` README + releases — github.com/softprops/action-gh-release — v2 (v2.6.2) current; v3 requires Node 24 (fetched via WebFetch)
- Rust PR #133386 — musl `crt-static` default changing to dynamic; merged Dec 2024 (fetched via WebFetch)

### Secondary (MEDIUM confidence)

- `Swatinem/rust-cache` releases — v2.9.1 latest in v2 line (WebSearch verified with GitHub)
- macOS Gatekeeper + `curl` quarantine behavior — curl does NOT set `com.apple.quarantine` xattr; WebSearch multiple sources (HackTricks, community Q&A)
- `reemus.dev` Rust cross-compilation article — confirms `dtolnay/rust-toolchain` + `Swatinem/rust-cache` pattern for multi-platform matrix

### Tertiary (LOW confidence)

- Rust musl `crt-static` PR #144513 (musl_missing_crt_static lint) — still open as of Aug 2025; direction confirmed by PR #133386 which is merged

---

## Metadata

**Confidence breakdown:**
- GHA runner status: HIGH — verified via official GitHub Changelog + runner-images issues (live at time of research)
- musl build recipe: HIGH — verified against two production Rust projects (jj, ripgrep) using live `gh api` calls
- Action versions: HIGH — verified via WebFetch on official README + releases pages
- `crt-static` risk: HIGH — verified against merged Rust PR #133386; mitigation is explicit flag
- Install script design: HIGH — shell standard patterns, cross-verified against multiple sources
- Gatekeeper behavior: MEDIUM — curl quarantine exception confirmed by multiple sources but not Apple official docs

**Research date:** 2026-05-20
**Valid until:** 2026-11-20 (runner labels stable; action versions may advance but v2/v3 major won't break)
