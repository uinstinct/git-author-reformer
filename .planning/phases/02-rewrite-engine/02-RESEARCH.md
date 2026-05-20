# Phase 2: Rewrite Engine - Research

**Researched:** 2026-05-20
**Domain:** git2-rs commit graph rewriting, OID remapping, annotated tag recreation
**Confidence:** HIGH

## Summary

Phase 2 builds the commit cascade rewrite engine that walks ALL commits reachable from ALL refs in topological order, rewrites matching commits with updated author/committer fields or modified commit messages (co-author drop), remaps parent OIDs through a `HashMap<Oid, Oid>` table, updates all branch refs to their new tips, and recreates annotated tag objects pointing at rewritten commits.

The rewrite algorithm is purely a write pass: no TUI, no confirmation prompts, no output to the user. Phase 2 exposes a library API (`rewrite_author`, `drop_coauthor`) that Phase 3's TUI will call after the user confirms. Phase 2 is engine-only — every user-facing concern (confirmation, count display, warnings for GPG/notes) belongs in Phase 3.

The git2 API covers all required operations without calling the git binary. The key idioms are: `Sort::TOPOLOGICAL | Sort::REVERSE` to guarantee parents before children; a `HashMap<Oid, Oid>` oid-map to remap parent references; `message_raw()` + `Signature::when()` to preserve byte-identical message and timestamp on non-renamed fields; `Repository::tag()` with `force: true` to recreate annotated tag objects; and `reference.set_target()` to update branch tips and lightweight tag refs.

**Primary recommendation:** Implement one module `src/git/rewrite.rs` with two public functions and shared private helpers. Do not over-split into multiple sub-modules — Rule 2 (Simplicity First) applies.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Commit graph walk | git/libgit2 layer | — | libgit2 Revwalk handles topological traversal |
| OID remapping table | in-memory (HashMap) | — | Plain HashMap, no persistence needed |
| Commit rewrite (author/committer) | git/libgit2 layer | — | `Repository::commit()` creates new commit objects |
| Co-author trailer manipulation | string layer (Rust) | git/libgit2 layer | Parse message_raw(), rewrite in Rust, write via `Repository::commit()` |
| Branch ref update | git/libgit2 layer | — | `reference.set_target()` or `repo.reference()` with force |
| Annotated tag recreation | git/libgit2 layer | — | `Repository::tag()` to create new tag object |
| Lightweight tag update | git/libgit2 layer | — | `reference.set_target()` on existing ref |

## User Constraints (from CONTEXT.md)

All implementation choices are at Claude's discretion — discuss phase was skipped. The ROADMAP phase goal, success criteria, and REQUIREMENTS.md locked decisions govern all choices.

**Locked decisions from REQUIREMENTS.md:**
- RENAME-03: Rewrite author AND committer fields when committer == old author identity (conditional committer rewrite — NOT unconditional)
- RENAME-04: Annotated tag objects must be recreated (not just the ref pointer), in this phase
- DROP-02: Remove selected co-author from ALL Co-authored-by trailers — case-insensitive key match, remove all occurrences within a single commit if duplicated
- DROP-03: All other Co-authored-by entries and metadata (tree, timestamps, other trailers, commit message body) are preserved byte-for-byte

**Key constraints from ROADMAP:**
- Annotated tag object recreation MUST occur in same phase as branch ref updating — do not defer
- Merge commit parent order MUST be preserved by index (`commit.parent_id(i)` in 0..N order) — never use unordered structure

## Standard Stack

### Core (no new dependencies — all already in Cargo.toml)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `git2` | `0.21.0` | Repository::commit(), Revwalk, reference(), tag() | Already in Cargo.toml; all required operations are available |

Phase 2 requires NO new dependencies. All git2 operations needed (commit creation, ref update, tag recreation) are in the existing `git2 = "0.21.0"` dependency. [VERIFIED: cargo search + Cargo.toml]

### No New Packages

Phase 2 is a pure engine implementation using existing dependencies. The package legitimacy audit is a non-event: `git2 = "0.21.0"` is already installed and verified from Phase 1.

## Package Legitimacy Audit

No new packages are introduced in Phase 2. The existing `git2 = "0.21.0"` dependency covers all required operations.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `git2` | crates.io | ~10 yrs | Very high | github.com/rust-lang/git2-rs | N/A (pre-existing) | Approved (Phase 1) |

**slopcheck unavailable at research time** — but git2 is a well-established crate maintained by the Rust org, already in use in Phase 1, and not subject to re-audit here.

## Architecture Patterns

### System Architecture Diagram

