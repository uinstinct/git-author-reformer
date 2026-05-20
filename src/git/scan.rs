use crate::git::reader::{parse_coauthor_value, strip_coauthor_prefix};
use git2::{ObjectType, Oid, Sort};
use std::collections::HashSet;

/// Read-only preview of what a rewrite operation would affect.
/// Computed before any rewrite is attempted — satisfies RENAME-05 and DROP-04.
#[derive(Debug, Clone)]
pub struct RewritePreview {
    /// Number of commit objects that would be written (cascade-accurate).
    pub affected_count: usize,
    /// GPG/SSH signed commits inside the cascade set (SAFE-03).
    pub signed_commit_count: usize,
    /// Short names of annotated tags whose target commit is in the cascade set (SAFE-04).
    pub annotated_tags_affected: Vec<String>,
    /// True when refs/notes/commits (or the configured default notes ref) exists (SAFE-05).
    pub has_notes_ref: bool,
    /// "origin" if that remote exists, else the first remote name, else None (OUT-01).
    pub remote_name: Option<String>,
}

/// Returns a `RewritePreview` describing which commits `rewrite_author` would touch
/// for the given `old_name` + `old_email` identity — without writing anything.
///
/// Replicates the cascade logic from `rewrite.rs` exactly so the count shown at
/// the confirmation prompt matches what is actually rewritten (RENAME-05).
pub fn scan_rename(
    repo: &git2::Repository,
    old_name: &str,
    old_email: &str,
) -> Result<RewritePreview, crate::error::AppError> {
    let mut revwalk = build_full_revwalk(repo)?;
    let mut would_remap: HashSet<Oid> = HashSet::new();

    for oid_result in &mut revwalk {
        let old_oid = oid_result?;
        let commit = repo.find_commit(old_oid)?;

        let identity_matches = commit.author().name().unwrap_or("") == old_name
            && commit.author().email().unwrap_or("") == old_email;

        let any_parent_remapped = (0..commit.parent_count())
            .any(|i| would_remap.contains(&commit.parent_id(i).unwrap()));

        if identity_matches || any_parent_remapped {
            would_remap.insert(old_oid);
        }
    }

    let affected_count = would_remap.len();
    let (signed_commit_count, annotated_tags_affected, has_notes_ref, remote_name) =
        collect_warnings(repo, &would_remap)?;

    Ok(RewritePreview {
        affected_count,
        signed_commit_count,
        annotated_tags_affected,
        has_notes_ref,
        remote_name,
    })
}

