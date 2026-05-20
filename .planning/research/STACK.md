# Stack: git-author-reformer

**Researched:** 2026-05-20
**Overall confidence:** HIGH (all versions verified against crates.io and official docs)

---

## Recommended Stack

### Core

| Library | Version | Role | Confidence |
|---------|---------|------|------------|
| `ratatui` | `0.30.0` | TUI rendering and widget layout | HIGH — verified via `cargo search` |
| `crossterm` | `0.29.0` | Terminal backend for ratatui (cross-platform) | HIGH — verified via `cargo search` |
| `git2` | `0.21.0` | libgit2 bindings: repo open, revwalk, commit creation, ref update | HIGH — verified via `cargo search` |
| `nucleo` | `0.5.0` | Fuzzy matching for author selector (same engine as Helix editor) | HIGH — verified via `cargo search` |
| `tui-textarea` | `0.7.0` | Optional: inline text-edit widget if commit message editing is added | MEDIUM — exists, confirmed ratatui 0.29+ compatible |
| `clap` | `4.6.1` | CLI entry point (`--version`, `--help`); derive macro preferred | HIGH — verified via `cargo search` |

### Rationale

**ratatui 0.30:** The de-facto standard Rust TUI library as of 2025. Provides `List` + `ListState` for scrollable, selectable item lists — exactly what the author-selector screens need. No built-in text input or fuzzy search; those come from separate crates (see nucleo below). Ratatui explicitly delegates event handling to the backend: use `crossterm::event::{Event, KeyEvent, KeyCode}` directly in the application event loop.

**crossterm 0.29:** The default and recommended ratatui backend. Cross-platform (Linux, macOS, Windows). The version pinned must match ratatui's expected crossterm version — ratatui 0.30 uses crossterm 0.29. Termion and termwiz are alternatives but termion is Linux/macOS-only and termwiz is heavier; crossterm is the right pick for a general-distribution binary.

**git2 0.21 (libgit2 via `vendored-libgit2`):** The load-bearing choice. git2 exposes exactly the APIs needed for a full filter-branch-equivalent rewrite:
- `Repository::open_from_env()` — detect repo from CWD by walking up (same semantics as git)
- `Revwalk` + `push_glob("*")` — walk ALL commits reachable from ALL refs (all branches, tags)
- `Sort::TOPOLOGICAL | Sort::REVERSE` — walk oldest-first so parent OIDs are always known before children when building the new graph
- `Commit::author()`, `Commit::committer()`, `Commit::time()`, `Commit::message()`, `Commit::tree()`, `Commit::parent_ids()` — read everything needed to reconstruct a commit
- `Repository::commit()` — create a new commit with modified author/message, existing tree, and remapped parent OIDs
- `Repository::reference()` with `force: true` — update each branch/tag ref to point at the new tip after the rewrite
- `Commit::amend()` — available for single-commit amend; for a full graph rewrite, the `Repository::commit()` + ref-update loop is the right pattern (amend only works on the tip of a ref, not arbitrary ancestors)

**nucleo 0.5:** High-performance fuzzy matcher (the exact engine used by the Helix editor). Preferred over the older `fuzzy-matcher` crate because it is actively maintained and significantly faster on large inputs. The author-selector will show Name + Email pairs — nucleo lets users type to filter the list without a separate text-input widget unless inline editing is desired. `nucleo-matcher` is the underlying engine crate; `nucleo` (the meta crate) wraps it with a higher-level API including threading support. For this tool's size (at most thousands of unique authors) both work; use `nucleo` for simplicity.

**clap 4.6 (derive feature):** Even for a TUI-first tool, `--version` and `--help` flags are table stakes for discoverability (the curl-and-run audience will try `--help`). Clap derive is the most ergonomic approach. If the app grows to add `--repo <path>` in a future phase, clap is already in place. `pico-args` would be fine if the binary size budget were extreme, but it is not for this tool.

**tui-textarea 0.7:** Not needed for v1 (the tool edits author name/email, not commit messages, and confirmation is a Y/N prompt). Include it as a documented option for any future "edit commit message" feature.

---

### What NOT to Use

