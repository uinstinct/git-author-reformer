---
phase: 01-foundation-read-layer
plan: 01
subsystem: infra
tags: [rust, git2, libgit2, clap, thiserror, tempfile, vendored]

requires: []

provides:
  - Buildable Rust crate with vendored libgit2 (no git binary at runtime)
  - AppError enum: NotARepo, StashDetected, WorktreesDetected, Git variants
  - AuthorIdentity and CoAuthorEntry structs (name, email, commit_count)
  - open_repo() calling Repository::open_from_env()
  - Stub enumerate_authors / enumerate_coauthors (todo! for Plan 03)
  - Stub check_stash / check_worktrees (todo! for Plan 02)
  - --version and --help via clap derive
  - Shared test fixture helpers: create_fixture_repo(), add_commit_with_message()

affects: [01-02-preflight, 01-03-reader, 01-04-wiring, all-future-phases]

tech-stack:
  added:
    - "git2 0.21 (default-features=false, vendored-libgit2)"
    - "thiserror 2"
    - "clap 4.6 with derive feature"
    - "tempfile 3 (dev-dep)"
  patterns:
    - "AppError via thiserror::Error derive — all public fns return Result<_, AppError>"
    - "Fully-qualified paths in stub modules (no use clutter) to match interface contracts"
    - "_repo parameter prefix to silence unused-variable warnings on stubs"
    - "tests/common/mod.rs convention for shared integration-test helpers"

key-files:
  created:
    - Cargo.toml
    - Cargo.lock
    - src/error.rs
    - src/main.rs
    - src/git/mod.rs
    - src/git/types.rs
    - src/git/reader.rs
    - src/git/preflight.rs
    - tests/common/mod.rs
  modified: []

key-decisions:
  - "git2 default-features=false disables ssh/https to eliminate OpenSSL from the build (CORE-03, musl-safe)"
  - "Stub bodies use todo!(\"implemented in Plan NN\") not unimplemented!() — documents intentional deferral"
  - "tests/common/mod.rs over tests/fixtures.rs: bare .rs files under tests/ each become their own test binary; common/ subdirectory avoids that"
  - "open_repo() maps git2::Error to AppError::NotARepo (not AppError::Git) — callers need the specific variant for exit-code wiring in Plan 04"

patterns-established:
  - "Pattern: All public functions return Result<_, crate::error::AppError>"
  - "Pattern: Stub modules use _param prefix to silence warnings without blanket allow(dead_code)"
  - "Pattern: Test fixtures use fixed git2::Time::new(1_000_000, 0) for reproducibility"

requirements-completed: [CORE-02, CORE-03]

duration: ~45min
completed: 2026-05-20
---

# Phase 01 Plan 01: Project Scaffolding Summary

**Rust crate scaffold with vendored libgit2 (no ssh/https/OpenSSL), AppError/types/open_repo contracts, and stub modules for Plan 02/03 TDD**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-05-20
- **Completed:** 2026-05-20
- **Tasks:** 3 (Task 1 was pre-approved checkpoint; Tasks 2, 3, 4 executed)
- **Files modified:** 9 created

## Accomplishments

- Cargo manifest with vendored libgit2 (no ssh/https features — CORE-03 satisfied, musl-link-safe)
- AppError enum with all four required variants and exact Display strings from Pattern 7
- AuthorIdentity + CoAuthorEntry structs, open_repo() wired to Repository::open_from_env()
- Stub reader and preflight modules compile with todo! bodies — Plan 02 and 03 fill them in via TDD
- Binary responds to --help and --version (clap derive)
- Shared test fixture helpers under tests/common/mod.rs (no git binary — pure git2)

## Task Commits

Each task was committed atomically:

1. **Task 2: Cargo.toml with vendored libgit2** - `dc35c2b` (feat)
2. **Task 3: AppError, types, and module skeleton** - `6d0d7cd` (feat) — includes Cargo.lock
3. **Task 4: Shared test fixtures** - `84534ca` (feat)

## Contract: Stub Signatures for Plan 02 / Plan 03

Plans 02 and 03 must implement against these exact signatures (do not change):

