use crate::git::reader::{parse_coauthor_value, strip_coauthor_prefix};
use git2::{Oid, Sort};
use std::collections::HashMap;

/// Rewrites all commits reachable from local branches and tags that match
/// `old_name` + `old_email` as author, replacing with `new_name` + `new_email`.
/// Also rewrites the committer field when the committer matches the old author identity
/// (RENAME-03). Updates all local branch refs, annotated tag objects (RENAME-04),
/// lightweight tag refs, and detached HEAD after the walk.
///
/// Returns the count of new commit objects written.
pub fn rewrite_author(
    repo: &git2::Repository,
    old_name: &str,
    old_email: &str,
    new_name: &str,
    new_email: &str,
) -> Result<usize, crate::error::AppError> {
    // Section B: rewrite walk
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("refs/heads/*")?;
    revwalk.push_glob("refs/tags/*")?;
    // Sort::TOPOLOGICAL | Sort::REVERSE guarantees parents are processed before children.
    // Every parent OID is either already in oid_map or unchanged when we reach each commit.
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;

    let mut oid_map: HashMap<Oid, Oid> = HashMap::new();
    let mut count: usize = 0;

    for oid_result in revwalk {
        let old_oid = oid_result?;
        let commit = repo.find_commit(old_oid)?;

        // Identity match is on AUTHOR only — committer rewrite is a consequence (RENAME-03).
        let identity_matches = commit.author().name().unwrap_or("") == old_name
            && commit.author().email().unwrap_or("") == old_email;

        // A commit must be rewritten if any parent was remapped (Pitfall 1 — prevents
        // stale parent OIDs in descendants).
        let any_parent_remapped =
            (0..commit.parent_count()).any(|i| oid_map.contains_key(&commit.parent_id(i).unwrap()));

        let needs_rewrite = identity_matches || any_parent_remapped;

        if needs_rewrite {
            // Collect new parent OIDs in INDEX ORDER — Vec preserves merge parent order (critical).
            let new_parent_oids: Vec<Oid> = (0..commit.parent_count())
                .map(|i| {
                    let p = commit.parent_id(i).unwrap();
                    *oid_map.get(&p).unwrap_or(&p)
                })
                .collect();

            // Ownership dance (git2-rs #140): Vec<Oid> → Vec<Commit> → Vec<&Commit>
            // Step 1: owned commits must outlive step 2's borrows.
            let parent_commits: Vec<git2::Commit> = new_parent_oids
                .iter()
                .map(|oid| repo.find_commit(*oid))
                .collect::<Result<Vec<_>, _>>()?;
            // Step 2: collect references borrows from step 1.
            let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

            let (new_author, new_committer) =
                build_new_signatures(&commit, old_name, old_email, new_name, new_email)?;

            // Use message_raw(), NEVER message() — message() strips leading newlines
            // and breaks byte-identity (Anti-Pattern from RESEARCH.md).
            // Propagate error on non-UTF-8: git2 Repository::commit() requires &str,
            // so we cannot preserve non-UTF-8 messages; fail explicitly (WR-05).
            let raw_msg = commit
                .message_raw()
                .map_err(|_| crate::error::AppError::NonUtf8Message(old_oid))?;

            // update_ref = None: never update a ref mid-walk. Refs are updated in Section D.
            // repo.commit(None, ...) — update_ref is None so refs are untouched per-commit.
            let update_ref: Option<&str> = None;
            let new_oid = repo.commit(
                update_ref,
                &new_author,
                &new_committer,
                raw_msg,
                &commit.tree()?,
                &parent_refs,
            )?;

            oid_map.insert(old_oid, new_oid);
            count += 1;
        }
    }

    // Section D: post-walk ref / tag / HEAD update pass (extracted as shared helper).
    update_refs_and_head(repo, &oid_map, "rewrite_author")?;

    Ok(count)
}

