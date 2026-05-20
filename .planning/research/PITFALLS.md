# Domain Pitfalls

**Domain:** Rust TUI git history rewriting tool (git2 + ratatui)
**Researched:** 2026-05-20
**Confidence:** HIGH for git2/libgit2 mechanics and ratatui panic handling; MEDIUM for macOS signing evolution (Apple keeps tightening this)

---

## Critical Pitfalls

Mistakes that cause data loss, repo corruption, or non-functional distribution binaries.

---

### Pitfall 1: Annotated Tags Point at the Old Commit SHA After Rewrite

**What goes wrong:**
After rewriting all branch tips to new SHAs, annotated tags still point at the original commit objects. Lightweight tags are refs that point directly at a commit object — `git2::Repository::reference()` with `force: true` updates them correctly. Annotated tags are different: they are *objects* (type `tag`) that embed the target OID. Updating the ref `refs/tags/v1.0` points the tag ref at the new commit but does not update the tag object itself; the embedded OID inside the tag object still points at the pre-rewrite commit. `git describe`, `git log --tags`, and any tool that dereferences the tag object will surface the old, reachable-but-orphaned commit.

**Why it happens:**
Developers iterate `repo.references_glob("refs/tags/*")` and call `ref.set_target(new_oid)` or re-create via `repo.reference()`. This works for lightweight tags but silently fails to update the embedded OID inside an annotated tag object.

**Consequences:**
- The annotated tag object object remains in the repo forever (unreachable from branches, reachable from the tag ref's object)
- Tools like `git describe` return the annotated tag name but the embedded tagger metadata (date, message) still references the old history
- If the user force-pushes, the remote will also have this split state

**Prevention:**
For each tag ref, check if it's annotated: `repo.find_tag(ref.target())` succeeds for annotated tags, fails for lightweight. For annotated tags, create a new tag object via `repo.tag(name, new_target_commit, tagger, message, true)` (the `force` flag replaces the existing ref), then delete the old tag object if it is now unreferenced. Walk `refs/tags/*` explicitly after all commits are rewritten.

**Detection (warning signs):**
`git tag -v <tagname>` shows "object" field pointing at an old SHA that no longer exists in `git log`. `git cat-file -p refs/tags/<name>` reveals the raw object type.

**Phase:** Commit rewriting core (whichever phase implements the actual graph walk and ref update). Must be in the same phase as branch ref updating, not a later phase. Treating tags as "polish" and deferring them is what causes the bug.

---

### Pitfall 2: Merge Commit Parent Order Corrupts Graph Topology

**What goes wrong:**
When rewriting a merge commit, you must pass the rewritten parents to `repo.commit()` in the *exact same order* as the original. libgit2's `git_commit_parent_id(commit, n)` returns the nth parent. If the rewrite loop collects parents into a `Vec` using iteration order that differs from index order (e.g. from a HashSet or parallel lookups keyed by old OID), the first parent and second parent can be swapped.

**Why it happens:**
The author/committer change is the goal; parent order feels like a detail. A `HashMap<OldOid, NewOid>` lookup is correct — but the parent *list* must be reconstructed using `commit.parent_ids()` (ordered) not by iterating the map.

**Consequences:**
- `git log --first-parent` traces the wrong branch
- Merge-base calculations change, breaking `git rebase` and `git cherry-pick` downstream
- `git log --graph` shows the topology flipped; cosmetically wrong and semantically wrong for bisect

**Prevention:**
Always reconstruct parents from `commit.parent_count()` and `commit.parent_id(i)` in index order (0..N), then map each through the old→new OID table. Never use a set or unordered structure for parent accumulation.

**Detection (warning signs):**
After rewrite, `git log --oneline --graph | head -40` looks "flipped" on merge commits. Add a post-rewrite assertion in tests: for any commit with `parent_count > 1`, verify `rewritten.parent_id(0)` matches the expected first parent.

**Phase:** Commit graph traversal and rewriting (core phase). Add a test with a repo that has at least one merge commit before shipping.

---

### Pitfall 3: Stash Entries Are Silently Broken After Rewrite

**What goes wrong:**
`refs/stash` and its reflog entries point at stash commits that are built on top of pre-rewrite commits. After rewriting all branches, the stash entries' parent commits are now orphaned (old SHAs, unreachable from any ref). The stash entries themselves are still ref-reachable (via `refs/stash`), so they survive GC, but `git stash pop` and `git stash apply` will fail or produce bizarre behavior because the three-way merge base (the WIP commit's first parent) no longer matches any branch HEAD.