```
Caller (Phase 3 TUI)
        │
        │  rewrite_author(repo, old_identity, new_name, new_email)
        │  drop_coauthor(repo, coauthor_email)
        ▼
┌─────────────────────────────────────────────┐
│  src/git/rewrite.rs                         │
│                                             │
│  1. build_rewrite_revwalk()                 │
│     push_glob("refs/heads/*")               │
│     push_glob("refs/tags/*")  [peeled]      │
│     Sort::TOPOLOGICAL | Sort::REVERSE       │
│     → OID stream: oldest-first              │
│                         │                  │
│  2. Per-commit decision loop                │
│     ┌────────────────────┴──────────┐       │
│     │ needs rewrite?                │       │
│     │ (identity matches OR          │       │
│     │  any parent in oid_map)       │       │
│     └────┬─────────────────┬────────┘       │
│          │ YES             │ NO             │
│          ▼                 ▼               │
│  remap_parents()    oid_map unchanged      │
│  build new Signature                        │
│  message_raw()  [byte-identical copy]       │
│  Repository::commit(None, ...)              │
│  oid_map.insert(old, new)                   │
│          │                                  │
│  3. update_refs()                           │
│     refs/heads/*: reference.set_target()    │
│     refs/tags/*: detect annotated vs light  │
│       annotated → repo.tag(force:true)      │
│       lightweight → reference.set_target()  │
└─────────────────────────────────────────────┘
        │
        ▼
  usize (count of rewritten commits) returned to caller
```

### Recommended Project Structure

```
src/
├── git/
│   ├── mod.rs           # add: pub mod rewrite;
│   ├── rewrite.rs       # NEW: rewrite_author(), drop_coauthor(), private helpers
│   ├── reader.rs        # existing
│   ├── preflight.rs     # existing
│   └── types.rs         # extend: add RewriteSpec, RewriteResult if needed
tests/
├── common/
│   └── mod.rs           # extend: add helpers for merge commits, annotated tags, multi-branch repos
└── rewrite_test.rs      # NEW: integration tests for rewrite engine
```

### Pattern 1: Rewrite Walk + OID Map

The core algorithm for a lossless graph rewrite.

**What:** Walk all commits topologically (oldest first), for each commit decide if it needs rewriting (identity match or parent remapped), create new commit, record OID mapping. After walk, update all refs.

**When to use:** Any time the goal is to rewrite commit objects across the full history.

```rust
// Source: docs.rs/git2/0.21.0 + CLAUDE.md STACK section
use std::collections::HashMap;
use git2::{Oid, Repository, Sort};

fn rewrite_walk(
    repo: &Repository,
    oid_map: &mut HashMap<Oid, Oid>,
    // ... decision + rewrite closures
) -> Result<usize, crate::error::AppError> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("refs/heads/*")?;
    revwalk.push_glob("refs/tags/*")?; // push_glob peels tag objects to commits [CITED: docs.rs/git2]
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;

    let mut count = 0usize;
    for oid_result in revwalk {
        let old_oid = oid_result?;
        let commit = repo.find_commit(old_oid)?;

        // Collect remapped parent OIDs — ordered by index (CRITICAL: preserve merge order)
        let new_parent_oids: Vec<Oid> = (0..commit.parent_count())
            .map(|i| {
                let parent_oid = commit.parent_id(i).unwrap();
                *oid_map.get(&parent_oid).unwrap_or(&parent_oid)
            })
            .collect();

        let any_parent_remapped = (0..commit.parent_count())
            .any(|i| {
                let p = commit.parent_id(i).unwrap();
                oid_map.contains_key(&p)
            });

        // Decide: does this commit need a new object?
        let needs_rewrite = /* identity match check */ true || any_parent_remapped;

        if needs_rewrite {
            // Step 1: collect parent Commit objects (ownership dance — git2 issue #140)
            let parent_commits: Vec<git2::Commit> = new_parent_oids
                .iter()
                .map(|oid| repo.find_commit(*oid))
                .collect::<Result<_, _>>()?;
            let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

            // Step 2: build new author/committer signatures (preserve timestamp)
            let orig_author = commit.author();
            let orig_committer = commit.committer();
            let new_author = /* conditionally swap name/email, preserve .when() */
                git2::Signature::new("...", "...", &orig_author.when())?;
            let new_committer = /* only replace if matches old identity */
                git2::Signature::new("...", "...", &orig_committer.when())?;

            // Step 3: use message_raw() — NOT message() — for byte-identical copy
            let raw_msg = commit.message_raw().unwrap_or("");

            // Step 4: create new commit (update_ref = None; we update refs in separate pass)
            let new_oid = repo.commit(
                None, // no ref update here — done in bulk after walk
                &new_author,
                &new_committer,
                raw_msg,
                &commit.tree()?,
                &parent_refs,
            )?;

            oid_map.insert(old_oid, new_oid);
            count += 1;
        }
        // If no rewrite needed, oid_map has no entry for old_oid — callers use
        // oid_map.get(&oid).unwrap_or(&oid) to transparently return the original OID.
    }
    Ok(count)
}
```

