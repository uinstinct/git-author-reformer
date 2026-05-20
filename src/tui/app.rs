use git2::Repository;

pub struct App {
    pub repo: Repository,
    pub screen: Screen,
    pub should_exit: bool,
}

pub enum Screen {
    MainMenu { selected: usize },
    NotImplemented(&'static str),
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
}