**Why it happens:**
`refs/stash` is not a branch and is not in the refs that a branch-focused rewrite scans. It is easy to miss because `repo.branches()` and `repo.references_glob("refs/heads/*")` do not include it.

**Consequences:**
- User loses stashed work-in-progress (functionally, even though objects exist)
- `git stash list` shows entries; `git stash apply` fails with confusing conflict messages
- No error is surfaced during the rewrite — the corruption is silent

**Prevention:**
Before rewriting, check if `refs/stash` exists via `repo.find_reference("refs/stash")`. If it does, emit a prominent warning: "You have stashed changes. History rewriting will leave your stash entries in an inconsistent state. Pop or drop stashes before running this tool." Block on user confirmation, or offer to abort. Do NOT attempt to rewrite stash entries — the structure (index snapshot + untracked snapshot) is complex and not worth the implementation risk.

**Detection (warning signs):**
`repo.find_reference("refs/stash")` returns `Ok(_)` before starting the rewrite.

**Phase:** Pre-flight safety checks phase (the same phase that shows the "N commits will be rewritten" confirmation prompt).

---

### Pitfall 4: Worktree Branches Are Checked-Out Refs and Cannot Be Updated Normally

**What goes wrong:**
Git worktrees check out branches exclusively — a branch checked out in a worktree cannot be updated via `repo.find_reference("refs/heads/feature").set_target(new_oid)`. libgit2 will return an error because the ref is locked by the worktree. If the tool ignores this error and continues, that branch is not rewritten; it silently keeps the old history.

**Why it happens:**
The tool iterates `refs/heads/*` and tries to update each branch. Most succeed. A worktree-locked branch fails quietly if errors are swallowed.

**Consequences:**
- That branch retains old author data — the core feature fails for that branch
- The user thinks the rewrite succeeded; the branch in the worktree still has old commits

**Prevention:**
Before rewriting, call `repo.worktrees()` and check if any branch is currently checked out in a secondary worktree. If yes, warn: "Branch X is checked out in worktree at /path. Close or remove that worktree before running this tool." Treat this as a blocking condition, not a warning.

**Detection (warning signs):**
`repo.worktrees()` returns a non-empty list. Errors from `reference.set_target()` must be propagated, not ignored.

**Phase:** Pre-flight safety checks, alongside stash detection.

---

### Pitfall 5: libgit2 Static Linking on Linux — OpenSSL dlopen Symbols Break the Linker

**What goes wrong:**
When building with `vendored-libgit2` for a static Linux binary, the build links OpenSSL's `libcrypto.a` statically. `libcrypto.a` contains `dso_dlfcn.o`, which requires `dlopen`, `dlsym`, `dlclose`, `dladdr`, and `dlerror` — symbols from `libdl`. The linker does not automatically pull in `-ldl` as a transitive dependency of the static archive. The result is: `undefined reference to 'dlopen'` at link time.

**Why it happens:**
libgit2's CMake build system includes `${CMAKE_DL_LIBS}` for dynamic linking cases but the transitive `-ldl` requirement is not propagated through libgit2's `.pc` (pkg-config) file when everything is statically linked.

**Consequences:**
GitHub Actions CI build fails. The binary cannot be produced for Linux at all.

**Prevention:**
For the Linux x86_64 target, use `x86_64-unknown-linux-musl` (musl libc) rather than the GNU glibc target. The Rust musl target produces fully static binaries and the musl toolchain handles the `libdl` situation differently. Use the `cross` tool or `cargo-zigbuild` with a musl cross-compilation Docker image. Disable the `ssh` and `https` features on git2 entirely — this tool does not clone or fetch, so those features add OpenSSL/libssh2 complexity for zero benefit. In `Cargo.toml`: `git2 = { version = "...", default-features = false }` — confirm which features are enabled by default and strip SSH/HTTPS.