| Library | Why Not |
|---------|---------|
| `actions-rs/toolchain` | **Archived and unmaintained.** Uses deprecated `node12` and `set-output` GHA features. Every major Rust project has migrated away. Use `dtolnay/rust-toolchain@stable` instead. |
| `cross` (cross-rs) | Docker-based; cannot cross-compile to macOS from Linux without the Apple SDK, which Docker images cannot legally include. Adds complexity with no benefit given the native-runner strategy below. |
| `termion` | Linux/macOS only. crossterm is strictly better for a tool distributed to both platforms. |
| `fuzzy-matcher` | Older, slower than nucleo. No longer maintained as actively. |
| `tokio` / `async-std` | No async I/O needed. TUI event loops are synchronous by nature (crossterm's event polling is blocking with a timeout). Adding an async runtime increases binary size and complexity for zero benefit. |
| Raw `.git/` pack-file parsing | Weeks of complexity to handle all edge cases (thin packs, delta chains, alternates). git2/libgit2 is battle-tested and purpose-built. |

---

## Cargo.toml Feature Flags for Static Linking

```toml
[dependencies]
git2 = { version = "0.21", default-features = false, features = [
    "vendored-libgit2",   # statically compile libgit2 from source
    "vendored-openssl",   # statically compile OpenSSL (needed for HTTPS, bundled for completeness)
] }

ratatui = { version = "0.30", features = ["crossterm"] }
crossterm = "0.29"
nucleo = "0.5"
clap = { version = "4.6", features = ["derive"] }
```

**Why `default-features = false` on git2:** The default features include `ssh` and `https` which pull in OpenSSL dynamically on some platforms. Since this tool only opens local repos (no network operations), disabling them reduces attack surface and simplifies linking. Add `https` back only if remote operations are ever needed.

**Note on `vendored-openssl`:** Even with network features disabled, `vendored-openssl` ensures no system OpenSSL dependency leaks in. Safe to include; it's a no-op if OpenSSL is not otherwise pulled in.

---

## Cross-Compilation Strategy

### Platform Constraints

| Target | Triplet | Strategy | Notes |
|--------|---------|----------|-------|
| Linux x86_64 | `x86_64-unknown-linux-musl` | musl static | Fully static binary; no glibc dependency, runs on any Linux kernel ≥ 3.2 |
| macOS Apple Silicon | `aarch64-apple-darwin` | Native build on `macos-14` runner | macOS cannot produce fully static binaries; libSystem is always dynamic (OS constraint, not a build tool limitation) |
| macOS Intel | `x86_64-apple-darwin` | Native build on `macos-13` runner | Same macOS constraint; `macos-13` is the last GHA-hosted Intel runner |

**Critical macOS caveat:** macOS does not support fully static binaries. `libSystem.dylib` (the macOS libc equivalent) must always be linked dynamically. This is an Apple OS constraint, not a Rust or libgit2 limitation. The correct framing: the macOS binaries are "self-contained" (libgit2 and OpenSSL are compiled in statically) but they depend on the system-provided `libSystem`. This is fine for distribution — `libSystem` ships with every macOS installation. Do NOT describe these as "fully statically linked" in user-facing docs.

### Linux musl — Additional Build Flag

For the musl target, set `CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER` or use a toolchain that handles musl linking:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

On GitHub Actions, the `ubuntu-latest` runner has `musl-tools` available via `apt-get install musl-tools`. The `vendored-libgit2` feature compiles libgit2 from source and links it statically, which works with musl.

Alternatively, `cargo-zigbuild` (v0.22.3 as of research date) can replace the linker step and handles musl transparently without needing `musl-tools`. It also allows specifying minimum glibc versions for GNU targets. For this project's Linux target (musl-only), direct musl toolchain is simpler and has fewer moving parts; cargo-zigbuild is a valid fallback if link errors arise.

---

## Build / CI Setup

### GitHub Actions Workflow Structure

**Recommended:** Three separate native jobs (not cross-compilation from Linux):

```yaml
strategy:
  matrix:
    include:
      - target: x86_64-unknown-linux-musl
        os: ubuntu-latest
        binary_suffix: ""
      - target: aarch64-apple-darwin
        os: macos-14        # ARM64 runner (Apple Silicon)
        binary_suffix: ""
      - target: x86_64-apple-darwin
        os: macos-13        # Intel runner (last hosted Intel macOS)
        binary_suffix: ""
```

**Why native runners for macOS:** Cross-compiling macOS targets from Linux requires the Apple macOS SDK, which Apple does not freely license for redistribution. Third-party solutions exist (osxcross) but add significant CI complexity and maintenance burden. Native runners on `macos-14` (ARM) and `macos-13` (Intel) are simpler, legally unambiguous, and produce verified-native output. The cost difference (macOS runners are 10x the price of Linux) is acceptable for a small open-source tool with infrequent releases.

### Required Actions (no deprecated tools)

```yaml
steps:
  - uses: actions/checkout@v4

  - uses: dtolnay/rust-toolchain@stable
    with:
      targets: ${{ matrix.target }}

  - uses: Swatinem/rust-cache@v2
    # Caches ~/.cargo registry + build artifacts per target triple

  # Linux only: install musl linker
  - name: Install musl tools
    if: matrix.target == 'x86_64-unknown-linux-musl'
    run: sudo apt-get install -y musl-tools

  - name: Build release binary
    run: cargo build --release --target ${{ matrix.target }}

  - name: Upload artifact
    uses: actions/upload-artifact@v4
    with:
      name: git-author-reformer-${{ matrix.target }}
      path: target/${{ matrix.target }}/release/git-author-reformer
```

**Action versions:**
- `dtolnay/rust-toolchain@stable` — maintained by dtolnay (prolific Rust ecosystem contributor), actively updated. Replaces the archived `actions-rs/toolchain`.
- `Swatinem/rust-cache@v2` — the standard Rust CI cache action; caches registry and incremental build artifacts per toolchain version and target.
- `actions-rs/*` — DO NOT USE. Entire organization is archived; uses deprecated GHA features.

### Release Workflow

On tag push (`v*`), build all three targets in parallel, then create a GitHub Release with:
- `git-author-reformer-linux-x86_64` (musl binary, strip symbols for size)
- `git-author-reformer-macos-aarch64`
- `git-author-reformer-macos-x86_64`

Strip release binaries with `strip target/.../release/git-author-reformer` to reduce size (especially important for the Linux musl build which can be large before stripping).

---

## Minimum Rust Version

Stable Rust. No nightly features required. Set `rust-version = "1.74"` in `Cargo.toml` (the MSRV that shipped ratatui 0.26+ compatibility; 0.30 likely requires newer — verify against ratatui's own MSRV when pinning). In practice, CI should use `dtolnay/rust-toolchain@stable` which always tracks the latest stable release.

---

## Dependency Tree Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| libgit2 C compilation fails on musl | Low | `vendored-libgit2` compiles from source; musl-tools provides the needed headers |
| macOS runner `macos-13` is retired | Medium (Intel Macs deprecated) | Monitor GHA runner announcements; migrate to cross-compile from `macos-14` via `x86_64` cross when needed |
| ratatui breaking API changes | Low | 0.30 is a stable release; pin with `=0.30.0` if needed |
| nucleo API churn | Low | `nucleo` 0.5 is stable; the lower-level `nucleo-matcher` crate is even more stable if needed |

---

## Sources

- ratatui version: `cargo search ratatui` — `0.30.0`
- git2 version: `cargo search git2` — `0.21.0`
- crossterm version: `cargo search crossterm` — `0.29.0`
- nucleo version: `cargo search nucleo` — `0.5.0`
- clap version: `cargo search clap` — `4.6.1`
- git2 feature flags: [git2-rs Cargo.toml on GitHub](https://github.com/rust-lang/git2-rs/blob/master/Cargo.toml)
- git2 Repository API: [docs.rs/git2/latest/git2/struct.Repository.html](https://docs.rs/git2/latest/git2/struct.Repository.html)
- git2 Commit API: [docs.rs/git2/latest/git2/struct.Commit.html](https://docs.rs/git2/latest/git2/struct.Commit.html)
- git2 Revwalk API: [docs.rs/git2/latest/git2/struct.Revwalk.html](https://docs.rs/git2/latest/git2/struct.Revwalk.html)
- ratatui widgets: [docs.rs/ratatui/latest/ratatui/widgets/index.html](https://docs.rs/ratatui/latest/ratatui/widgets/index.html)
- ratatui event model: no built-in input handling; use crossterm::event directly (official docs)
- actions-rs deprecated: [tauri-apps/tauri issue #8078](https://github.com/tauri-apps/tauri/issues/8078)
- macos-14 = ARM64 only: [actions/runner-images issue #9741](https://github.com/actions/runner-images/issues/9741)
- Swatinem/rust-cache: [GitHub Marketplace](https://github.com/marketplace/actions/rust-cache)
- cargo-zigbuild version: [rust-cross/cargo-zigbuild releases](https://github.com/rust-cross/cargo-zigbuild/releases) — v0.22.3
- macOS static binary constraint: Apple OS design; libSystem always dynamic