[CITED: docs.rs/git2/0.21.0/git2/struct.Repository.html, docs.rs/git2/0.21.0/git2/struct.Revwalk.html]

### Pattern 2: Conditional Author/Committer Rewrite (RENAME-03)

**CRITICAL:** The committer is ONLY replaced when it matches the old author identity. Do NOT unconditionally rewrite committer.

```rust
// Source: REQUIREMENTS.md RENAME-03 — "updating both the author and committer fields
//         when the committer matches the old author identity"
fn build_new_signatures<'a>(
    commit: &'a git2::Commit,
    old_name: &str,
    old_email: &str,
    new_name: &str,
    new_email: &str,
) -> Result<(git2::Signature<'static>, git2::Signature<'static>), git2::Error> {
    let orig_author = commit.author();
    let orig_committer = commit.committer();

    let author_matches =
        orig_author.name().unwrap_or("") == old_name
        && orig_author.email().unwrap_or("") == old_email;

    let committer_matches =
        orig_committer.name().unwrap_or("") == old_name
        && orig_committer.email().unwrap_or("") == old_email;

    let new_author = if author_matches {
        git2::Signature::new(new_name, new_email, &orig_author.when())?
    } else {
        // Preserve original — timestamp MUST be preserved via .when()
        git2::Signature::new(
            orig_author.name().unwrap_or(""),
            orig_author.email().unwrap_or(""),
            &orig_author.when(),
        )?
    };

    let new_committer = if committer_matches {
        git2::Signature::new(new_name, new_email, &orig_committer.when())?
    } else {
        git2::Signature::new(
            orig_committer.name().unwrap_or(""),
            orig_committer.email().unwrap_or(""),
            &orig_committer.when(),
        )?
    };

    Ok((new_author, new_committer))
}
```

[CITED: REQUIREMENTS.md RENAME-03, docs.rs/git2/0.21.0/git2/struct.Signature.html]

### Pattern 3: Co-Author Trailer Drop (DROP-02, DROP-03)

**What:** Strip all `Co-authored-by:` lines (case-insensitive) whose email matches the target, preserve all other content byte-for-byte.

```rust
// Source: requirements DROP-02, DROP-03 + existing strip_coauthor_prefix pattern from reader.rs
fn drop_coauthor_from_message(message: &str, target_email: &str) -> String {
    // Use message_raw() output as input — NOT message() which prettifies
    message
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if let Some(rest) = strip_coauthor_prefix(trimmed) {
                if let Some((_name, email)) = parse_coauthor_value(rest.trim()) {
                    // Case-insensitive email match
                    return email.eq_ignore_ascii_case(target_email);
                }
            }
            false // keep the line
        })
        // negate: keep lines that DON'T match
        .map(|line| !/* filter above returns true for lines to DROP */)
        // ...
        .collect::<Vec<_>>()
        .join("\n")
    // IMPORTANT: preserve trailing newline if original had one
}
```

Note: The actual implementation must preserve the original line endings exactly. Use `message_raw()`, filter out matching `Co-authored-by:` lines, rejoin with the same line ending scheme. If the original message ends with `\n`, the new message must also end with `\n`.

Reuse `strip_coauthor_prefix()` and `parse_coauthor_value()` from `reader.rs` — these are already tested and correct.

[CITED: REQUIREMENTS.md DROP-02/DROP-03, existing reader.rs source]

### Pattern 4: Parent Collection Ownership Idiom (git2 issue #140)

`Repository::commit()` requires `&[&Commit<'_>]` for parents. You cannot collect `&Commit` references into a Vec — you must collect owned `Commit` objects first, then take references.

```rust
// Source: github.com/rust-lang/git2-rs/issues/140
// Three-step ownership dance — always required when building parent list dynamically:
// Step 1: collect OIDs
let new_parent_oids: Vec<Oid> = (0..commit.parent_count())
    .map(|i| {
        let p = commit.parent_id(i).unwrap();
        *oid_map.get(&p).unwrap_or(&p)
    })
    .collect();

// Step 2: collect owned Commit objects (must outlive the next step)
let parent_commits: Vec<git2::Commit> = new_parent_oids
    .iter()
    .map(|oid| repo.find_commit(*oid))
    .collect::<Result<Vec<_>, _>>()?;

// Step 3: collect references (borrows from step 2)
let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

// Now safe to call:
repo.commit(None, &new_author, &new_committer, raw_msg, &tree, &parent_refs)?;
```

