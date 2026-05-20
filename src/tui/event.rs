use crate::tui::app::{App, FormField, PendingOp, RenameDraft, Screen};
use crossterm::event::KeyCode;

pub fn handle_key(app: &mut App, key: KeyCode) {
    match &mut app.screen {
        Screen::MainMenu { selected } => match key {
            KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1) % 2,
            KeyCode::Up | KeyCode::Char('k') => *selected = (*selected + 1) % 2,
            KeyCode::Enter => {
                let choice = if *selected == 0 { "rename" } else { "drop" };
                app.screen = Screen::NotImplemented(choice);
            }
            KeyCode::Char('q') | KeyCode::Esc => app.should_exit = true,
            _ => {}
        },
        Screen::NotImplemented(_) => match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.screen = Screen::MainMenu { selected: 0 };
            }
            _ => {}
        },
        // Task 2 (03-03) implements these arms fully
        Screen::AuthorList { .. } | Screen::RenameForm { .. } | Screen::Preview(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::types::AuthorIdentity;
    use crate::tui::app::{build_author_nucleo, apply_filter};
    use tempfile::TempDir;

    fn make_test_app() -> (TempDir, App) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init_bare(dir.path()).unwrap();
        (dir, App::new(repo))
    }

    /// Creates a non-bare repo with one commit by Alice so enumerate_authors returns data.
    fn make_test_app_with_commits() -> (TempDir, App) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Alice", "alice@example.com").unwrap();
        let tree_oid = {
            let mut tb = repo.treebuilder(None).unwrap();
            tb.write().unwrap()
        };
        {
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
        } // drop tree borrow before moving repo
        (dir, App::new(repo))
    }

    fn make_author_list_screen(names: &[&str]) -> Screen {
        let items: Vec<AuthorIdentity> = names
            .iter()
            .map(|n| AuthorIdentity {
                name: n.to_string(),
                email: format!("{}@example.com", n.to_lowercase()),
                commit_count: 1,
            })
            .collect();
        let mut nucleo = build_author_nucleo(&items);
        let matched = apply_filter(&mut nucleo, "");
        Screen::AuthorList {
            items,
            filter: String::new(),
            matched,
            nucleo,
            selected: 0,
        }
    }

    // ---- Existing tests from Plan 03-02 (kept intact) ----

    #[test]
    fn test_main_menu_down_increments_selected_mod_2() {
        // CORE-01: keyboard navigation must cycle the two options.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Down);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 1 }));
        handle_key(&mut app, KeyCode::Down);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_main_menu_up_decrements_with_wrap() {
        // CORE-01: up arrow wraps the two-option list.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Up);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 1 }));
        handle_key(&mut app, KeyCode::Up);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_main_menu_j_k_same_as_down_up() {
        // CORE-01: vim bindings j/k behave identically to Down/Up.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Char('j'));
        assert!(matches!(app.screen, Screen::MainMenu { selected: 1 }));
        handle_key(&mut app, KeyCode::Char('k'));
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_main_menu_enter_with_selected_1_transitions_to_drop_placeholder() {
        // CORE-01 + DROP-01: selecting Drop still goes to NotImplemented (Plan 03-04 replaces).
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Down);
        handle_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::NotImplemented("drop")));
    }

    #[test]
    fn test_main_menu_q_sets_should_exit() {
        // CORE-01: q exits cleanly.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Char('q'));
        assert!(app.should_exit);
    }

    #[test]
    fn test_main_menu_esc_sets_should_exit() {
        // CORE-01: Esc exits cleanly.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Esc);
        assert!(app.should_exit);
    }

    #[test]
    fn test_not_implemented_esc_returns_to_main_menu() {
        // Pressing Esc on the placeholder screen returns to main menu.
        let (_dir, mut app) = make_test_app();
        app.screen = Screen::NotImplemented("rename");
        handle_key(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    // ---- New tests for Plan 03-03 ----

    #[test]
    fn test_main_menu_enter_rename_now_loads_author_list() {
        // RENAME-01: pressing Enter on 'Rename an author' loads the author list.
        let (_dir, mut app) = make_test_app_with_commits();
        handle_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::AuthorList { .. }));
    }

    #[test]
    fn test_author_list_typing_filter_updates_matched() {
        // RENAME-01: typing a character filters the list; filter field and matched update.
        let (_dir, mut app) = make_test_app();
        app.screen = make_author_list_screen(&["Alice", "Bob"]);
        handle_key(&mut app, KeyCode::Char('a'));
        match &app.screen {
            Screen::AuthorList { filter, matched, selected, .. } => {
                assert_eq!(filter, "a");
                assert!(matched.iter().any(|m| m.name == "Alice"), "Alice should match 'a'");
                assert_eq!(*selected, 0, "selection resets on filter change");
            }
            _ => panic!("expected AuthorList"),
        }
    }

    #[test]
    fn test_author_list_backspace_removes_char() {
        // RENAME-01: Backspace removes the last filter character.
        let (_dir, mut app) = make_test_app();
        app.screen = make_author_list_screen(&["Alice", "Bob"]);
        // Set filter to "ali" manually then backspace
        handle_key(&mut app, KeyCode::Char('a'));
        handle_key(&mut app, KeyCode::Char('l'));
        handle_key(&mut app, KeyCode::Char('i'));
        handle_key(&mut app, KeyCode::Backspace);
        match &app.screen {
            Screen::AuthorList { filter, .. } => assert_eq!(filter, "al"),
            _ => panic!("expected AuthorList"),
        }
    }

    #[test]
    fn test_author_list_down_wraps_selection() {
        // RENAME-01: Down wraps from last item back to first.
        let (_dir, mut app) = make_test_app();
        app.screen = make_author_list_screen(&["Alice", "Bob", "Carol"]);
        handle_key(&mut app, KeyCode::Down);
        handle_key(&mut app, KeyCode::Down);
        handle_key(&mut app, KeyCode::Down); // should wrap to 0
        match &app.screen {
            Screen::AuthorList { selected, .. } => assert_eq!(*selected, 0),
            _ => panic!("expected AuthorList"),
        }
    }

    #[test]
    fn test_author_list_enter_transitions_to_rename_form_with_selected_source() {
        // RENAME-02: selecting an author opens the form, not a second list.
        let (_dir, mut app) = make_test_app();
        app.screen = make_author_list_screen(&["Alice", "Bob"]);
        handle_key(&mut app, KeyCode::Enter);
        match &app.screen {
            Screen::RenameForm { source, draft } => {
                assert_eq!(source.name, "Alice");
                assert!(matches!(draft.focused, FormField::Name));
            }
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_author_list_esc_returns_to_main_menu() {
        // RENAME-01: Esc from AuthorList returns to MainMenu.
        let (_dir, mut app) = make_test_app();
        app.screen = make_author_list_screen(&["Alice"]);
        handle_key(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_rename_form_tab_toggles_focused_field() {
        // RENAME-02: Tab switches focus between Name and Email fields.
        let (_dir, mut app) = make_test_app();
        app.screen = Screen::RenameForm {
            source: AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            draft: RenameDraft::default(),
        };
        handle_key(&mut app, KeyCode::Tab);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert!(matches!(draft.focused, FormField::Email)),
            _ => panic!("expected RenameForm"),
        }
        handle_key(&mut app, KeyCode::Tab);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert!(matches!(draft.focused, FormField::Name)),
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_printable_appends_to_focused_field() {
        // RENAME-02: typing a char appends to the currently focused field.
        let (_dir, mut app) = make_test_app();
        app.screen = Screen::RenameForm {
            source: AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            draft: RenameDraft::default(),
        };
        handle_key(&mut app, KeyCode::Char('A'));
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert_eq!(draft.new_name, "A"),
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_backspace_pops_focused_field() {
        // RENAME-02: Backspace removes the last character from the focused field.
        let (_dir, mut app) = make_test_app();
        let mut draft = RenameDraft::default();
        draft.focused = FormField::Email;
        draft.new_email = "alice@x".to_string();
        app.screen = Screen::RenameForm {
            source: AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            draft,
        };
        handle_key(&mut app, KeyCode::Backspace);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert_eq!(draft.new_email, "alice@"),
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_enter_with_incomplete_draft_does_nothing() {
        // RENAME-02: Enter with empty name does not transition.
        let (_dir, mut app) = make_test_app();
        app.screen = Screen::RenameForm {
            source: AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            draft: RenameDraft::default(), // both fields empty
        };
        handle_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::RenameForm { .. }));
    }

    #[test]
    fn test_rename_form_enter_with_complete_draft_transitions_to_preview() {
        // RENAME-02: Enter with both fields filled transitions to Preview.
        let (_dir, mut app) = make_test_app();
        let mut draft = RenameDraft::default();
        draft.new_name = "Bob".to_string();
        draft.new_email = "bob@example.com".to_string();
        app.screen = Screen::RenameForm {
            source: AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            draft,
        };
        handle_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::Preview(PendingOp::Rename { .. })));
    }

    #[test]
    fn test_rename_form_esc_returns_to_main_menu_v1() {
        // v1: no back-stack — Esc from RenameForm goes to MainMenu (not AuthorList).
        // Wave 4/5 may add a back-stack (out of scope here).
        let (_dir, mut app) = make_test_app();
        app.screen = Screen::RenameForm {
            source: AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            draft: RenameDraft::default(),
        };
        handle_key(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }
}
