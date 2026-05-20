---
phase: quick-260520-rbp
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - scripts/release.sh
autonomous: true
requirements:
  - QUICK-260520-rbp-01
must_haves:
  truths:
    - "Running scripts/release.sh prompts the user to choose patch or minor"
    - "Cargo.toml [package] version line is updated to the new version after a successful run"
    - "A git tag named v{new_version} is created pointing at HEAD"
    - "Script prints a clear summary of old version, new version, and the tag that was created"
    - "Script aborts cleanly with a non-zero exit code if the working tree is dirty, the version cannot be parsed, or the tag already exists"
  artifacts:
    - path: "scripts/release.sh"
      provides: "Interactive release script: bump Cargo.toml version + create git tag"
      contains: "git tag"
  key_links:
    - from: "scripts/release.sh"
      to: "Cargo.toml"
      via: "in-place sed update of the version = \"X.Y.Z\" line in [package]"
      pattern: "^version ="
    - from: "scripts/release.sh"
      to: "git repository"
      via: "git tag v{version} on HEAD"
      pattern: "git tag"
---

<objective>
Add a Bash release helper script that bumps the Cargo.toml version (patch or minor, user-selected at the prompt) and creates a matching `v{version}` git tag in one step.

Purpose: Make cutting a release a one-command operation so the GitHub Actions release workflow (triggered by tag pushes) can be kicked off reliably without hand-editing Cargo.toml or computing version strings by hand.
Output: `scripts/release.sh` — a pure-bash, dependency-free script with no installation step.
</objective>

<execution_context>
@/Users/instinct/Desktop/working/git-author-reformer/.claude/get-shit-done/workflows/execute-plan.md
@/Users/instinct/Desktop/working/git-author-reformer/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@Cargo.toml
@CLAUDE.md

<interfaces>
<!-- Current Cargo.toml [package] header (excerpt) — the script must locate and update the `version` line in this section only. -->

```toml
[package]
name = "git-author-reformer"
version = "0.1.0"
edition = "2021"
rust-version = "1.74"
```

Constraint: there is also a `[dev-dependencies]` block with a `version =` field for `git2`. The script MUST only touch the `version =` line inside `[package]`, not any dependency `version =` keys.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create scripts/release.sh with version bump + tag</name>
  <files>scripts/release.sh</files>
  <action>
Create `scripts/release.sh` as a pure-bash script (shebang `#!/usr/bin/env bash`, `set -euo pipefail`). The script must:

1. Resolve the repo root from its own location (`SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"`) and `cd "$REPO_ROOT"`. Do NOT assume the user runs it from the repo root.

2. Verify prerequisites and fail with a clear error message + non-zero exit if any fail:
   - `Cargo.toml` exists at the repo root
   - `git` is available on PATH (`command -v git >/dev/null`)
   - Inside a git repository (`git rev-parse --git-dir >/dev/null 2>&1`)
   - Working tree is clean (`git diff-index --quiet HEAD --`). If dirty, print the offending files via `git status --short` and exit 1.

3. Extract the CURRENT version from Cargo.toml's `[package]` section ONLY (the `[dev-dependencies]` block also contains `version = ...` lines for `git2` — those must NOT match). Use an awk-based extractor that tracks the current TOML section:
   - When a line matches `^\[.*\]`, update the current section name.
   - Only when section is `package` AND line matches `^version[[:space:]]*=[[:space:]]*"([^"]+)"`, capture the version.
   - Exit 1 with "Could not find version in [package] section of Cargo.toml" if not found.