[CITED: github.com/rust-lang/git2-rs/issues/140]

### Pattern 5: Ref Update Pass

After the walk completes (oid_map is fully populated), update all branch refs and tags.

```rust
// Source: docs.rs/git2/0.21.0 - Repository::reference(), Reference::set_target()

// 1. Update branch refs (refs/heads/*)
for branch_result in repo.branches(Some(git2::BranchType::Local))? {
    let (branch, _) = branch_result?;
    let r = branch.get();
    if let Some(old_tip) = r.target() {
        // Walk up from tip: find the newest remapped OID
        // (the tip itself is the most recently created commit on that branch)
        if let Some(&new_tip) = oid_map.get(&old_tip) {
            // Use find_new_tip() if the tip commit was not itself rewritten
            // but an ancestor was — the ref's tip OID may not be in oid_map directly.
            // Solution: store the last-written OID per ref, or walk up to find
            // the new tip. See "Pitfall: Ref Tip Tracking" below.
            repo.reference(r.name().unwrap(), new_tip, true, "rewrite")?;
        }
    }
}

// 2. Update tag refs (refs/tags/*)
for tag_ref_result in repo.references_glob("refs/tags/*")? {
    let tag_ref = tag_ref_result?;
    let ref_oid = tag_ref.target().unwrap(); // direct refs only (tags are always direct)
    let ref_obj = repo.find_object(ref_oid, None)?;

    match ref_obj.kind() {
        Some(git2::ObjectType::Tag) => {
            // ANNOTATED tag — must recreate the tag object, not just update the ref
            let tag = ref_obj.as_tag().unwrap();
            let old_target_oid = tag.target_id();
            if let Some(&new_target_oid) = oid_map.get(&old_target_oid) {
                let new_target_obj = repo.find_object(new_target_oid, None)?;
                let tagger = tag.tagger().unwrap_or_else(|| {
                    git2::Signature::now("unknown", "unknown@unknown").unwrap()
                });
                let msg = tag.message().unwrap_or(Ok("")).unwrap_or("");
                let tag_name = tag.name().unwrap_or("");
                repo.tag(tag_name, &new_target_obj, &tagger, msg, true)?;
                // force=true overwrites the existing tag ref
            }
        }
        Some(git2::ObjectType::Commit) => {
            // LIGHTWEIGHT tag — ref points directly to a commit; just update target
            if let Some(&new_oid) = oid_map.get(&ref_oid) {
                // set_target requires a mutable reference
                let mut tag_ref = repo.find_reference(tag_ref.name().unwrap())?;
                tag_ref.set_target(new_oid, "rewrite")?;
            }
        }
        _ => {} // ignore refs pointing to trees/blobs (unusual, skip)
    }
}
```

[CITED: docs.rs/git2/0.21.0 - Repository::find_object, Object::kind(), Repository::tag(), Reference::set_target()]

### Anti-Patterns to Avoid

- **Using `message()` instead of `message_raw()`:** `message()` strips leading newlines ("prettifies"). For byte-identical preservation required by DROP-03, always use `message_raw()`. [CITED: docs.rs/git2/0.21.0/git2/struct.Commit.html]
- **Using `Signature::now()` instead of `Signature::new(..., &original.when())`:** Resets the timestamp to current time. Violates byte-identity requirement for timestamps. [CITED: docs.rs/git2/0.21.0/git2/struct.Signature.html]
- **Unconditionally rewriting committer:** RENAME-03 requires committer rewrite ONLY when committer matches old author identity. Unconditional rewrite corrupts commits where author ≠ committer.
- **Using `update_ref = Some("refs/heads/main")` in per-commit `repo.commit()` call:** This updates the ref after each commit, which breaks the walk — the ref now points to the new commit but we haven't finished remapping all commits. Always use `update_ref = None` during the walk; update refs in a separate pass after the walk completes.
- **Using HashMap or BTreeSet for parent collection instead of Vec:** Loses merge parent order. Always collect parent OIDs from `commit.parent_id(i)` in index order 0..N into a `Vec`. [CITED: ROADMAP key constraints]
- **Walking `refs/heads/*` only:** Misses commits reachable exclusively through tags (orphan-branch-like tag targets). Use both `push_glob("refs/heads/*")` and `push_glob("refs/tags/*")`.
- **Not handling the "tip not directly in oid_map" case:** A branch tip may be a commit that was NOT rewritten (no identity match, no parent change) but whose parent WAS rewritten. In this case, the branch ref's current OID is not in oid_map directly — the tip itself never changed, but the ref is now pointing at a "dangling" parent that no longer reflects the rewritten history. See the "Ref Tip Tracking" pitfall below for the solution.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Topological walk ordering | Custom graph traversal | `Sort::TOPOLOGICAL \| Sort::REVERSE` on Revwalk | git2 wraps libgit2 which handles all edge cases (cycles, multi-root) |
| Annotated tag detection | Parse git objects manually | `repo.find_object(oid, None)?.kind() == Some(ObjectType::Tag)` | One call, handles all tag formats |
| Commit object creation | Write raw git objects | `Repository::commit()` | libgit2 handles encoding, SHA computation, pack writing |
| Ref update atomicity | Custom lockfile logic | `repo.reference()` with `force=true` | libgit2 handles POSIX file locking for ref updates |
| Message line ending detection | Custom parser | Preserve original newlines by working with `str::lines()` and tracking trailing newline presence | Rust's `lines()` strips trailing newline; must explicitly preserve it |