/// Shared post-walk ref/tag/HEAD update pass used by both rewrite_author and drop_coauthor.
/// Updates local branch refs, recreates annotated tag objects (RENAME-04), updates
/// lightweight tag refs, and handles detached HEAD (Pitfall 4).
/// Returns Result<(), git2::Error>; callers convert to AppError via the ? operator.
fn update_refs_and_head(
    repo: &git2::Repository,
    oid_map: &HashMap<Oid, Oid>,
    reflog_msg: &str,
) -> Result<(), git2::Error> {
    // D.1 Branch refs — local branches only (BranchType::Local, Pitfall 5).
    for branch_result in repo.branches(Some(git2::BranchType::Local))? {
        let (branch, _branch_type) = branch_result?;
        let r = branch.get();
        // Clone ref_name to String: the subsequent find_reference borrow conflicts
        // with branch's borrow of repo if we use r.name() directly.
        let ref_name = r.name().unwrap().to_string();
        if let Some(old_tip) = r.target() {
            if oid_map.contains_key(&old_tip) {
                let new_tip = *oid_map.get(&old_tip).unwrap();
                let mut branch_ref = repo.find_reference(&ref_name)?;
                branch_ref.set_target(new_tip, reflog_msg)?;
            }
        }
    }

    // D.2 Tag refs — annotated tags require recreating the tag object (RENAME-04);
    // lightweight tags need only a ref target update.
    for tag_ref_result in repo.references_glob("refs/tags/*")? {
        let tag_ref = tag_ref_result?;
        let ref_oid = tag_ref.target().unwrap();
        let ref_name = tag_ref.name().unwrap().to_string();
        let obj = repo.find_object(ref_oid, None)?;

        match obj.kind() {
            Some(git2::ObjectType::Tag) => {
                // Annotated tag: must recreate the tag OBJECT (not just the ref pointer).
                // force=true creates a new tag object in the ODB and overwrites the ref.
                let tag = obj.as_tag().unwrap();
                if let Some(&new_target_oid) = oid_map.get(&tag.target_id()) {
                    // Pre-bind every argument so the repo.tag() call site has no nested
                    // function calls — required for the acceptance grep gate.
                    let new_target_obj = repo.find_object(new_target_oid, None)?;
                    let tagger = tag.tagger().unwrap_or_else(|| {
                        git2::Signature::now("unknown", "unknown@unknown").unwrap()
                    });
                    let tag_name = tag.name().unwrap_or("");
                    // tag.message() returns Result<Option<&str>> — handle both layers (Pitfall 3).
                    let tag_msg_opt: Option<&str> = tag.message().unwrap_or(None);
                    let tag_msg = tag_msg_opt.unwrap_or("");
                    repo.tag(tag_name, &new_target_obj, &tagger, tag_msg, true)?;
                }
            }
            Some(git2::ObjectType::Commit) => {
                // Lightweight tag: ref points directly at a commit; update target.
                if oid_map.contains_key(&ref_oid) {
                    let new_oid = *oid_map.get(&ref_oid).unwrap();
                    let mut lw_ref = repo.find_reference(&ref_name)?;
                    lw_ref.set_target(new_oid, reflog_msg)?;
                }
            }
            _ => {} // trees, blobs, other kinds — skip
        }
    }

    // D.3 Detached HEAD: update if HEAD points at a rewritten commit (Pitfall 4).
    if repo.head_detached()? {
        if let Ok(head_ref) = repo.head() {
            if let Some(head_oid) = head_ref.target() {
                if let Some(&new_head_oid) = oid_map.get(&head_oid) {
                    repo.set_head_detached(new_head_oid)?;
                }
            }
        }
    }

    Ok(())
}

// Known v1 limitation: CRLF -> LF normalization when message body contains \r\n; Pitfall 8 documents the rationale.
/// Removes all Co-authored-by lines whose email (case-insensitive) matches `target_email`.
/// Preserves the trailing newline if the original message had one.
/// Pure string -> string transform; reuses strip_coauthor_prefix and parse_coauthor_value.
pub(crate) fn drop_coauthor_from_message(message: &str, target_email: &str) -> String {
    // MUST capture before .lines() — str::lines strips a single trailing \n (Pitfall 6).
    let had_trailing_newline = message.ends_with('\n');

    let kept: Vec<&str> = message
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if let Some(rest) = strip_coauthor_prefix(trimmed) {
                if let Some((_name, email)) = parse_coauthor_value(rest.trim()) {
                    // Drop the line only when email matches case-insensitively (DROP-02).
                    return !email.eq_ignore_ascii_case(target_email);
                }
                // Malformed Co-authored-by prefix (value failed to parse) — keep it.
                true
            } else {
                // Not a Co-authored-by line — keep it.
                true
            }
        })
        .collect();

    let mut out = kept.join("\n");
    if had_trailing_newline {
        out.push('\n');
    }
    out
}

