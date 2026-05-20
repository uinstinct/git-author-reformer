use crate::tui::app::{apply_coauthor_filter, apply_filter, build_author_nucleo, build_coauthor_nucleo, App, FormField, PendingOp, RenameDraft, Screen};
use crossterm::event::KeyCode;

pub fn handle_key(app: &mut App, key: KeyCode) {
    match &mut app.screen {
        Screen::MainMenu { selected } => match key {
            KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1) % 2,
            KeyCode::Up | KeyCode::Char('k') => *selected = (*selected + 1) % 2,
            KeyCode::Enter => {
                if *selected == 0 {
                    // Rename — load authors
                    match crate::git::reader::enumerate_authors(&app.repo) {
                        Ok(items) => {
                            let mut nucleo = build_author_nucleo(&items);
                            let matched = apply_filter(&mut nucleo, "");
                            app.screen = Screen::AuthorList {
                                items,
                                filter: String::new(),
                                matched,
                                nucleo,
                                selected: 0,
                            };
                        }
                        Err(_e) => {
                            // TODO Plan 03-05: proper error screen
                            app.screen = Screen::NotImplemented("error");
                        }
                    }
                } else {
                    // Drop — load co-authors
                    match crate::git::reader::enumerate_coauthors(&app.repo) {
                        Ok(items) => {
                            let mut nucleo = build_coauthor_nucleo(&items);
                            let matched = apply_coauthor_filter(&mut nucleo, "");
                            app.screen = Screen::CoAuthorList {
                                items,
                                filter: String::new(),
                                matched,
                                nucleo,
                                selected: 0,
                            };
                        }
                        Err(_) => {
                            // TODO Plan 03-05: proper error screen
                            app.screen = Screen::NotImplemented("error");
                        }
                    }
                }
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
        Screen::AuthorList {
            items: _,
            filter,
            matched,
            nucleo,
            selected,
        } => match key {
            KeyCode::Esc => app.screen = Screen::MainMenu { selected: 0 },
            KeyCode::Down => {
                if !matched.is_empty() {
                    *selected = (*selected + 1) % matched.len();
                }
            }
            KeyCode::Up => {
                if !matched.is_empty() {
                    *selected = (*selected + matched.len() - 1) % matched.len();
                }
            }
            KeyCode::Enter => {
                if let Some(src) = matched.get(*selected).cloned() {
                    app.screen = Screen::RenameForm {
                        source: src,
                        draft: RenameDraft::default(),
                    };
                }
            }
            KeyCode::Backspace => {
                filter.pop();
                *matched = apply_filter(nucleo, filter);
                *selected = 0;
            }
            KeyCode::Char(c) => {
                filter.push(c);
                *matched = apply_filter(nucleo, filter);
                *selected = 0;
            }
            _ => {}
        },
        Screen::RenameForm { source, draft } => match key {
            KeyCode::Esc => app.screen = Screen::MainMenu { selected: 0 }, // v1: no back-stack
            KeyCode::Tab | KeyCode::BackTab => {
                let toggled = draft.focused.clone().toggle();
                draft.focused = toggled;
            }
            KeyCode::Enter => {
                if draft.is_complete() {
                    // Detach source from borrow before reassigning app.screen
                    let source = source.clone();
                    let new_name = std::mem::take(&mut draft.new_name);
                    let new_email = std::mem::take(&mut draft.new_email);
                    let op = PendingOp::Rename {
                        source,
                        new_name: new_name.trim().to_string(),
                        new_email: new_email.trim().to_string(),
                    };
                    app.screen = Screen::Preview(op);
                }
            }
            KeyCode::Backspace => match draft.focused {
                FormField::Name => {
                    draft.new_name.pop();
                }
                FormField::Email => {
                    draft.new_email.pop();
                }
            },
            KeyCode::Char(c) => match draft.focused {
                FormField::Name => draft.new_name.push(c),
                FormField::Email => draft.new_email.push(c),
            },
            _ => {}
        },
        Screen::Preview(_) => match key {
            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::MainMenu { selected: 0 },
            _ => {}
        },
        Screen::CoAuthorList {
            items: _,
            filter,
            matched,
            nucleo,
            selected,
        } => match key {
            KeyCode::Esc => app.screen = Screen::MainMenu { selected: 0 },
            KeyCode::Down => {
                if !matched.is_empty() {
                    *selected = (*selected + 1) % matched.len();
                }
            }
            KeyCode::Up => {
                if !matched.is_empty() {
                    *selected = (*selected + matched.len() - 1) % matched.len();
                }
            }
            KeyCode::Enter => {
                if let Some(target) = matched.get(*selected).cloned() {
                    app.screen = Screen::Preview(PendingOp::Drop { target });
                }
            }
            KeyCode::Backspace => {
                filter.pop();
                *matched = apply_coauthor_filter(nucleo, filter);
                *selected = 0;
            }
            KeyCode::Char(c) => {
                filter.push(c);
                *matched = apply_coauthor_filter(nucleo, filter);
                *selected = 0;
            }
            _ => {}
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::types::AuthorIdentity;
    use crate::tui::app::{apply_filter, build_author_nucleo};
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
    fn test_main_menu_enter_with_selected_1_bare_repo_goes_to_error() {
        // DROP-01: bare repo has no refs so enumerate_coauthors returns Ok([]).
        // The screen transitions to CoAuthorList with empty items (not NotImplemented).
        let (_dir, mut app) = make_test_app(); // bare repo — no commits
        handle_key(&mut app, KeyCode::Down);
        handle_key(&mut app, KeyCode::Enter);
        // Bare repo with no refs: enumerate_coauthors returns Ok(vec![]) -> CoAuthorList
        assert!(matches!(app.screen, Screen::CoAuthorList { .. }));
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

    // ---- New tests for Plan 03-04 (DROP-01) ----

    /// Creates a non-bare repo with one commit that has a Co-authored-by trailer.
    fn make_test_app_with_coauthors() -> (TempDir, App) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Alice", "alice@example.com").unwrap();
        let tree_oid = {
            let mut tb = repo.treebuilder(None).unwrap();
            tb.write().unwrap()
        };
        {
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "initial\n\nCo-authored-by: Bob <bob@example.com>",
                &tree,
                &[],
            )
            .unwrap();
        }
        (dir, App::new(repo))
    }

    fn make_coauthor_list_screen(names: &[(&str, &str)]) -> Screen {
        use crate::git::types::CoAuthorEntry;
        let items: Vec<CoAuthorEntry> = names
            .iter()
            .map(|(n, e)| CoAuthorEntry {
                name: n.to_string(),
                email: e.to_string(),
                commit_count: 1,
            })
            .collect();
        let mut nucleo = build_coauthor_nucleo(&items);
        let matched = apply_coauthor_filter(&mut nucleo, "");
        Screen::CoAuthorList {
            items,
            filter: String::new(),
            matched,
            nucleo,
            selected: 0,
        }
    }

    #[test]
    fn test_main_menu_enter_drop_now_loads_coauthor_list() {
        // DROP-01: pressing Enter on 'Drop a co-author' loads the co-author list.
        let (_dir, mut app) = make_test_app_with_coauthors();
        // Navigate to "Drop a co-author" (index 1)
        handle_key(&mut app, KeyCode::Down);
        handle_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::CoAuthorList { .. }));
    }

    #[test]
    fn test_coauthor_list_typing_updates_matched() {
        // DROP-01: typing a character filters the co-author list.
        let (_dir, mut app) = make_test_app();
        app.screen = make_coauthor_list_screen(&[("Alice", "alice@x"), ("Bob", "bob@x")]);
        handle_key(&mut app, KeyCode::Char('b'));
        match &app.screen {
            Screen::CoAuthorList { filter, matched, selected, .. } => {
                assert_eq!(filter, "b");
                assert!(matched.iter().any(|m| m.name == "Bob"), "Bob should match 'b'");
                assert_eq!(*selected, 0, "selection resets on filter change");
            }
            _ => panic!("expected CoAuthorList"),
        }
    }

    #[test]
    fn test_coauthor_list_enter_transitions_to_preview_drop() {
        // DROP-01: selecting a co-author transitions to Screen::Preview(PendingOp::Drop).
        let (_dir, mut app) = make_test_app();
        app.screen = make_coauthor_list_screen(&[("Bob", "bob@x")]);
        handle_key(&mut app, KeyCode::Enter);
        match &app.screen {
            Screen::Preview(PendingOp::Drop { target }) => {
                assert_eq!(target.name, "Bob");
                assert_eq!(target.email, "bob@x");
            }
            _ => panic!("expected Preview(PendingOp::Drop)"),
        }
    }

    #[test]
    fn test_coauthor_list_esc_returns_to_main_menu() {
        // DROP-01: Esc from CoAuthorList returns to MainMenu.
        let (_dir, mut app) = make_test_app();
        app.screen = make_coauthor_list_screen(&[("Bob", "bob@x")]);
        handle_key(&mut app, KeyCode::Esc);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_coauthor_list_empty_repo_still_safe() {
        // DROP-01: empty co-author list — Enter does nothing (no panic on empty index).
        let (_dir, mut app) = make_test_app();
        app.screen = make_coauthor_list_screen(&[]);
        handle_key(&mut app, KeyCode::Enter); // must not panic
        assert!(matches!(app.screen, Screen::CoAuthorList { .. }));
    }
}