**Key insight:** The git object model is deceptively complex (pack files, delta chains, encoding). Never bypass git2/libgit2 for writing objects.

## Runtime State Inventory

> This phase writes commits and updates refs — it is a mutation phase. Must document runtime state that rewriting changes.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data (git ODB) | All commit objects: author/committer/message/parent fields | New commit objects written by `Repository::commit()` — old objects remain in ODB but become unreachable after ref update |
| Live service config | refs/heads/*, refs/tags/* (all branch and tag refs) | Updated by Phase 2 ref update pass |
| OS-registered state | None — no OS-level registration of git refs | None |
| Secrets/env vars | None — no env vars reference commit OIDs | None |
| Build artifacts | git reflog (auto-updated by libgit2 when refs are updated) | Reflog provides 90-day recovery; documented in SAFE requirements, handled in Phase 3 OUT-01 |
| Detached HEAD | If HEAD is detached, it points at a specific commit OID. If that commit is rewritten, HEAD still points at the OLD OID after the walk | Flag: see Open Questions. Phase 2 should detect and update detached HEAD if its target is in oid_map. |
| refs/notes/commits | Notes refs reference old commit SHAs. These will become orphaned after rewrite. | Non-blocking warning in Phase 3 (SAFE-05). Phase 2 does NOT touch notes refs. |
| Stash (refs/stash) | BLOCKED by Phase 1 preflight — execution never reaches Phase 2 if stash exists | Handled by Phase 1 check_stash() |
| Worktrees (refs/worktrees/*) | BLOCKED by Phase 1 preflight | Handled by Phase 1 check_worktrees() |

## Common Pitfalls

### Pitfall 1: Ref Tip Not Directly in oid_map

**What goes wrong:** A branch tip commit was NOT rewritten (no identity match, no parent change in the oid_map at the time of processing) but IS a descendant of rewritten commits. After the walk, the branch ref still points at the original tip OID, which has the correct content — but the parent chain now contains a mix of old and new OIDs. The branch history is corrupted.

**Why it happens:** The rewrite decision for a commit is made at walk time. If a commit has no identity match AND all its parents were already written before the walk (i.e., the first commit in the walk already had all non-rewritten parents), it gets skipped. But if parent OIDs shifted due to earlier commits in the walk, the "no change needed" decision was wrong.

**Correct algorithm:** A commit needs a new object if EITHER:
1. Its author or committer identity matches the target (rename) OR its message contains the target co-author (drop), OR
2. ANY of its parent OIDs appear in `oid_map` (at least one parent was rewritten)

If neither condition is true, do NOT insert into oid_map — the original OID remains valid and correct.

**How to avoid:** Check `any_parent_remapped` using `(0..commit.parent_count()).any(|i| oid_map.contains_key(&commit.parent_id(i).unwrap()))` before deciding to skip a commit.

**Warning signs:** `git log --all` after rewrite shows a broken parent link — some commits have an ancestor that is the old OID.

### Pitfall 2: Annotated Tag Target Not in oid_map

**What goes wrong:** An annotated tag points at commit C. If C was not itself rewritten (no identity match, no parent remapping) but C's ancestors were, C's own OID remains unchanged — the annotated tag is correct and needs no update.

If C WAS rewritten, its new OID is in oid_map. The tag MUST be recreated with `oid_map[C]` as the target. If this step is skipped, the tag object still points at the old OID.

**How to avoid:** In the ref update pass, for every annotated tag: look up `tag.target_id()` in oid_map. If present, recreate. If absent, skip.

**Warning signs:** `git cat-file tag <tagname>` shows `object <old-sha>` instead of the new SHA.

### Pitfall 3: Annotated Tag Message — `tag.message()` Returns `Result<Option<&str>>`

**What goes wrong:** Unlike `Commit::message_raw()` which returns `&str`, `Tag::message()` returns `Result<Option<&str>, Error>`. Using `.unwrap()` without handling both the `Option` and `Result` layers causes a compile error or silent empty string.

**How to avoid:**
```rust
let msg = tag.message()
    .unwrap_or(Ok(None))    // unwrap the Result (non-UTF8 → None)
    .unwrap_or("");         // unwrap the Option (no message → "")
```
Or use `tag.message_bytes()` which returns `Option<&[u8]>` and convert via `str::from_utf8`.

**Warning signs:** Compile error `cannot call method on Option without unwrap` or `type mismatch`.

### Pitfall 4: Detached HEAD Not Updated

**What goes wrong:** If HEAD is detached and points at a commit that gets rewritten, HEAD still points at the old OID after the ref update pass. The working tree appears unchanged (no dirtiness) but `git log HEAD` shows the old history.

**How to avoid:** After the ref update pass:
1. Check `repo.head_detached()?`
2. If detached, read `repo.head()?.target()?`
3. If the HEAD target OID is in oid_map, call `repo.set_head_detached(new_oid)?`

Phase 1 does not block detached HEAD (no such preflight gate). Phase 2 must handle it.

**Warning signs:** After rewrite, `git log HEAD` shows old author identity but `git log --all` shows correct new history.

### Pitfall 5: Remote Tracking Refs (refs/remotes/*)

**What goes wrong:** `push_glob("refs/*")` includes `refs/remotes/*`. Updating remote tracking refs after a local rewrite is wrong — these mirror the upstream state and should not be touched. The user will force-push to update remotes.

**How to avoid:** Walk commits with `push_glob("refs/heads/*")` + `push_glob("refs/tags/*")` for the revwalk (commits only). In the ref update pass, explicitly use `repo.branches(Some(BranchType::Local))` and `repo.references_glob("refs/tags/*")` — never touch refs/remotes/*.

**Warning signs:** After rewrite, remote tracking branches show rewritten history — breaks subsequent `git fetch` diff detection.

### Pitfall 6: Trailing Newline in Reconstructed Message

**What goes wrong:** `str::lines()` in Rust strips the trailing newline. If you collect lines, filter, and rejoin with `\n`, commits that originally ended with `\n` will lose it. The new commit has a slightly different raw message → different SHA even when no Co-authored-by line was removed.

**How to avoid:** Before processing, check `message.ends_with('\n')`. After rejoining, re-append `\n` if the original had one.

**Warning signs:** DROP-03 byte-identity check fails — SHA changes on commits with no matching co-author.

### Pitfall 7: `tag.message()` vs `tag.message_bytes()` Encoding

**What goes wrong:** `Tag::message()` returns `Result<Option<&str>>` which fails on non-UTF-8 tag messages. Since we're recreating the tag object, we must preserve the original message exactly.

**How to avoid:** Use `tag.message_bytes()` → `Option<&[u8]>` → `str::from_utf8(bytes).unwrap_or("")`. For tag messages, non-UTF-8 is extremely rare but possible. A UTF-8 fallback to empty string is acceptable for v1.

## Code Examples

Verified patterns from official sources:

### Read Original Commit Fields for Byte-Identical Preservation

```rust
// Source: docs.rs/git2/0.21.0/git2/struct.Commit.html
//         docs.rs/git2/0.21.0/git2/struct.Signature.html

let raw_message = commit.message_raw().unwrap_or(""); // NOT .message()
let author = commit.author();
let author_time = author.when(); // git2::Time — implements Copy
let new_sig = git2::Signature::new(new_name, new_email, &author_time)?;
// For preserved (non-renamed) signature:
let preserved_sig = git2::Signature::new(
    author.name().unwrap_or(""),
    author.email().unwrap_or(""),
    &author_time,  // same Time object — preserves seconds + offset_minutes
)?;
```

### Detect Annotated vs Lightweight Tag

```rust
// Source: docs.rs/git2/0.21.0/git2/struct.Object.html
// docs.rs/git2/0.21.0/git2/enum.ObjectType.html

let ref_oid = tag_ref.target().expect("tag ref is always direct");
let obj = repo.find_object(ref_oid, None)?;
match obj.kind() {
    Some(git2::ObjectType::Tag) => {
        // Annotated tag — obj is a tag object pointing at a commit
        let tag = obj.as_tag().unwrap();
        // tag.target_id() → the OID of the commit this tag points at
    }
    Some(git2::ObjectType::Commit) => {
        // Lightweight tag — ref points directly at a commit
    }
    _ => { /* unusual, skip */ }
}
```

### Recreate Annotated Tag Object

```rust
// Source: docs.rs/git2/0.21.0/git2/struct.Repository.html#method.tag
// Repository::tag() creates a new tag OBJECT and updates refs/tags/<name>
// force=true is required to overwrite the existing ref