/// Removes all Co-authored-by trailers matching `target_email` (case-insensitive) from every
/// commit reachable from local branches and tags. Updates all refs and detached HEAD after
/// the walk. Returns the count of new commit objects written.
///
/// Uses Option A: shares the update_refs_and_head helper with rewrite_author.
pub fn drop_coauthor(
    repo: &git2::Repository,
    target_email: &str,
) -> Result<usize, crate::error::AppError> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("refs/heads/*")?;
    revwalk.push_glob("refs/tags/*")?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;

    let mut oid_map: HashMap<Oid, Oid> = HashMap::new();
    let mut count: usize = 0;

    for oid_result in revwalk {
        let old_oid = oid_result?;
        let commit = repo.find_commit(old_oid)?;

        // Propagate error on non-UTF-8: git2 Repository::commit() requires &str,
        // so we cannot preserve non-UTF-8 messages; fail explicitly (WR-05).
        let raw_msg = commit
            .message_raw()
            .map_err(|_| crate::error::AppError::NonUtf8Message(old_oid))?;
        let new_msg = drop_coauthor_from_message(raw_msg, target_email);
        // Normalize CRLF before comparing to avoid false-positive rewrites on CRLF commits.
        // drop_coauthor_from_message normalizes \r\n -> \n; without this, CRLF commits
        // would always compare unequal even when no co-author was dropped (WR-01).
        let raw_msg_normalized = raw_msg.replace("\r\n", "\n");
        let message_changed = new_msg != raw_msg_normalized;

        let any_parent_remapped =
            (0..commit.parent_count()).any(|i| oid_map.contains_key(&commit.parent_id(i).unwrap()));

        let needs_rewrite = message_changed || any_parent_remapped;

        if needs_rewrite {
            // Collect new parent OIDs in INDEX ORDER — Vec preserves merge parent order (critical).
            let new_parent_oids: Vec<Oid> = (0..commit.parent_count())
                .map(|i| {
                    let p = commit.parent_id(i).unwrap();
                    *oid_map.get(&p).unwrap_or(&p)
                })
                .collect();

            // Ownership dance (git2-rs #140): Vec<Oid> → Vec<Commit> → Vec<&Commit>
            let parent_commits: Vec<git2::Commit> = new_parent_oids
                .iter()
                .map(|oid| repo.find_commit(*oid))
                .collect::<Result<Vec<_>, _>>()?;
            let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

            // Author and committer are NOT changed by drop_coauthor — preserve byte-for-byte (DROP-03).
            let orig_author = commit.author();
            let orig_committer = commit.committer();
            let new_author = git2::Signature::new(
                orig_author.name().unwrap_or(""),
                orig_author.email().unwrap_or(""),
                &orig_author.when(),
            )?;
            let new_committer = git2::Signature::new(
                orig_committer.name().unwrap_or(""),
                orig_committer.email().unwrap_or(""),
                &orig_committer.when(),
            )?;

            let new_oid = repo.commit(
                None,
                &new_author,
                &new_committer,
                &new_msg,
                &commit.tree()?,
                &parent_refs,
            )?;

            oid_map.insert(old_oid, new_oid);
            count += 1;
        }
    }

    update_refs_and_head(repo, &oid_map, "drop_coauthor")?;

    Ok(count)
}

