# Feature Landscape

**Domain:** Git history rewriting tool (author rename + co-author drop)
**Researched:** 2026-05-20
**Scope:** Covers only the two operations this tool does — rename primary author, drop co-author trailer

---

## Prerequisite: Mailmap vs. Rewrite

Before describing features, the tool must address why a user reaches for history rewriting instead of `.mailmap`. This is the single most important anti-feature risk: solving a display problem when `.mailmap` would suffice.

**When `.mailmap` is the right answer (tool should say so):**
- User only wants `git log`, `git shortlog`, and GitHub to display a different name
- The repo has collaborators who cannot all be coordinated for a fresh clone
- Audit/compliance requirements do not demand actual object changes

**When history rewriting is necessary (tool's domain):**
- The actual commit objects must be changed (e.g., old personal email must be scrubbed from commit objects, not just display)
- A `Co-authored-by` trailer must be removed entirely (`.mailmap` has no concept of trailers)
- The repo is solo or small-team and force-push coordination is feasible

The tool should not try to teach this in the TUI, but documentation and the README must state it clearly so users arrive having already made the right choice.

---

## Table Stakes

Features users expect from any git history rewriting tool. Missing = product feels incomplete or broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Rewrite all branches, not just current | Competing tools (filter-branch, filter-repo) all do this; partial rewrites leave ghost author discoverable in other branches | Medium | All refs under `refs/heads/` must be traversed and updated. git2 requires walking each branch's history and rebasing from the common ancestor forward. |
| Show count of commits affected before acting | Every destructive operation in this space shows impact before executing. Users are scared of history rewriting. | Low | Count is a read-only pass before the write pass. |
| Confirm before rewriting | Table stakes for any destructive operation. Ubiquitous in filter-branch, filter-repo, BFG. | Low | Single keypress Y/n prompt. |
| Post-rewrite force-push reminder with exact command | After rewrite, SHAs are new. Every tutorial, tool, and blog post about history rewriting ends with "now force push." Users will forget. | Low | Display `git push --force-with-lease --all` (force-with-lease is safer than --force; warn that plain --force exists but is more dangerous). |
| Correct display of Name + Email pair as the author identity unit | Users who know git know that name and email are distinct fields. Showing only one of them causes false matches (two people named "Alex"). | Low | Display as `Name <email>` everywhere. |
| Handle repos with large commit counts without hanging | A 10-year-old repo may have 50,000+ commits. The read pass (collecting all authors) must complete in reasonable time. | Medium | Use git2's walk API efficiently; display progress indicator during scan. |
| Graceful error if not in a git repository | Universally expected. Cryptic panics are unacceptable. | Low | Check repo detection at startup; print a clear message and exit. |
| Preserve all other commit metadata | Tree, parents, timestamps, commit message body, other trailers (`Signed-off-by`, `Reviewed-by`, `Fixes`, etc.) must be byte-identical except for what is being changed. | Medium | Any inadvertent normalization (e.g., stripping trailing newlines from messages) would corrupt commits. |

---

## Table Stakes: Author Rewrite Operation

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| List all distinct primary authors (Name+Email pairs) found in repo | The entry point for the operation. Without this, user cannot select who to rename. | Medium | Requires walking all commits on all branches and deduplicating by exact (name, email) pair. |
| Rename author's name and email atomically | Both fields change together. Changing only name leaves the old email pointing at the new name, which confuses tools like `git shortlog`. | Low | Rewrite both `author.name` and `author.email` in the same pass. |
| Match by exact Name+Email pair, not name-only or email-only | Prevents accidental merging of distinct people who share a name or share an email (the latter is uncommon but possible with team/bot accounts). | Low | Decision already recorded in PROJECT.md. |
| Rewrite committer identity when it matches the old author identity | git stores both author and committer. For most commits they match. If only author is changed, the committer field still exposes the old identity. filter-branch rewrites both by default. Users will notice and be confused. | Low | When performing the rewrite, check both author and committer fields. If committer matches the old identity, change it too. This is the correct default; do not offer a "only author" toggle for v1. |
| Show commit count for the selected source identity before confirming | Users need to know the blast radius before confirming. "Rewrite 847 commits?" is more informative than "Rewrite commits?". | Low | Computed during the selection step. |
| Accept new target author as free-text input | The target author may not exist in the repo yet (e.g., the correct corporate email was never used). Must support free-text entry. | Low | Text input field for name, separate field for email. Do not force picking from existing authors — that would prevent legitimate corrections. |

---

## Table Stakes: Co-Author Drop Operation

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| List all distinct `Co-authored-by` trailer entries found across all commits | Entry point for the drop operation. | Medium | Walk all commits on all branches; parse commit message body for trailer blocks. |
| Match trailer entries case-insensitively on the key | `git interpret-trailers` spec: trailer keys are case-insensitive. `Co-authored-by`, `Co-Authored-By`, and `co-authored-by` are the same key. | Low | Normalize key to lowercase for matching; preserve original casing of non-matching trailers. |
| Remove only the matching trailer line(s); preserve all others | Other trailers (`Signed-off-by`, `Reviewed-by`, `Fixes`, etc.) must survive the rewrite. Deleting the whole trailer block would corrupt commit messages. | Medium | Must parse the trailer block carefully and reconstruct the message with the target trailer removed. |
| Handle duplicate trailer occurrence in a single commit | A commit can have the same `Co-authored-by` trailer appear twice (e.g., tooling bug, manual edit). Remove all occurrences of the matching entry. | Low | Loop over all trailer lines in the block, remove all that match. |
| Normalize whitespace variants when matching trailer value | A user might have entered `Co-authored-by:  Name <email>` (two spaces), or trailing whitespace in the email. The tool must match these as the same entry when building the list. | Low | Trim key and value when comparing; use normalized form for the selector display. |
| Show count of commits affected before confirming | Same rationale as author rewrite. | Low | |

---

## Differentiators

Features that make this tool genuinely better for its narrow scope than general-purpose alternatives.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Zero dependencies to run | filter-repo requires Python 3. BFG requires Java. This tool: download and run. | Low (build-time complexity, not feature complexity) | Already decided. Static linking via git2 with libgit2 bundled. Primary user-facing differentiator. |
| Single binary distribution via curl | Reduces time-to-first-rewrite from "20 minutes installing Python+pip+filter-repo" to "30 seconds." | Low (CI complexity) | Already decided. |
| TUI author selector showing commit count per identity | Generic tools show no count. Seeing "alex@old.com (847 commits)" vs "alex@old.com (3 commits)" helps users catch typos or bot accounts before the wrong one is selected. | Low | Counts are naturally available from the scan pass. |
| Warn explicitly about GPG/SSH signed commits before rewriting | filter-repo strips signatures silently. filter-branch corrupts them silently. This tool should detect signed commits in the affected set and warn the user before confirming: "N commits have signatures that will be invalidated by this rewrite." Not a blocker — user still confirms and proceeds — but the warning prevents surprise. | Medium | Requires checking commit signature field via git2. Signatures on annotated tags should also be checked. |
| Warn about tags pointing at rewritten commits | After rewriting, any tag pointing at a rewritten commit SHA will be orphaned (it still points at the old object). filter-repo handles this automatically; this tool should warn the user to re-tag or that their tags need manual attention. | Medium | Detect tags pointing at commits in the rewrite set; emit warning after rewrite. |
| Clear post-rewrite collaborator impact statement | After rewriting, show: "Anyone who has cloned this repo must run: git fetch --all && git reset --hard origin/BRANCH". Generic tools do not provide this. | Low | Static text in post-rewrite output; branch names are available from the walk. |

---

## Anti-Features

Features that look useful but would cause confusion, data loss, or scope creep. Deliberately exclude these.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Backup refs (`refs/original/`) | filter-branch's approach. Creates false sense of safety — the refs keep old objects reachable but are not a real backup. Users forget to clean them up, refs are left dangling, and `git gc` must be run manually. The warn+confirm prompt is the safety gate. | Warn + confirm; document that reflog provides ~90 days of local recovery. |
| `--dry-run` flag | Adds code complexity and is redundant with the confirm prompt. The confirm prompt already shows the commit count and acts as a preview gate. A dry-run that doesn't write any commits adds another mode to maintain. | Keep confirmation prompt; show count and identity being changed clearly so user has full information before confirming. |
| Undo / rollback after rewrite | Requires storing old refs, adds significant complexity, and could give false confidence. Reflog already provides recovery for 90 days locally. | Document reflog recovery in post-rewrite output: "To undo locally: git reflog && git reset --hard <old-sha>". |
| Path argument (run on non-CWD repo) | Every user of this tool runs it in their repo. Adding --path or -C adds a flag surface that needs testing and documentation. | Always operate on CWD. Document this clearly. |
| Windows support in v1 | PowerShell download mechanism is a different UX. Cross-compilation adds CI complexity before the core product is validated. | Scope to Linux x86_64 + macOS aarch64/x86_64 for v1. |
| Auto force-push after rewrite | Would require shelling out to git binary, breaking the no-external-tools constraint. Also: if the push fails mid-way, state is ambiguous. | Show force-push command; let user run it. |
| Interactive rebase / commit squashing / message editing | Out of scope entirely. This is not a general git rewriting tool. Adding features beyond author rename and co-author drop turns it into a half-built filter-repo clone. | Non-goal. Redirect users to filter-repo for these use cases. |
| Rename co-author (vs. drop) | Tempting but distinct workflow. Would require selecting source co-author, entering target name+email, and handling all the matching complexity twice. Scope creep for v1. | Drop is the v1 operation. Rename can be achieved by drop + manually amending if needed. |
| `.mailmap` generation as output | Some tools suggest generating a mailmap from the rewrite plan instead of rewriting. Creates confusion about what the tool actually does. | If user wants mailmap, point them to `gitmailmap` documentation. This tool rewrites. |
| Selective branch rewriting | "Rewrite only main, not feature/xyz". Adds a branch-picker step and inconsistency: old author remains on skipped branches. | Always rewrite all branches. Document this explicitly. |
| Merge commit de-duplication / squashing | Merge commits that rewrite parent SHAs will have new SHAs. This is correct. Do not attempt to collapse or squash merge commits. | Merge commits are rewritten like any other commit; their tree and parent structure is preserved. |

---

## Feature Dependencies

```
Repo detection
  → All other features (nothing works without a valid repo)

Commit walk (all branches)
  → Author list collection
  → Co-author list collection
  → Commit count per identity
  → Signed commit detection (differentiator)
  → Tag impact detection (differentiator)

Author list collection
  → Author selector (Rename operation entry point)

Author selector (pick source)
  → Target author input (free text)
  → Commit count display (available from list collection)
    → Confirmation prompt
      → Rewrite pass (author + committer fields)
        → Post-rewrite output (force-push reminder, collaborator warning, tag warning)

Co-author list collection
  → Co-author selector (Drop operation entry point)
    → Commit count display
      → Confirmation prompt
        → Rewrite pass (trailer removal)
          → Post-rewrite output
```

---

## Open Questions (Not Resolved by Research)

These are design decisions not yet made in PROJECT.md that affect feature complexity:

1. **Author vs. Committer independence**: For the Rename operation, the default behavior should be to rewrite committer when it matches the old author (rationale above). But should the tool also handle the case where a user wants to rename a committer that never appears as an author? Recommendation: match and rename both fields whenever either matches the old identity. This covers all cases without a separate UI toggle.

2. **Target author input UX**: PROJECT.md says "second selector for target author" but the target may not exist in the repo. The UX should be a free-text input form (two fields: new name, new email), not a second list picker. This is architecturally different from a second list.

3. **Sort order for author/co-author lists**: Research found no established convention for TUI author lists. Recommendation: sort by commit count descending. The identity being renamed is usually the one with the most commits (the developer themselves). Showing it first reduces scrolling.

4. **Email matching for co-author deduplication in the list**: RFC 5321 says email local-parts are case-sensitive in theory but case-insensitive in practice. GitHub treats emails as case-insensitive for contribution attribution. Recommendation: normalize to lowercase for deduplication in the co-author list display; show the most common casing variant as the display string. Match case-insensitively during the actual trailer removal pass.

5. **Annotated tags on rewritten commits**: filter-repo explicitly converts signed tags to unsigned annotated tags. This tool should warn but not auto-rewrite tags. Rewriting tags requires a separate decision about re-tagging (tag name, message, tagger). Scope this as a warning only in v1.

6. **Trailer block position edge cases**: git's trailer spec requires trailers to be at the end of the commit message body, preceded by a blank line. Some commits have non-standard message structures where trailers appear mid-body. The tool should only parse trailers from the trailing block (contiguous lines at end of body per `git interpret-trailers` spec). Mid-body `Co-authored-by` mentions should be left as body text, not removed.

---

## MVP Recommendation

Build in this order, as each layer depends on the previous:

1. **Repo detection + commit walk** — foundation for both operations
2. **Author list collection + rename operation** — higher frequency use case; user more likely to have tested rename before drop
3. **Co-author list collection + drop operation** — secondary but fully specified
4. **Signed commit + tag impact warnings** — safety differentiators; add after core operations work

Defer to post-v1:
- Rename co-author (vs. drop): significant additional UX complexity, low validation
- Any additional git operation types: non-goal by design

---

## Sources

- git filter-branch deprecation and safety pitfalls: https://git-scm.com/docs/git-filter-branch
- git filter-repo fresh-clone requirement, tag handling, signature stripping: https://manpages.ubuntu.com/manpages/jammy/man1/git-filter-repo.1.html
- git filter-repo --mailmap rewrites author, committer, and tagger: https://github.com/newren/git-filter-repo/blob/main/Documentation/git-filter-repo.txt
- git interpret-trailers case-insensitivity and whitespace rules: https://git-scm.com/docs/git-interpret-trailers
- git mailmap limitations (display-layer only, does not modify objects): https://git-scm.com/docs/gitmailmap
- mailmap vs. rewrite decision guide: https://ntietz.com/blog/git-mailmap-for-name-changes/
- git2-rs has no filter-branch equivalent (closed issue): https://github.com/rust-lang/git2-rs/issues/821
- GPG signature invalidation on history rewrite: https://github.com/blog/changelog/2022-05-31-improved-verification-of-historic-git-commit-signatures/
- GitHub Co-authored-by format: https://docs.github.com/en/pull-requests/committing-changes-to-your-project/creating-and-editing-commits/creating-a-commit-with-multiple-authors
- BFG Repo-Cleaner features and limitations: https://rtyley.github.io/bfg-repo-cleaner/
- force-with-lease vs force push safety: https://dev.to/vast-cow/rewriting-remote-git-history-with-reset-and-push-force-with-lease-407a
- ratatui fuzzy finder community discussion: https://forum.ratatui.rs/t/vote-to-get-a-fuzzy-finder-widget/198