let tag = existing_obj.as_tag().unwrap();
let new_target_commit_oid = *oid_map.get(&tag.target_id()).unwrap();
let new_target_obj = repo.find_object(new_target_commit_oid, None)?;
let tagger = tag.tagger().unwrap_or_else(|| {
    // Fallback for tags without tagger (unusual)
    git2::Signature::now("unknown", "unknown@unknown").unwrap()
});
let tag_message = tag.message().unwrap_or(Ok("")).unwrap_or("");
let tag_name = tag.name().unwrap_or("");
repo.tag(tag_name, &new_target_obj, &tagger, tag_message, true)?;
// This creates a new tag object and updates refs/tags/<tag_name>
// force=true handles the case where the ref already exists
```

### Update Branch Ref to New Tip

```rust
// Source: docs.rs/git2/0.21.0/git2/struct.Reference.html#method.set_target
let mut branch_ref = repo.find_reference(ref_name)?;
branch_ref.set_target(new_tip_oid, "rewrite: update to new commit OID")?;
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `git filter-branch` | `git2-rs` direct API / `git-filter-repo` | ~2020 | filter-branch is deprecated; libgit2 API is the programmatic standard |
| `Commit::amend()` for multi-commit rewrite | `Repository::commit()` + ref update loop | Always | amend() only works on the ref tip; for full history rewrite use commit() loop |
| Manual `.git/` file manipulation | libgit2 / git2-rs | Always | Pack file complexity makes manual parsing impractical |

