pub mod parse;
pub mod path;
pub mod render;
pub mod write;

use std::fs;
use std::io;
use std::path::PathBuf;

pub enum HookState {
    Absent,
    Managed { emails: Vec<String> },
    NotToolManaged(PathBuf),
}

pub enum AddResult {
    Installed { count: usize },
    AlreadyStripped,
}

pub enum RemoveResult {
    Updated { remaining: usize },
    HookDeleted,
    NotFound,
}

/// Return the current state of the commit-msg hook: absent, tool-managed, or foreign.
pub fn read_strip_list(repo: &git2::Repository) -> Result<HookState, crate::error::AppError> {
    let hook_path = path::commit_msg_hook_path(repo);
    if !hook_path.exists() {
        return Ok(HookState::Absent);
    }
    let contents = fs::read_to_string(&hook_path)?;
    if parse::detect_markers(&contents).is_none() {
        return Ok(HookState::NotToolManaged(hook_path));
    }
    Ok(HookState::Managed {
        emails: parse::extract_strip_list(&contents),
    })
}

/// Add `email` to the commit-msg hook strip list.
///
/// - Absent hook: creates the file.
/// - Tool-managed hook: appends the email if not already present.
/// - Foreign hook (no markers): returns `Err(AppError::HookExists)`.
/// - Duplicate email: returns `Ok(AddResult::AlreadyStripped)` without writing.
///
/// Emails are stored lowercase; the caller does not need to pre-lowercase.
pub fn install_strip(
    repo: &git2::Repository,
    email: &str,
) -> Result<AddResult, crate::error::AppError> {
    if email.is_empty() || render::validate_email_for_embedding(email).is_err() {
        return Err(crate::error::AppError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "email is empty or contains characters forbidden for hook embedding",
        )));
    }
    let lowered = email.to_ascii_lowercase();
    let hook_path = path::commit_msg_hook_path(repo);
    let mut emails = match read_strip_list(repo)? {
        HookState::NotToolManaged(p) => return Err(crate::error::AppError::HookExists(p)),
        HookState::Absent => vec![],
        HookState::Managed { emails } => emails,
    };
    if emails.iter().any(|e| e.eq_ignore_ascii_case(&lowered)) {
        return Ok(AddResult::AlreadyStripped);
    }
    emails.push(lowered);
    write::atomic_write_executable(&hook_path, &render::render_hook(&emails))?;
    Ok(AddResult::Installed {
        count: emails.len(),
    })
}

/// Remove `email` from the commit-msg hook strip list.
///
/// - Last entry: deletes the hook file and returns `Ok(RemoveResult::HookDeleted)`.
/// - Non-last: rewrites the file with the email removed.
/// - Email not in list or hook absent: returns `Ok(RemoveResult::NotFound)`.
/// - Foreign hook: returns `Err(AppError::HookExists)`.
pub fn remove_strip(
    repo: &git2::Repository,
    email: &str,
) -> Result<RemoveResult, crate::error::AppError> {
    let hook_path = path::commit_msg_hook_path(repo);
    let lowered = email.to_ascii_lowercase();
    let emails = match read_strip_list(repo)? {
        HookState::NotToolManaged(p) => return Err(crate::error::AppError::HookExists(p)),
        HookState::Absent => return Ok(RemoveResult::NotFound),
        HookState::Managed { emails } => emails,
    };
    let original_len = emails.len();
    let filtered: Vec<String> = emails
        .into_iter()
        .filter(|e| !e.eq_ignore_ascii_case(&lowered))
        .collect();
    if filtered.len() == original_len {
        return Ok(RemoveResult::NotFound);
    }
    if filtered.is_empty() {
        write::delete_hook(&hook_path)?;
        return Ok(RemoveResult::HookDeleted);
    }
    write::atomic_write_executable(&hook_path, &render::render_hook(&filtered))?;
    Ok(RemoveResult::Updated {
        remaining: filtered.len(),
    })
}
