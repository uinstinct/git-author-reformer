# Phase 4: CI + Distribution - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning
**Mode:** Auto-generated (discuss skipped via workflow.skip_discuss)

<domain>
## Phase Boundary

Pre-built static binaries on GitHub Releases with a curl install command for all three target platforms.

**Requirements**: DIST-01, DIST-02, DIST-03, DIST-04, DIST-05

**Success Criteria:**
1. Running the curl install command on Linux x86_64 downloads the correct binary, verifies its SHA256 checksum, and executes the tool
2. Running the curl install command on macOS Apple Silicon (aarch64) and macOS Intel (x86_64) each downloads the correct binary, verifies its checksum, and executes the tool
3. Pushing a git tag triggers the GitHub Actions CI workflow, which builds and uploads all three release binaries automatically
4. The Linux binary has no dynamic library dependencies (verified with `ldd` showing "not a dynamic executable")

**Key constraints:**
- Linux target: `x86_64-unknown-linux-musl` (musl, not glibc) — genuinely static, no dynamic deps
- macOS aarch64: native build on `macos-14` runner (ARM)
- macOS x86_64: native build on `macos-13` runner (Intel)
- Never use `actions-rs/*` — use `dtolnay/rust-toolchain` and shell commands directly
- Docker/cross-rs forbidden — no Docker-based cross-compilation

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

</decisions>

<code_context>
## Existing Code Insights

Codebase context will be gathered during plan-phase research.

</code_context>

<specifics>
## Specific Ideas

No specific requirements — discuss phase skipped. Refer to ROADMAP phase description and success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — discuss phase skipped.

</deferred>
