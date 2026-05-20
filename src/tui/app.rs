use crate::git::types::{AuthorIdentity, CoAuthorEntry};
use git2::Repository;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};
use std::sync::Arc;

pub struct App {
    pub repo: Repository,
    pub screen: Screen,
    pub should_exit: bool,
}

pub enum Screen {
    MainMenu { selected: usize },
    NotImplemented(&'static str),
    AuthorList {
        items: Vec<AuthorIdentity>,
        filter: String,
        matched: Vec<AuthorIdentity>,
        nucleo: Nucleo<AuthorIdentity>,
        selected: usize,
    },
    RenameForm {
        source: AuthorIdentity,
        draft: RenameDraft,
    },
    /// Placeholder destination for RenameForm Enter — Plan 03-05 adds scan field.
    Preview(PendingOp),
    CoAuthorList {
        items: Vec<CoAuthorEntry>,
        filter: String,
        matched: Vec<CoAuthorEntry>,
        nucleo: Nucleo<CoAuthorEntry>,
        selected: usize,
    },
}

pub struct RenameDraft {
    pub new_name: String,
    pub new_email: String,
    pub focused: FormField,
}

impl Default for RenameDraft {
    fn default() -> Self {
        Self {
            new_name: String::new(),
            new_email: String::new(),
            focused: FormField::Name,
        }
    }
}

impl RenameDraft {
    pub fn is_complete(&self) -> bool {
        !self.new_name.trim().is_empty() && !self.new_email.trim().is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormField {
    Name,
    Email,
}

impl FormField {
    pub fn toggle(self) -> Self {
        match self {
            Self::Name => Self::Email,
            Self::Email => Self::Name,
        }
    }
}

#[derive(Debug)]
pub enum PendingOp {
    Rename {
        source: AuthorIdentity,
        new_name: String,
        new_email: String,
    },
    Drop {
        target: CoAuthorEntry,
    },
}

pub enum MenuChoice {
    Rename,
    Drop,
}

impl MenuChoice {
    pub fn from_index(i: usize) -> Self {
        if i == 0 {
            Self::Rename
        } else {
            Self::Drop
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Rename => "Rename an author",
            Self::Drop => "Drop a co-author",
        }
    }

    pub fn all() -> [Self; 2] {
        [Self::Rename, Self::Drop]
    }
}

impl App {
    pub fn new(repo: Repository) -> Self {
        Self {
            repo,
            screen: Screen::MainMenu { selected: 0 },
            should_exit: false,
        }
    }
}

pub fn build_author_nucleo(items: &[AuthorIdentity]) -> Nucleo<AuthorIdentity> {
    let nucleo = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);
    let injector = nucleo.injector();
    for item in items {
        let display = format!("{} <{}>", item.name, item.email);
        let item = item.clone();
        injector.push(item, move |_, cols| {
            cols[0] = display.clone().into();
        });
    }
    nucleo
}

pub fn apply_filter(nucleo: &mut Nucleo<AuthorIdentity>, query: &str) -> Vec<AuthorIdentity> {
    nucleo
        .pattern
        .reparse(0, query, CaseMatching::Ignore, Normalization::Smart, false);
    nucleo.tick(10);
    let snap = nucleo.snapshot();
    snap.matched_items(..).map(|m| m.data.clone()).collect()
}

pub fn build_coauthor_nucleo(items: &[CoAuthorEntry]) -> Nucleo<CoAuthorEntry> {
    let nucleo = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);
    let injector = nucleo.injector();
    for item in items {
        let display = format!("{} <{}>", item.name, item.email);
        let item = item.clone();
        injector.push(item, move |_, cols| {
            cols[0] = display.clone().into();
        });
    }
    nucleo
}

pub fn apply_coauthor_filter(nucleo: &mut Nucleo<CoAuthorEntry>, query: &str) -> Vec<CoAuthorEntry> {
    nucleo
        .pattern
        .reparse(0, query, CaseMatching::Ignore, Normalization::Smart, false);
    nucleo.tick(10);
    let snap = nucleo.snapshot();
    snap.matched_items(..).map(|m| m.data.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::types::AuthorIdentity;

    fn make_identity(name: &str, email: &str) -> AuthorIdentity {
        AuthorIdentity {
            name: name.to_string(),
            email: email.to_string(),
            commit_count: 1,
        }
    }

    #[test]
    fn test_rename_draft_default_focused_on_name() {
        // RENAME-02: default form state has Name focused and empty strings.
        let draft = RenameDraft::default();
        assert!(matches!(draft.focused, FormField::Name));
        assert!(draft.new_name.is_empty());
        assert!(draft.new_email.is_empty());
    }

    #[test]
    fn test_rename_draft_is_complete_only_when_both_fields_non_empty() {
        // RENAME-02: form submission requires both fields filled.
        let mut draft = RenameDraft::default();
        assert!(!draft.is_complete(), "empty name and email: should not be complete");

        draft.new_name = "Alice".to_string();
        assert!(!draft.is_complete(), "empty email: should not be complete");

        draft.new_name = String::new();
        draft.new_email = "alice@example.com".to_string();
        assert!(!draft.is_complete(), "empty name: should not be complete");

        draft.new_name = "Alice".to_string();
        assert!(draft.is_complete(), "both fields filled: should be complete");
    }

    #[test]
    fn test_form_field_toggle() {
        // FormField::toggle switches between Name and Email.
        assert!(matches!(FormField::Name.toggle(), FormField::Email));
        assert!(matches!(FormField::Email.toggle(), FormField::Name));
    }

    #[test]
    fn test_build_author_nucleo_injects_all_items() {
        // build_author_nucleo injects all items; after tick(10) with empty pattern, all 3 appear.
        let items = vec![
            make_identity("Alice", "alice@example.com"),
            make_identity("Bob", "bob@example.com"),
            make_identity("Carol", "carol@example.com"),
        ];
        let mut nucleo = build_author_nucleo(&items);
        nucleo.tick(10);
        let snap = nucleo.snapshot();
        assert_eq!(snap.matched_item_count(), 3);
    }

    #[test]
    fn test_apply_filter_narrows_results() {
        // apply_filter with "ali" returns only Alice, not Bob or Carol.
        let items = vec![
            make_identity("Alice", "alice@example.com"),
            make_identity("Bob", "bob@example.com"),
            make_identity("Carol", "carol@example.com"),
        ];
        let mut nucleo = build_author_nucleo(&items);
        let matched = apply_filter(&mut nucleo, "ali");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "Alice");
    }

    fn make_coauthor(name: &str, email: &str) -> crate::git::types::CoAuthorEntry {
        crate::git::types::CoAuthorEntry {
            name: name.to_string(),
            email: email.to_string(),
            commit_count: 1,
        }
    }

    #[test]
    fn test_build_coauthor_nucleo_injects_all_items() {
        // DROP-01: build_coauthor_nucleo injects all items; after tick(10) with empty pattern, all 3 appear.
        let items = vec![
            make_coauthor("Alice", "alice@example.com"),
            make_coauthor("Bob", "bob@example.com"),
            make_coauthor("Carol", "carol@example.com"),
        ];
        let mut nucleo = build_coauthor_nucleo(&items);
        nucleo.tick(10);
        let snap = nucleo.snapshot();
        assert_eq!(snap.matched_item_count(), 3);
    }

    #[test]
    fn test_apply_coauthor_filter_narrows_results() {
        // DROP-01: apply_coauthor_filter with "ali" returns only Alice.
        let items = vec![
            make_coauthor("Alice", "alice@example.com"),
            make_coauthor("Bob", "bob@example.com"),
            make_coauthor("Carol", "carol@example.com"),
        ];
        let mut nucleo = build_coauthor_nucleo(&items);
        let matched = apply_coauthor_filter(&mut nucleo, "ali");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "Alice");
    }
}