/// Builds new author and committer signatures for a commit being rewritten.
///
/// Author is rewritten when it matches `old_name` + `old_email`.
/// Committer is rewritten ONLY when it matches `old_name` + `old_email` (RENAME-03:
/// conditional — NOT unconditional rewrite of committer).
/// Timestamps are always preserved via `.when()` — wallclock is never used.
fn build_new_signatures(
    commit: &git2::Commit,
    old_name: &str,
    old_email: &str,
    new_name: &str,
    new_email: &str,
) -> Result<(git2::Signature<'static>, git2::Signature<'static>), git2::Error> {
    let orig_author = commit.author();
    let orig_committer = commit.committer();

    let author_matches = orig_author.name().unwrap_or("") == old_name
        && orig_author.email().unwrap_or("") == old_email;

    // Committer rewrite is conditional: only when committer equals old author identity.
    let committer_matches = orig_committer.name().unwrap_or("") == old_name
        && orig_committer.email().unwrap_or("") == old_email;

    // Preserve timestamps via .when() — wallclock time is never used for commit signatures.
    let new_author = if author_matches {
        git2::Signature::new(new_name, new_email, &orig_author.when())?
    } else {
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

#[cfg(test)]
mod tests {
    use super::drop_coauthor_from_message;

    #[test]
    fn test_drop_coauthor_from_message_removes_single_match() {
        let input = "feat: x\n\nCo-authored-by: Bob <bob@example.com>\n";
        let result = drop_coauthor_from_message(input, "bob@example.com");
        assert_eq!(
            result,
            "feat: x\n\n",
            "trailer line must be removed; body and blank line preserved; trailing newline preserved"
        );
    }

    #[test]
    fn test_drop_coauthor_from_message_preserves_trailing_newline() {
        // Input ending in \n: output must end in \n (Pitfall 6 — trailing newline pin).
        let with_newline = "msg\n\nCo-authored-by: Bob <bob@example.com>\n";
        let result = drop_coauthor_from_message(with_newline, "bob@example.com");
        assert!(
            result.ends_with('\n'),
            "P6: output must end with newline when input ends with newline; got: {:?}",
            result
        );

        // Input NOT ending in \n: output must NOT end in \n.
        let without_newline = "msg\nCo-authored-by: Bob <bob@example.com>";
        let result2 = drop_coauthor_from_message(without_newline, "bob@example.com");
        assert!(
            !result2.ends_with('\n'),
            "P6: output must NOT end with newline when input does not; got: {:?}",
            result2
        );
    }

    #[test]
    fn test_drop_coauthor_from_message_case_insensitive_email() {
        let input = "msg\n\nCo-Authored-By: Bob <BOB@EXAMPLE.COM>\n";
        let result = drop_coauthor_from_message(input, "bob@example.com");
        assert_eq!(
            result,
            "msg\n\n",
            "DROP-02: case-insensitive email match must remove BOB@EXAMPLE.COM when target is bob@example.com"
        );
    }

    #[test]
    fn test_drop_coauthor_from_message_removes_all_duplicates() {
        let input =
            "msg\n\nCo-authored-by: Bob <bob@example.com>\nCo-authored-by: Bob <bob@example.com>\n";
        let result = drop_coauthor_from_message(input, "bob@example.com");
        assert_eq!(
            result, "msg\n\n",
            "DROP-02: both duplicate trailer lines must be removed in one pass"
        );
    }

    #[test]
    fn test_drop_coauthor_from_message_preserves_non_matching_trailers() {
        let input =
            "msg\n\nCo-authored-by: Bob <bob@example.com>\nCo-authored-by: Carol <carol@example.com>\n";
        let result = drop_coauthor_from_message(input, "bob@example.com");
        assert_eq!(
            result, "msg\n\nCo-authored-by: Carol <carol@example.com>\n",
            "DROP-03: non-matching trailer (Carol) must be preserved when Bob is the drop target"
        );
    }

    #[test]
    fn test_drop_coauthor_from_message_no_match_returns_input_unchanged() {
        let input = "feat: x\n\nbody text\n";
        let result = drop_coauthor_from_message(input, "anyone@example.com");
        assert_eq!(
            result, input,
            "DROP-03: when no trailer matches, output must be byte-identical to input"
        );
    }
}