```rust
// src/git/reader.rs — implemented in Plan 03
pub fn enumerate_authors(_repo: &git2::Repository) -> Result<Vec<crate::git::types::AuthorIdentity>, crate::error::AppError>
pub fn enumerate_coauthors(_repo: &git2::Repository) -> Result<Vec<crate::git::types::CoAuthorEntry>, crate::error::AppError>

// src/git/preflight.rs — implemented in Plan 02
pub fn check_stash(_repo: &git2::Repository) -> Result<(), crate::error::AppError>
pub fn check_worktrees(_repo: &git2::Repository) -> Result<(), crate::error::AppError>
```

## Cargo.toml Dependency Block (for reference)

```toml
[dependencies]
git2 = { version = "0.21", default-features = false, features = ["vendored-libgit2"] }
thiserror = "2"
clap = { version = "4.6", features = ["derive"] }

[dev-dependencies]
tempfile = "3"
```

## AppError Variants (exact Display strings — Plan 04 relies on these)

```rust
NotARepo(String)         → "Not inside a git repository: {0}"
StashDetected            → "Stash entries detected. Pop or drop all stashes before rewriting history.\nRun: git stash list"
WorktreesDetected(String)→ "Linked worktrees detected: {0}\nRemove worktrees before rewriting history.\nRun: git worktree list"
Git(#[from] git2::Error) → "Git error: {0}"
```

## tests/common/mod.rs Convention

- Location: `tests/common/mod.rs` (NOT `tests/fixtures.rs`)
- Reason: bare `.rs` files under `tests/` are each compiled as standalone test binaries; the `common/` subdirectory convention prevents that and allows shared code
- Access from integration tests: `mod common;` declaration in sibling test files
- **Compile status:** The file is syntactically correct but NOT compile-verified by `cargo test --no-run` in this plan — the helpers only compile when a sibling test file declares `mod common;`. First real compile happens in Plan 02 or 03.

## Files Created

- `Cargo.toml` — manifest with vendored libgit2, no ssh/https
- `Cargo.lock` — committed (binary crate convention)
- `src/error.rs` — AppError enum with four variants
- `src/main.rs` — clap derive CLI with --version / --help; no preflight wiring (Plan 04)
- `src/git/mod.rs` — module facade + open_repo()
- `src/git/types.rs` — AuthorIdentity + CoAuthorEntry structs
- `src/git/reader.rs` — stubs for Plan 03 TDD
- `src/git/preflight.rs` — stubs for Plan 02 TDD
- `tests/common/mod.rs` — create_fixture_repo() + add_commit_with_message()

## Decisions Made

1. `default-features = false` on git2 — disables ssh/https features which pull OpenSSL; this is REQUIRED for the Linux musl static-link target (CORE-03). Without it, the musl build fails with `undefined reference to 'dlopen'`.
2. `todo!("implemented in Plan NN")` bodies — documents intentional deferral vs. accidental incompleteness
3. `_repo` prefix on stub parameters — silences unused-variable warnings without blanket `#[allow(dead_code)]` that would persist into Plan 02/03 implementations
4. `open_repo()` maps git2 error to `AppError::NotARepo` (not `AppError::Git`) — the calling code in Plan 04 needs to distinguish repo-not-found from other git errors
5. clap `#[command(name = "git-author-reformer", version)]` — `version` attribute reads from Cargo.toml automatically; no about string yet (Plan 04 may add one)

## Deviations from Plan

### Auto-handled Issues

**1. [Rule 3 - Blocking] CLT ranlib SIP-protected on local machine**
- **Found during:** Task 3 (first `cargo build`)
- **Issue:** `/Library/Developer/CommandLineTools/usr/bin/ranlib` is a 7-byte SIP stub that cannot be executed. The CLT `ar` tool calls it internally, causing `fatal error: can't find or exec: ranlib (Operation not permitted)`.
- **Root cause:** macOS SIP protects certain CLT stubs. This is a local machine environment issue, NOT a project bug. GitHub Actions `macos-14` runners have a full Xcode installation where this issue does not occur.
- **Fix:** Created `/tmp/ar-wrapper.sh` that translates `ar cq output.a obj...` calls to `/usr/bin/libtool -static -o output.a obj...`. Build invoked as: `AR=/tmp/ar-wrapper.sh RANLIB=/usr/bin/ranlib cargo build`.
- **NOT committed:** The wrapper is local-only. A `.cargo/config.toml` pointing at `/tmp/` was deliberately not committed (Karpathy Rule 2 — no speculative flexibility; Posture A per advisor guidance).
- **CI impact:** None. CI runners won't hit this issue. Anyone else on macOS with a full Xcode install won't hit this either.
- **Developer workaround:** If another developer hits this, install full Xcode or use: `AR=/tmp/ar-wrapper.sh RANLIB=/usr/bin/ranlib cargo build` with the wrapper script content from this SUMMARY.