**Detection (warning signs):**
Linux CI build produces `undefined reference to 'dlopen'` or `undefined reference to 'dlsym'`. Also watch for: build succeeds but binary has `ldd` dependencies on `libssl.so` or `libcrypto.so` (means static linking did not work as intended).

**Phase:** CI/build infrastructure phase (the phase that sets up GitHub Actions cross-compilation). Must be solved before any release pipeline is considered working.

---

### Pitfall 6: ratatui Terminal Left in Raw Mode After Panic or SIGTERM

**What goes wrong:**
If the program panics mid-operation (e.g. an unwrap on a None during the commit graph walk, or an out-of-bounds access), the terminal is left in crossterm raw mode with the alternate screen still active. The user's shell session is broken — typed characters don't echo, Ctrl+C may not work, and the terminal appears frozen.

**Why it happens:**
ratatui's `ratatui::init()` enables raw mode and alternate screen. Without a registered panic hook, the panic unwinds Rust's stack, destructors run, but crossterm's raw mode is a kernel-level TTY setting that Rust destructors do not automatically reset. The panic message prints to the alternate screen and is immediately invisible when the alternate screen exits (or never exits at all if the destructor doesn't run).

SIGTERM (from `kill <pid>`) is even worse: it terminates the process without running destructors at all. The terminal stays broken until the user manually runs `reset`.

**Prevention:**
1. Use `ratatui::init()` (not `Terminal::new()` directly) — as of recent ratatui versions, `init()` automatically installs a panic hook that calls `ratatui::restore()` before panicking.
2. Install an explicit signal handler using `signal-hook` crate for SIGTERM and SIGINT that calls `ratatui::restore()` before exiting. In raw mode, Ctrl+C is intercepted by crossterm and does not generate SIGINT — handle it as a key event AND as a signal.
3. Wrap the top-level `main()` logic in a `std::panic::catch_unwind` block as a last resort fallback.

Canonical pattern:
```rust
let _terminal = ratatui::init();
install_signal_handlers(); // signal-hook: SIGTERM -> restore() + exit
let result = run_app(); // all app logic here
ratatui::restore();
result
```

**Detection (warning signs):**
Running the binary and pressing Ctrl+C (or sending SIGTERM) leaves the terminal broken — `stty sane` is required to recover. Test this explicitly: run the binary, kill it with `kill -TERM <pid>` from another terminal, verify the original terminal is usable.

**Phase:** TUI scaffolding phase (the first phase that introduces ratatui). Fix this before writing any application logic — retrofitting signal handling is painful.

---

## Moderate Pitfalls

---

### Pitfall 7: Co-authored-by Key Case Sensitivity — Wrong Assumption Causes Missed Matches

**What goes wrong:**
The git trailer spec explicitly states that trailer keys are case-insensitive for parsing. GitHub's UI writes `Co-authored-by:` (lowercase b, lowercase a). Other tools write `Co-Authored-By:` (title case). Still others write `co-authored-by:` (all lowercase). If the parser does a case-sensitive byte comparison, it misses variants.

Additionally, the spec requires the key to have *no whitespace before or inside it*, but multiple spaces/tabs between key and separator are valid. A regex that expects exactly one space before `:` will fail on `Co-authored-by  :`.

**Prevention:**
Normalize the key to lowercase before comparison. Use a pattern like: `key.trim().to_lowercase() == "co-authored-by"`. Do not use a hardcoded string literal for matching. Handle the multi-space-before-colon case by splitting on the first `:` and trimming the left portion.

**Detection (warning signs):**
Test corpus: create commits with `Co-Authored-By:`, `co-authored-by:`, and `CO-AUTHORED-BY:` variants. Verify all three appear in the "drop co-author" selector.

**Phase:** Co-author parsing logic phase.

---

### Pitfall 8: Trailer Block Detection — Non-Trailer Lines Break the Entire Block