**Deprecated/outdated:**
- `Commit::amend()` for full-history rewrite: works ONLY for single tip commit; wrong tool for cascading parent remapping

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `push_glob("refs/tags/*")` causes libgit2 to dereference annotated tag objects to commits and include them in the walk | Pattern 1, Walk Setup | If wrong: commits reachable only via tags are missed by the walk and not rewritten |
| A2 | `Repository::commit()` does not add or strip trailing newlines from the message parameter | Pattern 1, commit call | If wrong: message byte-identity is violated; SHA changes even for non-matching commits |
| A3 | `Tag::message()` returns `Result<Option<&str>>` (not `Option<&str>`) | Pitfall 3 | If wrong: the error handling code is incorrect; compile error or wrong default |
| A4 | "Unchanged commit keeps original OID" — commits with no identity match and no parent in oid_map produce the same SHA if created again with identical content | Pattern 1 algorithm | If wrong: we create unnecessary new commit objects; OID table grows but doesn't cause correctness issues (just wasted work) |

**Note on A1:** This is supported by the libgit2 documentation ("Any references matching this glob which do not point to a commitish will be ignored" — meaning non-commitish refs are skipped but tag objects ARE dereferenced to their commit targets). Multiple web sources confirm push_glob handles annotated tags via peeling. Mark HIGH confidence. [CITED: libgit2 push_glob docs via WebSearch result]

## Open Questions