/// Returns a `RewritePreview` describing which commits `drop_coauthor` would touch
/// for the given `target_email` — without writing anything.
///
/// Replicates the cascade logic from `rewrite.rs` exactly (DROP-04).
pub fn scan_drop(
    repo: &git2::Repository,
    target_email: &str,
) -> Result<RewritePreview, crate::error::AppError> {
    let mut revwalk = build_full_revwalk(repo)?;
    let mut would_remap: HashSet<Oid> = HashSet::new();

    for oid_result in &mut revwalk {
        let old_oid = oid_result?;
        let commit = repo.find_commit(old_oid)?;

        // Use message_raw() — message() strips leading newlines (anti-pattern).
        // Silent skip on non-UTF-8 for read-only scan (no error propagation needed).
        let raw_msg = commit.message_raw().unwrap_or("");
        let message_would_change = message_has_matching_coauthor(raw_msg, target_email);

        let any_parent_remapped = (0..commit.parent_count())
            .any(|i| would_remap.contains(&commit.parent_id(i).unwrap()));

        if message_would_change || any_parent_remapped {
            would_remap.insert(old_oid);
        }
    }

    let affected_count = would_remap.len();
    let (signed_commit_count, annotated_tags_affected, has_notes_ref, remote_name) =
        collect_warnings(repo, &would_remap)?;

    Ok(RewritePreview {
        affected_count,
        signed_commit_count,
        annotated_tags_affected,
        has_notes_ref,
        remote_name,
    })
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Build a full revwalk over refs/heads/* and refs/tags/*, topological + reverse.
/// Both scan_rename and scan_drop use identical revwalk setup — extracted here to
/// avoid duplication (REFACTOR pass, Task 5).
fn build_full_revwalk(repo: &git2::Repository) -> Result<git2::Revwalk<'_>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("refs/heads/*")?;
    revwalk.push_glob("refs/tags/*")?;
    // TOPOLOGICAL | REVERSE: parents are always processed before children,
    // so would_remap.contains(parent_id) is always valid at each step.
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
    Ok(revwalk)
}

/// Collect warning fields from the cascade set:
/// (signed_commit_count, annotated_tags_affected, has_notes_ref, remote_name)
fn collect_warnings(
    repo: &git2::Repository,
    would_remap: &HashSet<Oid>,
) -> Result<(usize, Vec<String>, bool, Option<String>), git2::Error> {
    let signed_commit_count = count_signed_commits(repo, would_remap)?;
    let annotated_tags_affected = collect_affected_annotated_tags(repo, would_remap)?;
    let has_notes = check_has_notes_ref(repo);
    let remote = detect_remote_name(repo);
    Ok((signed_commit_count, annotated_tags_affected, has_notes, remote))
}

/// Count commits in the cascade set that carry a GPG or SSH signature header (SAFE-03).
fn count_signed_commits(
    repo: &git2::Repository,
    would_remap: &HashSet<Oid>,
) -> Result<usize, git2::Error> {
    let mut count = 0usize;
    for &oid in would_remap {
        let commit = repo.find_commit(oid)?;
        if commit_is_signed(&commit) {
            count += 1;
        }
    }
    Ok(count)
}

/// Check whether a commit carries a GPG or SSH signature header.
fn commit_is_signed(commit: &git2::Commit) -> bool {
    commit.header_field_bytes("gpgsig").is_ok()
        || commit.header_field_bytes("sshsig").is_ok()
}

/// Collect short names (stripped of "refs/tags/" prefix) of annotated tags whose
/// underlying commit is in the cascade set (SAFE-04).
/// Lightweight tags (refs pointing directly at a commit) are skipped — they are
/// silently updated as ref pointer changes and need no warning.
fn collect_affected_annotated_tags(
    repo: &git2::Repository,
    would_remap: &HashSet<Oid>,
) -> Result<Vec<String>, git2::Error> {
    let mut result = Vec::new();
    for tag_ref in repo.references_glob("refs/tags/*")? {
        let tag_ref = tag_ref?;
        let ref_oid = match tag_ref.target() {
            Some(oid) => oid,
            None => continue,
        };
        let obj = repo.find_object(ref_oid, None)?;
        if obj.kind() == Some(ObjectType::Tag) {
            let tag = obj.as_tag().unwrap();
            if would_remap.contains(&tag.target_id()) {
                // Strip "refs/tags/" prefix for short name.
                let full_name = tag_ref.name().unwrap_or("");
                let short_name = full_name
                    .strip_prefix("refs/tags/")
                    .unwrap_or(full_name)
                    .to_string();
                result.push(short_name);
            }
        }
        // ObjectType::Commit = lightweight tag — skip (no warning needed).
    }
    Ok(result)
}

/// Check whether a notes ref exists in the repository (SAFE-05).
/// Checks the configured default notes ref first, then the canonical location.
fn check_has_notes_ref(repo: &git2::Repository) -> bool {
    let default_ref = repo
        .note_default_ref()
        .unwrap_or_else(|_| "refs/notes/commits".to_string());
    repo.find_reference(&default_ref).is_ok()
        || repo.find_reference("refs/notes/commits").is_ok()
}

/// Resolve the remote name: prefer "origin", else first remote, else None (OUT-01).
/// Returns None on error — remote enumeration is best-effort warning data.
fn detect_remote_name(repo: &git2::Repository) -> Option<String> {
    let remotes = repo.remotes().ok()?;
    // remotes.iter() yields Result<Option<&str>, Error>; flatten twice to get &str items.
    let names: Vec<&str> = remotes
        .iter()
        .filter_map(|r| r.ok().flatten())
        .collect();
    if names.contains(&"origin") {
        Some("origin".to_string())
    } else {
        names.first().map(|s| s.to_string())
    }
}

/// Check whether a commit message contains a matching Co-authored-by trailer for
/// the given email (case-insensitive). Used to determine if drop_coauthor would
/// change this commit's message (and thus include it in the cascade set).
///
/// Normalises CRLF to LF before checking — matching the behaviour of
/// `drop_coauthor_from_message` so CRLF commits don't falsely trigger cascade.
fn message_has_matching_coauthor(message: &str, target_email: &str) -> bool {
    // Normalise CRLF so CRLF commits don't register as changed when no trailer matches.
    let normalised = message.replace("\r\n", "\n");
    for line in normalised.lines() {
        let trimmed = line.trim();
        if let Some(rest) = strip_coauthor_prefix(trimmed) {
            if let Some((_name, email)) = parse_coauthor_value(rest.trim()) {
                if email.eq_ignore_ascii_case(target_email) {
                    return true;
                }
            }
        }
    }
    false
}