**What goes wrong:**
Git's trailer detection requires the trailer block to be at the *end* of the commit message, preceded by a blank line, and composed of at least 25% trailer lines. If a commit message has a "footer" section with non-trailer free text followed by some `Co-authored-by:` lines, git's spec treats the entire section as not-a-trailer-block. A naive parser that scans lines backwards until it finds a non-`Key: Value` line would also include those non-trailer lines as false positives (matching a body paragraph that happens to start with `Word: rest`).

Conversely, a strict parser may reject valid trailers if the commit message has a blank line *inside* the trailer block (folded values or human error).

**Prevention:**
Parse trailers using the same heuristic as git: scan lines from the end of the message, collect lines matching `^[A-Za-z][A-Za-z-]*:[ \t]` (key-colon-whitespace pattern, no leading whitespace), stop when you hit a non-matching non-empty line. Treat the collected lines as the trailer block. For the purpose of *finding* co-authors (not modifying all trailers), a simpler line-by-line scan of the entire body with a case-insensitive regex is more robust and avoids the 25% rule problem.

**Detection (warning signs):**
Test with commit messages that have: (a) body paragraphs before trailers, (b) a "Fixes: #123" trailer mixed with Co-authored-by lines, (c) a blank line inside what looks like a trailer block.

**Phase:** Co-author parsing logic phase.

---

### Pitfall 9: Unicode Names and Emails in Co-authored-by Lines

**What goes wrong:**
The `Co-authored-by: 陈伟 <chen@example.com>` format is valid. A parser that assumes ASCII, uses byte indexing without Unicode awareness, or compares raw bytes when deduplicating names will either panic, truncate, or fail to match.

**Prevention:**
In Rust, `str` is always valid UTF-8. Use `str::find('<')` and `str::rfind('>')` to extract the email portion rather than byte-indexing. For the display name (before `<`), trim whitespace with `.trim()` which is Unicode-aware. For deduplication in the TUI selector, compare by the full `Name <email>` string using standard Rust string equality (which is byte-level on UTF-8, which is correct for identity matching).

**Detection (warning signs):**
Add a test commit with a non-ASCII name. Run the selector and verify the entry appears correctly.

**Phase:** Co-author parsing logic phase.

---

### Pitfall 10: Git Notes Are Silently Orphaned After Rewrite

**What goes wrong:**
`refs/notes/commits` stores notes keyed by the commit SHA they annotate. After rewriting commits to new SHAs, all notes become detached — they reference old SHAs that no longer exist as reachable commits. `git log --notes` shows nothing for the rewritten commits. The notes objects still exist in the object store (reachable from `refs/notes/commits`) but the SHA-based lookup returns nothing.

**Why it happens:**
Notes are stored as a tree where each blob's name is the commit SHA it annotates. Rewriting the commit changes the SHA; the notes tree is not updated.

**Consequences:**
Users who use `git notes` to annotate commits (code review notes, CI metadata) silently lose those annotations after the rewrite. No error; the tool just doesn't show them anymore.

**Prevention (for v1):**
This tool is explicitly not going to rewrite notes (it would require rewriting the notes tree too — a significant additional feature). Before rewriting, check if `refs/notes/commits` exists (`repo.find_reference("refs/notes/commits")`). If it does, warn: "This repository has git notes. They will not be updated and will appear detached after the rewrite. See `git notes` for details." This is a warn-not-block condition for v1.

**Detection (warning signs):**
`repo.find_reference("refs/notes/commits")` returns `Ok(_)`.

**Phase:** Pre-flight safety checks phase.

---

### Pitfall 11: macOS Code Signing — curl Downloads Bypass Gatekeeper, but Rust Cross-Compilation Requires Ad-Hoc Signing

**What goes wrong:**
Two distinct problems:

**Problem A — Gatekeeper (macOS 15+):** Since macOS Sequoia 15.1, unsigned applications are blocked and the "right-click > Open Anyway" bypass was removed. However, binaries downloaded via `curl` do NOT receive the `com.apple.quarantine` extended attribute, so Gatekeeper does not check them. The curl-to-run install pattern works precisely because of this quarantine bypass.