4. Validate the extracted version matches `^[0-9]+\.[0-9]+\.[0-9]+$` (strict 3-part semver, no pre-release suffix for now — fail with a clear message if not, so we don't silently corrupt versions like `0.1.0-beta`).

5. Prompt the user interactively with a menu, defaulting to patch on empty input:
   ```
   Current version: X.Y.Z
   Select bump type:
     1) patch  -> X.Y.(Z+1)
     2) minor  -> X.(Y+1).0
   Choice [1]:
   ```
   Accept `1`, `patch`, `p` for patch; `2`, `minor`, `m` for minor. Reject anything else with a clear error and exit 1. Compute NEW_VERSION accordingly. (Note: per the task scope, major bumps are intentionally not offered.)

6. Verify the target tag `v$NEW_VERSION` does not already exist locally (`git rev-parse -q --verify "refs/tags/v$NEW_VERSION"` — if it succeeds, exit 1 with "Tag v$NEW_VERSION already exists").

7. Update Cargo.toml in place. Use awk (not sed — section-aware update is cleaner and more portable across BSD/GNU than sed) to rewrite ONLY the `version = "..."` line inside the `[package]` section, writing to a temp file and `mv`-ing over the original. After writing, re-extract the version from the new Cargo.toml and assert it equals `$NEW_VERSION`; if not, abort and restore from a `.bak` made before the rewrite.

8. Stage Cargo.toml (`git add Cargo.toml`), commit with message `chore: release v$NEW_VERSION`, then create the tag with `git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"` on HEAD.

9. Print a final summary block:
   ```
   Released:
     old version: X.Y.Z
     new version: X.Y.Z'
     commit:      <short sha>
     tag:         vX.Y.Z'

   Next step: git push origin main --follow-tags
   ```
   Do NOT push automatically — leave the push to the user (the release CI is triggered by pushing the tag, so the user should review before pushing).

10. Make the script executable: after writing it, the executor must run `chmod +x scripts/release.sh`.

Bash style: use `local` only inside functions, quote all variable expansions, use `$()` not backticks, prefer `[[ ]]` over `[ ]`. Match the project's "minimum code that solves the problem" rule (CLAUDE.md §2) — no flag-parsing library, no colored output, no dry-run mode unless trivially cheap to add.
  </action>
  <verify>
    <automated>bash -n scripts/release.sh && test -x scripts/release.sh && grep -q '^version' Cargo.toml</automated>
  </verify>
  <done>
- `scripts/release.sh` exists, is executable, and passes `bash -n` syntax check
- Script handles all error paths listed in <action> step 2 with non-zero exit codes
- Script extracts version from `[package]` section only (not from `[dev-dependencies]`)
- Script computes correct new version for both patch and minor bumps
- Script updates Cargo.toml, commits, and creates an annotated `v{version}` tag
- Script prints summary and does NOT push to remote
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 2: Smoke-test release.sh in a throwaway repo</name>
  <what-built>
Pure-bash `scripts/release.sh` that bumps `Cargo.toml` version (patch or minor) and creates a `v{version}` git tag in one step.
  </what-built>
  <how-to-verify>
Run the script against a throwaway clone so the real repo state isn't touched:

```bash
# 1. Make a throwaway copy
TMP=$(mktemp -d)
git clone "$(pwd)" "$TMP/test-repo"
cd "$TMP/test-repo"

# 2. Confirm starting version
grep -A1 '^\[package\]' Cargo.toml | grep '^version'
# Expect: version = "0.1.0"

# 3. Patch bump
./scripts/release.sh
# Answer prompt with: 1 (or just press Enter)
# Expect summary showing 0.1.0 -> 0.1.1, commit, tag v0.1.1

# Verify
grep -A1 '^\[package\]' Cargo.toml | grep '^version'   # version = "0.1.1"
git tag --list | grep v0.1.1                            # v0.1.1
git log -1 --pretty=%s                                  # chore: release v0.1.1

# 4. Minor bump on top
./scripts/release.sh
# Answer prompt with: 2
# Expect summary showing 0.1.1 -> 0.2.0, tag v0.2.0

grep -A1 '^\[package\]' Cargo.toml | grep '^version'   # version = "0.2.0"
git tag --list | grep v0.2.0                            # v0.2.0

# 5. Dirty-tree guard
echo "x" >> README.md
./scripts/release.sh
# Expect non-zero exit + "working tree" error; no new tag created
git tag --list | wc -l                                  # still 2

# 6. Duplicate-tag guard
git checkout -- README.md
./scripts/release.sh
# Answer with: 1 (would try v0.2.1) — should succeed
./scripts/release.sh
# Without committing the bump above? Actually, after success, working tree is clean again.
# Try with same target by manually creating a conflicting tag first:
git tag v0.2.2
./scripts/release.sh
# Answer with: 1 (target v0.2.1 already moved on; this depends on current state)
# Primary check: a duplicate target must fail cleanly with exit 1.

# 7. Verify [dev-dependencies] git2 version was NOT mangled
grep -B1 'version = "0.21"' Cargo.toml
# Should still show the git2 dev-dependency line intact.

# 8. Cleanup
cd - && rm -rf "$TMP"
```

Confirm:
- [ ] Patch bump: 0.1.0 -> 0.1.1, tag v0.1.1 created, commit message correct
- [ ] Minor bump: 0.1.1 -> 0.2.0, tag v0.2.0 created
- [ ] Dirty working tree blocks the run
- [ ] Duplicate tag blocks the run
- [ ] Only `[package]` version changes — `git2` dev-dep version is untouched
- [ ] Script never pushes to a remote
  </how-to-verify>
  <resume-signal>Type "approved" or describe issues</resume-signal>
</task>

</tasks>

<verification>
Phase-level checks (run from repo root):

```bash
# Syntax
bash -n scripts/release.sh

# Executable bit
test -x scripts/release.sh && echo "executable"

# No accidental pushes wired in
! grep -E 'git[[:space:]]+push' scripts/release.sh && echo "no push command present"

# Section-aware update (must reference [package] context, not blind sed on first version=)
grep -q 'package' scripts/release.sh && echo "section-aware"
```
</verification>

<success_criteria>
- `scripts/release.sh` exists, is executable, passes `bash -n`
- Script never invokes `git push`
- Manual smoke test in a throwaway clone (Task 2) passes all checklist items
- The `[dev-dependencies]` `git2` version line is provably untouched after a bump
</success_criteria>

<output>
Create `.planning/quick/260520-rbp-add-a-script-to-update-the-tag-for-relea/260520-rbp-SUMMARY.md` when done.
</output>