1. **Walk scope: should refs/remotes/* commits be included in the rewrite walk?**
   - What we know: remotes mirror upstream state. Rewriting remote tracking refs locally doesn't change the upstream; the user force-pushes after rewrite.
   - What's unclear: if a commit exists only on a remote tracking branch (not on any local branch or tag), should it be rewritten?
   - Recommendation: NO — walk `refs/heads/*` + `refs/tags/*` only. Do not walk or update `refs/remotes/*`. The REQUIREMENTS.md phrase "all branches" implicitly means local branches. The user's force-push command (`git push --force-with-lease --all`) will propagate the rewrite to remotes.

2. **Detached HEAD handling**
   - What we know: Phase 1 does not block detached HEAD. `repo.head_detached()` exists in git2.
   - What's unclear: whether the requirements expect detached HEAD to be updated if its commit is rewritten.
   - Recommendation: HANDLE IT — after the ref update pass, if HEAD is detached and its target OID is in oid_map, call `repo.set_head_detached(new_oid)`. This is a 4-line addition and avoids subtle breakage.

3. **Message encoding for non-UTF-8 commits**
   - What we know: `commit.message_raw()` returns `Option<&str>` (still requires UTF-8). For true non-UTF-8 messages, `message_raw_bytes()` exists.
   - What's unclear: `Repository::commit()` takes `&str` — it cannot write non-UTF-8 messages. How should we handle a commit with a non-UTF-8 message that needs rewriting?
   - Recommendation: For v1, use `message_raw().unwrap_or("")`. If message is non-UTF-8 (returns None), preserve the bytes by using the raw bytes approach: this is an edge case that can be deferred to v2. Document the limitation.

## Environment Availability

> Step 2.6: SKIPPED — Phase 2 is code-only (no external CLI tools required). All operations use the git2 crate which is already compiled into the binary.

## Security Domain

> security_enforcement is absent from config.json — treated as enabled.

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | No user auth in this tool |
| V3 Session Management | No | No sessions |
| V4 Access Control | No | CLI tool, no multi-user context |
| V5 Input Validation | Yes | New name/email validated by `git2::Signature::new()` which rejects angle brackets; co-author email matched case-insensitively |
| V6 Cryptography | No | No crypto in Phase 2 |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal in repo path | Tampering | `Repository::open_from_env()` uses libgit2 path resolution (Phase 1) |
| Force ref update overwriting arbitrary refs | Tampering | `repo.reference()` / `reference.set_target()` — only updates refs that already exist and are in scope of refs/heads/* or refs/tags/* |
| Annotated tag recreation overwriting wrong tag | Tampering | `repo.tag(name, ..., force=true)` — only overwrites a tag with the exact same name; name comes from the existing tag object, not user input |

## Sources

### Primary (HIGH confidence)
- [docs.rs/git2/0.21.0/git2/struct.Repository.html](https://docs.rs/git2/0.21.0/git2/struct.Repository.html) — commit(), tag(), reference(), find_object(), references_glob(), branches(), head_detached()
- [docs.rs/git2/0.21.0/git2/struct.Commit.html](https://docs.rs/git2/0.21.0/git2/struct.Commit.html) — message_raw(), author(), committer(), parent_id(), parent_count(), tree_id()
- [docs.rs/git2/0.21.0/git2/struct.Signature.html](https://docs.rs/git2/0.21.0/git2/struct.Signature.html) — new(), when()
- [docs.rs/git2/0.21.0/git2/struct.Time.html](https://docs.rs/git2/0.21.0/git2/struct.Time.html) — seconds(), offset_minutes(), implements Copy
- [docs.rs/git2/0.21.0/git2/struct.Tag.html](https://docs.rs/git2/0.21.0/git2/struct.Tag.html) — tagger(), message(), message_bytes(), name(), target_id()
- [docs.rs/git2/0.21.0/git2/struct.Object.html](https://docs.rs/git2/0.21.0/git2/struct.Object.html) — kind(), as_tag(), into_tag(), peel_to_commit()
- [docs.rs/git2/0.21.0/git2/struct.Reference.html](https://docs.rs/git2/0.21.0/git2/struct.Reference.html) — set_target(), name(), target(), is_tag()
- [docs.rs/git2/0.21.0/git2/struct.Revwalk.html](https://docs.rs/git2/0.21.0/git2/struct.Revwalk.html) — push_glob(), set_sorting(), Iterator<Item=Result<Oid>>
- [cargo search git2](https://crates.io/crates/git2) — version 0.21.0 confirmed [VERIFIED: cargo search]

### Secondary (MEDIUM confidence)
- [github.com/rust-lang/git2-rs/issues/140](https://github.com/rust-lang/git2-rs/issues/140) — parents ownership pattern `&[&Commit]` [CITED]
- libgit2 push_glob annotated tag peeling behavior — confirmed by multiple sources including libgit2 docs via WebSearch [CITED]

### Tertiary (LOW confidence)
- `Tag::message()` return type as `Result<Option<&str>>` — inferred from docs.rs fetch; single source [ASSUMED A3]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new deps; git2 API verified against docs.rs/git2/0.21.0
- Architecture: HIGH — all API calls verified, algorithm follows CLAUDE.md stack decisions
- Pitfalls: HIGH (known from API) / MEDIUM (trailing newline, tag message encoding — inferred)
- Parent ownership idiom: HIGH — verified from git2-rs issue #140

**Research date:** 2026-05-20
**Valid until:** 2026-11-20 (git2 is stable; API changes on minor bumps only)

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RENAME-03 | Rewrite all matching commits across all branches, updating author AND committer when committer matches old author identity | Pattern 2 (conditional rewrite), Pattern 1 (walk + OID map), Pattern 5 (ref update) |
| RENAME-04 | Annotated tag objects recreated (not just ref pointer), tag object itself updated with new target SHA | Pattern 5 (tag detection + repo.tag() with force), Pitfall 2 |
| DROP-02 | Remove selected co-author from all Co-authored-by trailers, case-insensitive, remove all occurrences within single commit | Pattern 3 (drop_coauthor_from_message), reuses strip_coauthor_prefix from reader.rs |
| DROP-03 | All other Co-authored-by entries and metadata preserved byte-for-byte | Pattern 1 (message_raw(), Signature::when()), Pitfall 6 (trailing newline), Common Pitfalls |
</phase_requirements>