**Problem B — Apple Silicon execution requirement:** ARM64 binaries on Apple Silicon *must* be code-signed to execute, even with an ad-hoc signature. The Rust linker (`ld`) automatically produces an ad-hoc signature when building natively on macOS. However, when cross-compiling for `aarch64-apple-darwin` on a Linux GitHub Actions runner, the resulting binary may lack any signature, causing `exec format error` or Gatekeeper rejection on Apple Silicon Macs.

**Prevention:**
For macOS binary distribution:
1. Build `aarch64-apple-darwin` on a macOS GitHub Actions runner (`macos-latest`), not a Linux runner. macOS runners produce correctly signed (ad-hoc at minimum) ARM binaries.
2. If a Developer ID certificate is available (paid Apple Developer account, $99/year), sign and notarize — this is the gold standard. Without it, the curl bypass keeps things working for technically-inclined users, but is not appropriate for broad consumer distribution.
3. For developer tool audiences (the target user for git-author-reformer), the curl bypass is acceptable and widely used (rustup, Homebrew formulae, etc. use this pattern).

**Detection (warning signs):**
Building `aarch64-apple-darwin` on a Linux CI runner and downloading the binary to an M1 Mac produces: `cannot execute binary file: Exec format error` or silent crash. Test by actually running the binary on an Apple Silicon Mac after downloading via `curl`.

**Phase:** CI/distribution pipeline phase.

---

## Minor Pitfalls

---

### Pitfall 12: Force-Push Reminder Uses Wrong Branch Name

**What goes wrong:**
The post-rewrite screen shows a hardcoded `git push --force origin main`. If the repo's default branch is `master`, or the remote is named `upstream` instead of `origin`, the command is wrong. The user copy-pastes it, it fails, and they are confused.

**Prevention:**
Detect the actual remote name by iterating `repo.remotes()`. If only one remote exists, use it. Fall back to `origin` if multiple exist. For the branch portion, show the list of all branches that were rewritten, or instruct the user to push all: `git push --force origin --all`. Alternatively, show: `git push --force <remote> <branch1> <branch2> ...` for each rewritten branch.

**Detection (warning signs):**
Test on a repo where the remote is named `upstream` or where the main branch is `trunk`.

**Phase:** Post-rewrite UX phase.

---

### Pitfall 13: HEAD Is on the Rewritten Branch — Must Update Working Directory State

**What goes wrong:**
After rewriting the commit at the tip of the currently checked-out branch, `HEAD` still points at the branch ref, and the branch ref now points at the new commit. git2 does not automatically update the working directory's index. If the tool also needs to update the index (it shouldn't — it's only rewriting metadata, not trees), an explicit `repo.checkout_head()` call would be needed. For metadata-only rewrites (author, committer, message), the working directory content is unchanged, so this is not a problem in practice. However, if the code accidentally uses `repo.reset()` or `repo.checkout_head()` unnecessarily, it can wipe uncommitted changes.

**Prevention:**
For metadata-only rewrites (author/committer/message changes with identical trees), never call `checkout_head` or `reset`. The working directory is not touched. The branch ref update is sufficient. Validate this assumption: the tree OID of the new commit must equal the tree OID of the original commit.

**Detection (warning signs):**
Any call to `repo.checkout_head()` or `repo.reset()` in the commit rewriting code path is a red flag.

**Phase:** Commit rewriting core.

---

### Pitfall 14: curl Install Script Integrity — No Checksum = Silent MITM

**What goes wrong:**
The curl-to-run pattern (`curl -fsSL https://example.com/install.sh | bash`) downloads and executes in one pipe. If the connection is intercepted (MITM on a network without certificate pinning, or a CDN/GitHub compromise), the user runs attacker-controlled code. Even with HTTPS, a compromised binary without a separate checksum is undetectable.

**Prevention:**
The install script should:
1. Download the binary to a temp file first (not pipe directly to execution)
2. Download the corresponding `sha256sum` file separately (from a different path than the binary, ideally from a different GitHub release asset)
3. Verify the checksum before `chmod +x` and execution
4. Use `curl -fsSL --fail` to abort on HTTP errors (avoid partial-download execution)