**2. [Rule 3 - Blocking] Task 2 cargo build deferred to Task 3**
- **Found during:** Task 2
- **Issue:** `cargo build` cannot run with only Cargo.toml (no src/main.rs or lib target). The plan's Task 2 `acceptance_criteria` says "cargo build exits 0" but the `<done>` note clarifies it runs "against a minimal src/main.rs (created in Task 3)" — an internal plan contradiction.
- **Fix:** Verified Cargo.toml grep checks in Task 2, deferred the build check to Task 3 (where src/main.rs exists). Documented in SUMMARY.
- **Committed in:** Build verified as part of Task 3 commit `6d0d7cd`.

---

**Total deviations:** 2 auto-handled (both Rule 3 — blocking environment issues)
**Impact on plan:** No scope creep. Both issues were environmental/procedural, not architectural.

## Build Environment Notes

The `cargo build` must_have is satisfied on:
- GitHub Actions `macos-14` / `macos-13` runners (full Xcode)
- Linux (no CLT/SIP issue)
- macOS with full Xcode.app installed (`xcode-select -p` pointing to Xcode.app, not CLT)

If you hit the CLT ranlib error locally, use this wrapper:

```bash
cat > /tmp/ar-wrapper.sh << 'WRAPPER'
#!/bin/bash
shift
OUTPUT="$1"; shift
if [ $# -eq 0 ]; then
    printf "int dummy() {}" > /tmp/ar-empty.c
    clang -c /tmp/ar-empty.c -o /tmp/ar-empty.o 2>/dev/null
    exec /usr/bin/libtool -static -o "$OUTPUT" /tmp/ar-empty.o
fi
exec /usr/bin/libtool -static -o "$OUTPUT" "$@"
WRAPPER
chmod +x /tmp/ar-wrapper.sh
AR=/tmp/ar-wrapper.sh RANLIB=/usr/bin/ranlib cargo build
```

## Known Warnings

8 `dead_code` / `unused` warnings in the debug build — all intentional:
- `AppError` enum, `open_repo()`, `AuthorIdentity`, `CoAuthorEntry` — unused until Plan 04 wires main.rs
- `enumerate_authors`, `enumerate_coauthors` — stubs, implemented in Plan 03
- `check_stash`, `check_worktrees` — stubs, implemented in Plan 02

These warnings will resolve as downstream plans implement and wire the functions.

## Issues Encountered

None beyond the CLT ranlib issue documented under Deviations.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 02 (preflight TDD): can implement `check_stash` / `check_worktrees` in `src/git/preflight.rs` immediately
- Plan 03 (reader TDD): can implement `enumerate_authors` / `enumerate_coauthors` in `src/git/reader.rs` immediately
- Both plans use `tests/common/mod.rs` via `mod common;` — first real compile-check of the helpers happens here
- Plan 04 (wiring): can wire `main.rs` once Plans 02+03 replace the stubs

---
*Phase: 01-foundation-read-layer*
*Completed: 2026-05-20*

## Self-Check: PASSED

**File existence checks:**
- [x] `Cargo.toml` exists
- [x] `Cargo.lock` exists
- [x] `src/error.rs` exists
- [x] `src/main.rs` exists
- [x] `src/git/mod.rs` exists
- [x] `src/git/types.rs` exists
- [x] `src/git/reader.rs` exists
- [x] `src/git/preflight.rs` exists
- [x] `tests/common/mod.rs` exists

**Commit existence checks:**
- [x] `dc35c2b` — feat(01-01): add Cargo.toml with vendored libgit2 dependencies
- [x] `6d0d7cd` — feat(01-01): add AppError, data types, and module skeleton
- [x] `84534ca` — feat(01-01): add shared test fixtures under tests/common/
