use std::collections::HashMap;
use git2::Sort;

pub fn enumerate_authors(
    repo: &git2::Repository,
) -> Result<Vec<crate::git::types::AuthorIdentity>, crate::error::AppError> {
    let revwalk = build_revwalk(repo)?;
    let mut counts: HashMap<(String, String), usize> = HashMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let author = commit.author();
        let name = author.name().unwrap_or("").to_string();
        let email = author.email().unwrap_or("").to_string();
        *counts.entry((name, email)).or_insert(0) += 1;
    }

    let mut result: Vec<crate::git::types::AuthorIdentity> = counts
        .into_iter()
        .map(|((name, email), count)| crate::git::types::AuthorIdentity {
            name,
            email,
            commit_count: count,
        })
        .collect();

    result.sort_by(|a, b| b.commit_count.cmp(&a.commit_count));
    Ok(result)
}

pub fn enumerate_coauthors(
    repo: &git2::Repository,
) -> Result<Vec<crate::git::types::CoAuthorEntry>, crate::error::AppError> {
    let revwalk = build_revwalk(repo)?;
    let mut counts: HashMap<(String, String), usize> = HashMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let message = commit.message().unwrap_or("");
        for line in message.lines() {
            let trimmed = line.trim();
            if let Some(rest) = strip_coauthor_prefix(trimmed) {
                if let Some((name, email)) = parse_coauthor_value(rest.trim()) {
                    *counts.entry((name, email)).or_insert(0) += 1;
                }
            }
        }
    }

    let mut result: Vec<crate::git::types::CoAuthorEntry> = counts
        .into_iter()
        .map(|((name, email), count)| crate::git::types::CoAuthorEntry {
            name,
            email,
            commit_count: count,
        })
        .collect();

    result.sort_by(|a, b| b.commit_count.cmp(&a.commit_count));
    Ok(result)
}

fn build_revwalk(repo: &git2::Repository) -> Result<git2::Revwalk<'_>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("refs/heads/*")?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
    Ok(revwalk)
}

/// Case-insensitive strip of "co-authored-by:" prefix.
/// Returns the rest of the line after the prefix, or None if no match.
fn strip_coauthor_prefix(line: &str) -> Option<&str> {
    let prefix = "co-authored-by:";
    if line.len() >= prefix.len() && line[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&line[prefix.len()..])
    } else {
        None
    }
}

/// Parse "Name <email>" -> (name, email). Returns None on malformed input.
fn parse_coauthor_value(value: &str) -> Option<(String, String)> {
    let lt = value.rfind('<')?;
    let gt = value.rfind('>')?;
    if gt < lt {
        return None;
    }
    let name = value[..lt].trim().to_string();
    let email = value[lt + 1..gt].trim().to_string();
    if name.is_empty() && email.is_empty() {
        return None;
    }
    Some((name, email))
}