Pattern:
```sh
curl -fsSL "$URL" -o /tmp/git-author-reformer
curl -fsSL "$URL.sha256" -o /tmp/git-author-reformer.sha256
sha256sum -c /tmp/git-author-reformer.sha256 || { echo "Checksum failed"; exit 1; }
chmod +x /tmp/git-author-reformer
exec /tmp/git-author-reformer "$@"
```

**Detection (warning signs):**
An install script that pipes `curl` output directly to `bash` or `sh` without saving and verifying. An install script without a checksum verification step.

**Phase:** Distribution/install script phase.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Commit graph traversal + ref updating | Annotated tag OIDs not updated (Pitfall 1) | Explicit tag type check; recreate annotated tags |
| Commit graph traversal + ref updating | Merge commit parent order corrupted (Pitfall 2) | Use indexed parent access, not set-based accumulation |
| Pre-flight safety checks UI | Stash entries silently broken (Pitfall 3) | Detect `refs/stash`, block with warning |
| Pre-flight safety checks UI | Worktree-locked branches skipped silently (Pitfall 4) | Detect worktrees, block with warning |
| Pre-flight safety checks UI | Git notes silently orphaned (Pitfall 10) | Detect `refs/notes/commits`, warn (non-blocking) |
| CI/build infrastructure | Linux static link OpenSSL dlopen failure (Pitfall 5) | musl target + disable ssh/https features |
| CI/build infrastructure | macOS ARM binary unsigned (Pitfall 11 - Problem B) | Build on macOS runner, not Linux cross-compile |
| TUI scaffolding | Terminal stuck in raw mode on panic/SIGTERM (Pitfall 6) | `ratatui::init()` + `signal-hook` for SIGTERM |
| Co-author parsing | Case-insensitive key matching (Pitfall 7) | `key.trim().to_lowercase()` comparison |
| Co-author parsing | Trailer block detection false positives (Pitfall 8) | Line-by-line scan with case-insensitive regex |
| Co-author parsing | Unicode names/emails (Pitfall 9) | `str::find('<')` / `str::rfind('>')`, not byte indexing |
| Post-rewrite UX | Wrong remote/branch in force-push command (Pitfall 12) | Detect actual remote name from repo |
| Distribution/install script | No checksum verification (Pitfall 14) | Download-then-verify pattern, separate sha256 asset |

---

## Sources

- [git-interpret-trailers documentation](https://git-scm.com/docs/git-interpret-trailers) — trailer key case-insensitivity, separator rules
- [ratatui Panic Hooks recipe](https://ratatui.rs/recipes/apps/panic-hooks/) — terminal restoration on panic
- [ratatui docs: restore()](https://docs.rs/ratatui/latest/ratatui/fn.restore.html) — what restore() does and when to call it
- [libgit2 issue #6632: statically linking libgit2 results in linker error](https://github.com/libgit2/libgit2/issues/6632) — dlopen undefined reference
- [git2-rs README](https://github.com/rust-lang/git2-rs) — vendored-libgit2 feature, LIBGIT2_NO_VENDOR env var
- [git-filter-repo issue #115: stashes deleted during rewrite](https://github.com/newren/git-filter-repo/issues/115) — stash data loss during history rewrite
- [git-filter-branch documentation](https://git-scm.com/docs/git-filter-branch) — annotated tag --tag-name-filter requirement
- [Distributing Mac apps without notarization](https://lapcatsoftware.com/articles/without-notarization.html) — curl quarantine bypass
- [Apple Silicon Macs will require signed code](https://eclecticlight.co/2020/08/22/apple-silicon-macs-will-require-signed-code/) — ad-hoc signing requirement for ARM
- [macOS 15.1 blocks unsigned applications](https://forums.macrumors.com/threads/macos-15-1-completely-removes-ability-to-launch-unsigned-applications.2441792/) — Sequoia Gatekeeper tightening
- [Crossterm SIGTERM signal handling](https://dev.to/plecos/handling-ctrl-c-while-using-crossterm-1kil) — Ctrl+C raw mode interception
- [curl pipe bash security discussion](https://www.arp242.net/curl-to-sh.html) — MITM risk and checksum mitigation
