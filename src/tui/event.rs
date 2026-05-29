use crate::tui::app::{
    apply_coauthor_filter, apply_filter, apply_strip_filter, build_author_nucleo,
    build_coauthor_nucleo, build_strip_nucleo, App, FormField, MenuChoice, PendingOp, RenameDraft,
    Screen,
};
use crossterm::event::{KeyCode, KeyModifiers};

/// Apply a backspace edit to a text buffer, honoring modifier keys:
/// Cmd (SUPER) clears the whole line, Option (ALT) deletes one word, otherwise
/// a single character is removed.
fn backspace_edit(s: &mut String, mods: KeyModifiers) {
    if mods.contains(KeyModifiers::SUPER) {
        s.clear();
    } else if mods.contains(KeyModifiers::ALT) {
        let trimmed = s.trim_end_matches(|c: char| c.is_whitespace());
        let cut = trimmed.trim_end_matches(|c: char| !c.is_whitespace());
        s.truncate(cut.len());
    } else {
        s.pop();
    }
}

fn base64_encode(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = *chunk.get(1).unwrap_or(&0) as usize;
        let b2 = *chunk.get(2).unwrap_or(&0) as usize;
        out.push(T[b0 >> 2] as char);
        out.push(T[((b0 & 3) << 4) | (b1 >> 4)] as char);
        out.push(if chunk.len() > 1 {
            T[((b1 & 15) << 2) | (b2 >> 6)] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[b2 & 63] as char
        } else {
            '='
        });
    }
    out
}

fn copy_via_osc52(text: &str) {
    use std::io::Write;
    let seq = format!("\x1b]52;c;{}\x07", base64_encode(text.as_bytes()));
    let _ = std::io::stdout().write_all(seq.as_bytes());
    let _ = std::io::stdout().flush();
}

pub fn handle_key(app: &mut App, key: KeyCode, mods: KeyModifiers) {
    // Universal readline bindings: Ctrl+U clears the line, Ctrl+W deletes the
    // last word. Cmd+Backspace / Option+Backspace only deliver SUPER/ALT in
    // terminals that support the keyboard enhancement protocol, but Ctrl+U/W
    // arrive as control chars in every terminal. Translate them to the
    // equivalent modified Backspace so every text field handles them through
    // the existing backspace_edit path.
    let (key, mods) = match key {
        KeyCode::Char('u') | KeyCode::Char('U') if mods.contains(KeyModifiers::CONTROL) => {
            (KeyCode::Backspace, KeyModifiers::SUPER)
        }
        KeyCode::Char('w') | KeyCode::Char('W') if mods.contains(KeyModifiers::CONTROL) => {
            (KeyCode::Backspace, KeyModifiers::ALT)
        }
        _ => (key, mods),
    };
    match &mut app.screen {
        Screen::MainMenu { selected } => match key {
            KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1) % 4,
            KeyCode::Up | KeyCode::Char('k') => *selected = (*selected + 4 - 1) % 4,
            KeyCode::Enter => {
                let sel = *selected;
                match MenuChoice::from_index(sel) {
                    MenuChoice::Rename => {
                        // Rename — preflight then load authors
                        if let Err(e) = crate::git::preflight::check_stash(&app.repo) {
                            app.screen = Screen::Err(e.to_string());
                            return;
                        }
                        if let Err(e) = crate::git::preflight::check_worktrees(&app.repo) {
                            app.screen = Screen::Err(e.to_string());
                            return;
                        }
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
                            Err(e) => {
                                app.screen = Screen::Err(e.to_string());
                            }
                        }
                    }
                    MenuChoice::Drop => {
                        // Drop — preflight then load co-authors
                        if let Err(e) = crate::git::preflight::check_stash(&app.repo) {
                            app.screen = Screen::Err(e.to_string());
                            return;
                        }
                        if let Err(e) = crate::git::preflight::check_worktrees(&app.repo) {
                            app.screen = Screen::Err(e.to_string());
                            return;
                        }
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
                            Err(e) => {
                                app.screen = Screen::Err(e.to_string());
                            }
                        }
                    }
                    MenuChoice::AddHook => {
                        // Read current strip list (no preflight — hook install bypasses SAFE-01/02)
                        let current_strip = match crate::hook::read_strip_list(&app.repo) {
                            Ok(crate::hook::HookState::Absent) => vec![],
                            Ok(crate::hook::HookState::Managed { emails }) => emails,
                            Ok(crate::hook::HookState::NotToolManaged(p)) => {
                                app.screen = Screen::Err(format!(
                                    "Foreign hook at {} — remove or rename it first.",
                                    p.display()
                                ));
                                return;
                            }
                            Err(e) => {
                                app.screen = Screen::Err(e.to_string());
                                return;
                            }
                        };
                        match crate::git::reader::enumerate_coauthors(&app.repo) {
                            Ok(items) => {
                                let mut nucleo = build_coauthor_nucleo(&items);
                                let matched = apply_coauthor_filter(&mut nucleo, "");
                                app.screen = Screen::HookAddList {
                                    current_strip,
                                    items,
                                    filter: String::new(),
                                    matched,
                                    nucleo,
                                    selected: 0,
                                };
                            }
                            Err(e) => {
                                app.screen = Screen::Err(e.to_string());
                            }
                        }
                    }
                    MenuChoice::ManageHook => match crate::hook::read_strip_list(&app.repo) {
                        Ok(crate::hook::HookState::Absent) => {
                            app.screen = Screen::HookSuccess {
                                state: crate::hook::HookState::Absent,
                            };
                        }
                        Ok(crate::hook::HookState::Managed { emails }) => {
                            let mut nucleo = build_strip_nucleo(&emails);
                            let matched = apply_strip_filter(&mut nucleo, "");
                            app.screen = Screen::HookManageList {
                                items: emails,
                                filter: String::new(),
                                matched,
                                nucleo,
                                selected: 0,
                            };
                        }
                        Ok(crate::hook::HookState::NotToolManaged(p)) => {
                            app.screen = Screen::Err(format!(
                                "Foreign hook at {} — remove or rename it first.",
                                p.display()
                            ));
                        }
                        Err(e) => {
                            app.screen = Screen::Err(e.to_string());
                        }
                    },
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => app.should_exit = true,
            _ => {}
        },
        Screen::AuthorList {
            items,
            filter,
            matched,
            nucleo,
            selected,
        } => match key {
            KeyCode::Esc => app.screen = Screen::MainMenu { selected: 0 },
            KeyCode::Down if !matched.is_empty() => {
                *selected = (*selected + 1) % matched.len();
            }
            KeyCode::Up if !matched.is_empty() => {
                *selected = (*selected + matched.len() - 1) % matched.len();
            }
            KeyCode::Enter => {
                if let Some(src) = matched.get(*selected).cloned() {
                    let rest: Vec<crate::git::types::AuthorIdentity> =
                        items.iter().filter(|a| **a != src).cloned().collect();
                    let mut rename_nucleo = build_author_nucleo(&rest);
                    let matched_list = apply_filter(&mut rename_nucleo, "");
                    app.screen = Screen::RenameForm {
                        source: src,
                        draft: RenameDraft::default(),
                        items: rest,
                        filter: String::new(),
                        matched: matched_list,
                        nucleo: rename_nucleo,
                        selected: 0,
                    };
                }
            }
            KeyCode::Backspace => {
                backspace_edit(filter, mods);
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
        Screen::RenameForm {
            source,
            draft,
            filter,
            matched,
            nucleo,
            selected,
            ..
        } => match key {
            KeyCode::Esc => app.screen = Screen::MainMenu { selected: 0 }, // v1: no back-stack
            KeyCode::Tab => {
                draft.focused = draft.focused.clone().toggle();
            }
            KeyCode::BackTab => {
                draft.focused = draft.focused.clone().toggle_back();
            }
            KeyCode::Enter => {
                // List focused: picking the highlighted author fills the draft so a single
                // Enter selects it and submits — no Tab into a field required.
                if matches!(draft.focused, FormField::List) {
                    if let Some(a) = matched.get(*selected) {
                        draft.new_name = a.name.clone();
                        draft.new_email = a.email.clone();
                    }
                }
                if !draft.is_complete() {
                    // Nothing to submit yet (empty list pick or partial manual entry).
                    return;
                }
                // Clone source before the borrow ends so we can assign app.screen
                let source_clone = source.clone();
                let new_name = draft.new_name.trim().to_string();
                let new_email = draft.new_email.trim().to_string();
                let old_name = source_clone.name.clone();
                let old_email = source_clone.email.clone();
                // Drop the borrow of app.screen by dropping source/draft
                match crate::git::scan::scan_rename(&app.repo, &old_name, &old_email) {
                    Ok(scan) => {
                        let op = PendingOp::Rename {
                            source: source_clone,
                            new_name,
                            new_email,
                        };
                        app.screen = Screen::Preview { op, scan };
                    }
                    Err(e) => app.screen = Screen::Err(e.to_string()),
                }
            }
            KeyCode::Up if matches!(draft.focused, FormField::List) && !matched.is_empty() => {
                *selected = (*selected + matched.len() - 1) % matched.len();
            }
            KeyCode::Down if matches!(draft.focused, FormField::List) && !matched.is_empty() => {
                *selected = (*selected + 1) % matched.len();
            }
            KeyCode::Backspace => match draft.focused {
                FormField::Name => backspace_edit(&mut draft.new_name, mods),
                FormField::Email => backspace_edit(&mut draft.new_email, mods),
                FormField::List => {
                    backspace_edit(filter, mods);
                    *matched = apply_filter(nucleo, filter);
                    *selected = 0;
                }
            },
            KeyCode::Char(c) => match draft.focused {
                FormField::Name => draft.new_name.push(c),
                FormField::Email => draft.new_email.push(c),
                FormField::List => {
                    filter.push(c);
                    *matched = apply_filter(nucleo, filter);
                    *selected = 0;
                }
            },
            _ => {}
        },
        Screen::Preview { op, scan } => {
            // Clone the data we need before reassigning app.screen (borrow-checker: NLL)
            let op_clone = op.clone();
            let remote_name = scan.remote_name.clone();
            match key {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let result = match &op_clone {
                        PendingOp::Rename {
                            source,
                            new_name,
                            new_email,
                        } => crate::git::rewrite::rewrite_author(
                            &app.repo,
                            &source.name,
                            &source.email,
                            new_name,
                            new_email,
                        ),
                        PendingOp::Drop { target } => {
                            crate::git::rewrite::drop_coauthor(&app.repo, &target.email)
                        }
                    };
                    match result {
                        Ok(rewritten) => {
                            app.screen = Screen::Success {
                                rewritten,
                                remote_name,
                                copied: false,
                            };
                        }
                        Err(e) => app.screen = Screen::Err(e.to_string()),
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.screen = Screen::MainMenu { selected: 0 };
                }
                _ => {}
            }
        }
        Screen::CoAuthorList {
            items: _,
            filter,
            matched,
            nucleo,
            selected,
        } => match key {
            KeyCode::Esc => app.screen = Screen::MainMenu { selected: 0 },
            KeyCode::Down if !matched.is_empty() => {
                *selected = (*selected + 1) % matched.len();
            }
            KeyCode::Up if !matched.is_empty() => {
                *selected = (*selected + matched.len() - 1) % matched.len();
            }
            KeyCode::Enter => {
                if let Some(target) = matched.get(*selected).cloned() {
                    let target_email = target.email.clone();
                    match crate::git::scan::scan_drop(&app.repo, &target_email) {
                        Ok(scan) => {
                            let op = PendingOp::Drop { target };
                            app.screen = Screen::Preview { op, scan };
                        }
                        Err(e) => app.screen = Screen::Err(e.to_string()),
                    }
                }
            }
            KeyCode::Backspace => {
                backspace_edit(filter, mods);
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
        Screen::Success {
            remote_name,
            copied,
            ..
        } => match key {
            KeyCode::Char('c') => {
                let remote = remote_name.as_deref().unwrap_or("<remote>");
                copy_via_osc52(&format!("git push --force-with-lease --all {}", remote));
                *copied = true;
            }
            _ => app.should_exit = true,
        },
        Screen::HookAddList {
            filter,
            matched,
            nucleo,
            selected,
            ..
        } => match key {
            KeyCode::Enter => {
                // NLL pattern: clone email out before reassigning app.screen
                let email = matched.get(*selected).map(|t| t.email.clone());
                if let Some(email) = email {
                    match crate::hook::install_strip(&app.repo, &email) {
                        Ok(crate::hook::AddResult::Installed { .. }) => {
                            match crate::hook::read_strip_list(&app.repo) {
                                Ok(state) => app.screen = Screen::HookSuccess { state },
                                Err(e) => app.screen = Screen::Err(e.to_string()),
                            }
                        }
                        Ok(crate::hook::AddResult::AlreadyStripped) => {
                            app.screen = Screen::HookAlreadyStripped { email };
                        }
                        Err(e) => app.screen = Screen::Err(e.to_string()),
                    }
                }
            }
            KeyCode::Char(c) => {
                filter.push(c);
                *matched = apply_coauthor_filter(nucleo, filter);
                *selected = 0;
            }
            KeyCode::Backspace => {
                backspace_edit(filter, mods);
                *matched = apply_coauthor_filter(nucleo, filter);
                *selected = 0;
            }
            KeyCode::Down if !matched.is_empty() => {
                *selected = (*selected + 1) % matched.len();
            }
            KeyCode::Up if !matched.is_empty() => {
                *selected = (*selected + matched.len() - 1) % matched.len();
            }
            KeyCode::Esc => app.screen = Screen::MainMenu { selected: 2 },
            _ => {}
        },
        Screen::HookManageList {
            filter,
            matched,
            nucleo,
            selected,
            ..
        } => match key {
            KeyCode::Enter => {
                // NLL pattern: clone email out before reassigning app.screen
                let email = matched.get(*selected).cloned();
                if let Some(email) = email {
                    match crate::hook::remove_strip(&app.repo, &email) {
                        Ok(crate::hook::RemoveResult::Updated { .. }) => {
                            match crate::hook::read_strip_list(&app.repo) {
                                Ok(state) => app.screen = Screen::HookSuccess { state },
                                Err(e) => app.screen = Screen::Err(e.to_string()),
                            }
                        }
                        Ok(crate::hook::RemoveResult::HookDeleted) => {
                            app.screen = Screen::HookRemoved;
                        }
                        Ok(crate::hook::RemoveResult::NotFound) => {
                            app.screen =
                                Screen::Err("email not found in strip list (unexpected)".into());
                        }
                        Err(e) => {
                            app.screen = Screen::Err(e.to_string());
                        }
                    }
                }
            }
            KeyCode::Char(c) => {
                filter.push(c);
                *matched = apply_strip_filter(nucleo, filter);
                *selected = 0;
            }
            KeyCode::Backspace => {
                backspace_edit(filter, mods);
                *matched = apply_strip_filter(nucleo, filter);
                *selected = 0;
            }
            KeyCode::Down if !matched.is_empty() => {
                *selected = (*selected + 1) % matched.len();
            }
            KeyCode::Up if !matched.is_empty() => {
                *selected = (*selected + matched.len() - 1) % matched.len();
            }
            KeyCode::Esc => app.screen = Screen::MainMenu { selected: 3 },
            _ => {}
        },
        Screen::HookSuccess { .. } => {
            app.should_exit = true;
        }
        Screen::HookAlreadyStripped { .. } => {
            app.screen = Screen::MainMenu { selected: 2 };
        }
        Screen::HookRemoved => {
            app.should_exit = true;
        }
        Screen::Err(_) => {
            app.should_exit = true;
        }
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
            let tb = repo.treebuilder(None).unwrap();
            tb.write().unwrap()
        };
        {
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
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

    /// Builds a widened Screen::RenameForm for tests.
    /// `source` is the author being renamed; `others` is a slice of (name, email) pairs
    /// for the embedded list (already excluded from source by the caller or transition).
    fn make_rename_form_screen(source: AuthorIdentity, others: &[(&str, &str)]) -> Screen {
        let items: Vec<AuthorIdentity> = others
            .iter()
            .map(|(n, e)| AuthorIdentity {
                name: n.to_string(),
                email: e.to_string(),
                commit_count: 1,
            })
            .collect();
        let mut nucleo = build_author_nucleo(&items);
        let matched = apply_filter(&mut nucleo, "");
        Screen::RenameForm {
            source,
            draft: RenameDraft::default(),
            items,
            filter: String::new(),
            matched,
            nucleo,
            selected: 0,
        }
    }

    // ---- Existing tests from Plan 03-02 (kept intact) ----

    #[test]
    fn test_main_menu_down_increments_selected() {
        // CORE-01: keyboard navigation must cycle four options; Down from 3 wraps to 0.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 1 }));
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 2 }));
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 3 }));
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_main_menu_up_decrements_with_wrap() {
        // CORE-01: up arrow wraps the four-option list; Up from 0 wraps to 3.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Up, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 3 }));
        handle_key(&mut app, KeyCode::Up, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 2 }));
    }

    #[test]
    fn test_main_menu_j_k_same_as_down_up() {
        // CORE-01: vim bindings j/k behave identically to Down/Up.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 1 }));
        handle_key(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_main_menu_shows_four_options() {
        // HOOK-01/HOOK-02: main menu must offer four choices.
        use crate::tui::app::MenuChoice;
        assert_eq!(MenuChoice::all().len(), 4);
    }

    #[test]
    fn test_main_menu_enter_with_selected_1_bare_repo_goes_to_error() {
        // DROP-01: bare repo has no refs so enumerate_coauthors returns Ok([]).
        // The screen transitions to CoAuthorList with empty items (not NotImplemented).
        let (_dir, mut app) = make_test_app(); // bare repo — no commits
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        // Bare repo with no refs: enumerate_coauthors returns Ok(vec![]) -> CoAuthorList
        assert!(matches!(app.screen, Screen::CoAuthorList { .. }));
    }

    #[test]
    fn test_main_menu_q_sets_should_exit() {
        // CORE-01: q exits cleanly.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(app.should_exit);
    }

    #[test]
    fn test_main_menu_esc_sets_should_exit() {
        // CORE-01: Esc exits cleanly.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.should_exit);
    }

    // ---- New tests for Plan 03-03 ----

    #[test]
    fn test_main_menu_enter_rename_now_loads_author_list() {
        // RENAME-01: pressing Enter on 'Rename an author' loads the author list.
        let (_dir, mut app) = make_test_app_with_commits();
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::AuthorList { .. }));
    }

    #[test]
    fn test_author_list_typing_filter_updates_matched() {
        // RENAME-01: typing a character filters the list; filter field and matched update.
        let (_dir, mut app) = make_test_app();
        app.screen = make_author_list_screen(&["Alice", "Bob"]);
        handle_key(&mut app, KeyCode::Char('a'), KeyModifiers::NONE);
        match &app.screen {
            Screen::AuthorList {
                filter,
                matched,
                selected,
                ..
            } => {
                assert_eq!(filter, "a");
                assert!(
                    matched.iter().any(|m| m.name == "Alice"),
                    "Alice should match 'a'"
                );
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
        handle_key(&mut app, KeyCode::Char('a'), KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Char('l'), KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
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
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // should wrap to 0
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
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { source, draft, .. } => {
                assert_eq!(source.name, "Alice");
                assert!(matches!(draft.focused, FormField::List));
            }
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_author_list_esc_returns_to_main_menu() {
        // RENAME-01: Esc from AuthorList returns to MainMenu.
        let (_dir, mut app) = make_test_app();
        app.screen = make_author_list_screen(&["Alice"]);
        handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_rename_form_tab_toggles_focused_field() {
        // RENAME-02: Tab cycles Name -> Email -> List -> Name (3-way).
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[],
        );
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::Name;
        }
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert!(matches!(draft.focused, FormField::Email)),
            _ => panic!("expected RenameForm"),
        }
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert!(matches!(draft.focused, FormField::List)),
            _ => panic!("expected RenameForm"),
        }
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert!(matches!(draft.focused, FormField::Name)),
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_printable_appends_to_focused_field() {
        // RENAME-02: typing a char appends to the currently focused field.
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[],
        );
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::Name;
        }
        handle_key(&mut app, KeyCode::Char('A'), KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert_eq!(draft.new_name, "A"),
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_backspace_pops_focused_field() {
        // RENAME-02: Backspace removes the last character from the focused field.
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[],
        );
        // Set Email focus and pre-fill email field.
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::Email;
            draft.new_email = "alice@x".to_string();
        }
        handle_key(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert_eq!(draft.new_email, "alice@"),
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_ctrl_u_clears_focused_field() {
        // Ctrl+U is the universal "clear line" binding and must work in every
        // terminal, unlike Cmd+Backspace which needs the enhancement protocol.
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[],
        );
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::Name;
            draft.new_name = "Alice Smith".to_string();
        }
        handle_key(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert_eq!(draft.new_name, ""),
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_ctrl_w_deletes_last_word() {
        // Ctrl+W is the universal "delete word" binding.
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[],
        );
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::Name;
            draft.new_name = "Alice Smith".to_string();
        }
        handle_key(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert_eq!(draft.new_name, "Alice "),
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_enter_with_incomplete_draft_does_nothing() {
        // RENAME-02: Enter with empty name does not transition.
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[],
        );
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::RenameForm { .. }));
    }

    #[test]
    fn test_rename_form_enter_with_complete_draft_transitions_to_preview() {
        // RENAME-05: Enter with both fields filled transitions to Preview with scan data.
        // Uses a bare repo so scan_rename returns Ok(RewritePreview { affected_count: 0, .. }).
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[],
        );
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::Name;
            draft.new_name = "Bob".to_string();
            draft.new_email = "bob@example.com".to_string();
        }
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(
            app.screen,
            Screen::Preview {
                op: PendingOp::Rename { .. },
                ..
            }
        ));
    }

    #[test]
    fn test_rename_form_esc_returns_to_main_menu_v1() {
        // v1: no back-stack — Esc from RenameForm goes to MainMenu (not AuthorList).
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[],
        );
        handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    // ---- Tests for Plan 06-01 (HOOK-12 preflight move) ----

    /// Creates a non-bare repo with one commit and a fake refs/stash reference so
    /// check_stash returns Err. The OID pointed at by the ref does not matter.
    fn make_test_app_with_stash() -> (TempDir, App) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Alice", "alice@example.com").unwrap();
        let tree_oid = {
            let tb = repo.treebuilder(None).unwrap();
            tb.write().unwrap()
        };
        let head_oid = {
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap()
        };
        // Create fake stash ref pointing at the commit OID.
        repo.reference("refs/stash", head_oid, false, "stash")
            .unwrap();
        (dir, App::new(repo))
    }

    #[test]
    fn test_rename_with_stash_repo_hits_preflight_err() {
        // HOOK-12: Rename flow must call check_stash; stash repo must land on Screen::Err.
        let (_dir, mut app) = make_test_app_with_stash();
        // selected 0 = Rename; no navigation needed
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(app.screen, Screen::Err(_)),
            "expected Screen::Err for Rename on stash repo, got something else"
        );
    }

    #[test]
    fn test_drop_with_stash_repo_hits_preflight_err() {
        // HOOK-12: Drop flow must call check_stash; stash repo must land on Screen::Err.
        let (_dir, mut app) = make_test_app_with_stash();
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // select Drop (index 1)
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(app.screen, Screen::Err(_)),
            "expected Screen::Err for Drop on stash repo, got something else"
        );
    }

    #[test]
    fn test_rename_without_stash_repo_loads_author_list() {
        // HOOK-12 regression: clean repo (no stash) still reaches AuthorList on Rename.
        let (_dir, mut app) = make_test_app_with_commits();
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(app.screen, Screen::AuthorList { .. }),
            "expected AuthorList for Rename on clean repo"
        );
    }

    // ---- New tests for Plan 03-04 (DROP-01) ----

    /// Creates a non-bare repo with one commit that has a Co-authored-by trailer.
    fn make_test_app_with_coauthors() -> (TempDir, App) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Alice", "alice@example.com").unwrap();
        let tree_oid = {
            let tb = repo.treebuilder(None).unwrap();
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
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::CoAuthorList { .. }));
    }

    #[test]
    fn test_coauthor_list_typing_updates_matched() {
        // DROP-01: typing a character filters the co-author list.
        let (_dir, mut app) = make_test_app();
        app.screen = make_coauthor_list_screen(&[("Alice", "alice@x"), ("Bob", "bob@x")]);
        handle_key(&mut app, KeyCode::Char('b'), KeyModifiers::NONE);
        match &app.screen {
            Screen::CoAuthorList {
                filter,
                matched,
                selected,
                ..
            } => {
                assert_eq!(filter, "b");
                assert!(
                    matched.iter().any(|m| m.name == "Bob"),
                    "Bob should match 'b'"
                );
                assert_eq!(*selected, 0, "selection resets on filter change");
            }
            _ => panic!("expected CoAuthorList"),
        }
    }

    #[test]
    fn test_coauthor_list_enter_transitions_to_preview_drop() {
        // DROP-01: selecting a co-author transitions to Screen::Preview { op: Drop, .. }.
        // Uses a bare repo so scan_drop returns Ok(RewritePreview { affected_count: 0, .. }).
        let (_dir, mut app) = make_test_app();
        app.screen = make_coauthor_list_screen(&[("Bob", "bob@x")]);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::Preview {
                op: PendingOp::Drop { target },
                ..
            } => {
                assert_eq!(target.name, "Bob");
                assert_eq!(target.email, "bob@x");
            }
            _ => panic!("expected Preview {{ op: PendingOp::Drop, .. }}"),
        }
    }

    #[test]
    fn test_coauthor_list_esc_returns_to_main_menu() {
        // DROP-01: Esc from CoAuthorList returns to MainMenu.
        let (_dir, mut app) = make_test_app();
        app.screen = make_coauthor_list_screen(&[("Bob", "bob@x")]);
        handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_coauthor_list_empty_repo_still_safe() {
        // DROP-01: empty co-author list — Enter does nothing (no panic on empty index).
        let (_dir, mut app) = make_test_app();
        app.screen = make_coauthor_list_screen(&[]);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // must not panic
        assert!(matches!(app.screen, Screen::CoAuthorList { .. }));
    }

    // ---- New tests for Plan 03-05 Task 2 ----

    #[test]
    fn test_rename_form_enter_calls_scan_and_transitions_to_preview_with_data() {
        // RENAME-05: Enter on complete form calls scan_rename and stores RewritePreview in Preview.
        let (_dir, mut app) = make_test_app_with_commits();
        // First navigate to author list
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        // Select Alice (first entry) and press Enter
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        // List is focused first; Tab to the Name field before typing.
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        // Fill in rename form
        for c in "NewAlice".chars() {
            handle_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        for c in "newalice@example.com".chars() {
            handle_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        // Should be in Preview with real scan data
        match &app.screen {
            Screen::Preview {
                op: PendingOp::Rename { source, .. },
                scan,
            } => {
                assert_eq!(source.name, "Alice");
                // scan_rename returns a real RewritePreview (affected_count >= 1 for one-commit repo)
                assert!(
                    scan.affected_count >= 1,
                    "scan.affected_count must reflect actual commits"
                );
            }
            _ => panic!(
                "expected Preview {{ op: Rename, scan }}, got: {:?}",
                match &app.screen {
                    Screen::MainMenu { .. } => "MainMenu",
                    Screen::AuthorList { .. } => "AuthorList",
                    Screen::RenameForm { .. } => "RenameForm",
                    Screen::Preview { .. } => "Preview",
                    Screen::CoAuthorList { .. } => "CoAuthorList",
                    Screen::Success { .. } => "Success",
                    Screen::HookAddList { .. } => "HookAddList",
                    Screen::HookManageList { .. } => "HookManageList",
                    Screen::HookSuccess { .. } => "HookSuccess",
                    Screen::HookAlreadyStripped { .. } => "HookAlreadyStripped",
                    Screen::HookRemoved => "HookRemoved",
                    Screen::Err(_) => "Err",
                }
            ),
        }
    }

    #[test]
    fn test_coauthor_list_enter_calls_scan_drop_and_transitions_to_preview_with_data() {
        // DROP-04: Enter on co-author list calls scan_drop and stores RewritePreview in Preview.
        let (_dir, mut app) = make_test_app_with_coauthors();
        // Navigate to Drop
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        // Should be in CoAuthorList; Bob is there
        // Press Enter to select Bob
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::Preview {
                op: PendingOp::Drop { target },
                scan,
            } => {
                assert_eq!(target.name, "Bob");
                // The commit has a Co-authored-by Bob trailer, so affected_count >= 1
                assert!(
                    scan.affected_count >= 1,
                    "scan.affected_count must reflect actual commits"
                );
            }
            _ => panic!("expected Preview {{ op: Drop, scan }}"),
        }
    }

    #[test]
    fn test_preview_y_transitions_directly_to_success() {
        // RENAME-05/DROP-04: Y on Preview runs the rewrite synchronously (Strategy A)
        // and transitions DIRECTLY to Success (no intermediate Executing state).
        let (_dir, mut app) = make_test_app_with_commits();
        use crate::git::scan::RewritePreview;
        let scan = RewritePreview {
            affected_count: 0,
            signed_commit_count: 0,
            annotated_tags_affected: vec![],
            has_notes_ref: false,
            remote_name: None,
        };
        let op = PendingOp::Rename {
            source: AuthorIdentity {
                name: "Nobody".into(),
                email: "nobody@x".into(),
                commit_count: 0,
            },
            new_name: "Someone".into(),
            new_email: "someone@x".into(),
        };
        app.screen = Screen::Preview { op, scan };
        handle_key(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
        // Must transition to Success (not Executing, not stay on Preview)
        assert!(
            matches!(app.screen, Screen::Success { .. }),
            "Y on Preview must transition directly to Success (Strategy A — synchronous)"
        );
    }

    #[test]
    fn test_preview_y_calls_rewrite_author_for_rename_op() {
        // RENAME-05: Y on Preview with Rename op calls rewrite_author and transitions to Success.
        let (_dir, mut app) = make_test_app_with_commits();
        // Go through the full flow to get a real Preview with real scan data
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // -> AuthorList
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // select Alice -> RenameForm
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // List -> Name before typing
        for c in "Alice2".chars() {
            handle_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        for c in "alice2@example.com".chars() {
            handle_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // -> Preview
        assert!(
            matches!(app.screen, Screen::Preview { .. }),
            "should be at Preview before Y"
        );
        handle_key(&mut app, KeyCode::Char('y'), KeyModifiers::NONE); // execute rewrite
        match &app.screen {
            Screen::Success { rewritten, .. } => {
                assert!(
                    *rewritten >= 1,
                    "rewrite_author should have written >= 1 commit"
                );
            }
            Screen::Err(e) => panic!("rewrite failed: {}", e),
            _ => panic!("expected Success after Y on Preview"),
        }
    }

    #[test]
    fn test_preview_y_calls_drop_coauthor_for_drop_op() {
        // DROP-04: Y on Preview with Drop op calls drop_coauthor and transitions to Success.
        let (_dir, mut app) = make_test_app_with_coauthors();
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // select Drop
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // -> CoAuthorList
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // select Bob -> Preview
        assert!(
            matches!(app.screen, Screen::Preview { .. }),
            "should be at Preview before Y"
        );
        handle_key(&mut app, KeyCode::Char('y'), KeyModifiers::NONE); // execute drop
        match &app.screen {
            Screen::Success { rewritten, .. } => {
                assert!(
                    *rewritten >= 1,
                    "drop_coauthor should have written >= 1 commit"
                );
            }
            Screen::Err(e) => panic!("drop failed: {}", e),
            _ => panic!("expected Success after Y on Preview"),
        }
    }

    #[test]
    fn test_preview_n_returns_to_main_menu() {
        // Pressing N on Preview cancels without writing and returns to MainMenu.
        let (_dir, mut app) = make_test_app();
        use crate::git::scan::RewritePreview;
        let scan = RewritePreview {
            affected_count: 0,
            signed_commit_count: 0,
            annotated_tags_affected: vec![],
            has_notes_ref: false,
            remote_name: None,
        };
        app.screen = Screen::Preview {
            op: PendingOp::Drop {
                target: crate::git::types::CoAuthorEntry {
                    name: "x".into(),
                    email: "x@x".into(),
                    commit_count: 0,
                },
            },
            scan,
        };
        handle_key(&mut app, KeyCode::Char('n'), KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_preview_esc_returns_to_main_menu() {
        // Pressing Esc on Preview cancels and returns to MainMenu.
        let (_dir, mut app) = make_test_app();
        use crate::git::scan::RewritePreview;
        let scan = RewritePreview {
            affected_count: 0,
            signed_commit_count: 0,
            annotated_tags_affected: vec![],
            has_notes_ref: false,
            remote_name: None,
        };
        app.screen = Screen::Preview {
            op: PendingOp::Drop {
                target: crate::git::types::CoAuthorEntry {
                    name: "x".into(),
                    email: "x@x".into(),
                    commit_count: 0,
                },
            },
            scan,
        };
        handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::MainMenu { selected: 0 }));
    }

    #[test]
    fn test_preview_other_keys_ignored() {
        // Pressing an unrelated key on Preview does nothing.
        let (_dir, mut app) = make_test_app();
        use crate::git::scan::RewritePreview;
        let scan = RewritePreview {
            affected_count: 0,
            signed_commit_count: 0,
            annotated_tags_affected: vec![],
            has_notes_ref: false,
            remote_name: None,
        };
        app.screen = Screen::Preview {
            op: PendingOp::Drop {
                target: crate::git::types::CoAuthorEntry {
                    name: "x".into(),
                    email: "x@x".into(),
                    commit_count: 0,
                },
            },
            scan,
        };
        handle_key(&mut app, KeyCode::F(1), KeyModifiers::NONE); // arbitrary unrelated key
        assert!(matches!(app.screen, Screen::Preview { .. }));
    }

    #[test]
    fn test_success_any_key_exits() {
        // Any key on Success causes should_exit = true (OUT-01: exit after rewrite).
        let (_dir, mut app) = make_test_app();
        app.screen = Screen::Success {
            rewritten: 5,
            remote_name: Some("origin".into()),
            copied: false,
        };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.should_exit);
    }

    #[test]
    fn test_err_any_key_exits() {
        // Any key on Err causes should_exit = true.
        let (_dir, mut app) = make_test_app();
        app.screen = Screen::Err("something went wrong".into());
        handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.should_exit);
    }

    // ---- New tests for Plan 06-03 (Add hook flow) ----

    fn make_hook_add_list_screen(entries: &[(&str, &str)]) -> Screen {
        use crate::git::types::CoAuthorEntry;
        let items: Vec<CoAuthorEntry> = entries
            .iter()
            .map(|(n, e)| CoAuthorEntry {
                name: n.to_string(),
                email: e.to_string(),
                commit_count: 1,
            })
            .collect();
        let mut nucleo = build_coauthor_nucleo(&items);
        let matched = apply_coauthor_filter(&mut nucleo, "");
        Screen::HookAddList {
            current_strip: vec![],
            items,
            filter: String::new(),
            matched,
            nucleo,
            selected: 0,
        }
    }

    #[test]
    fn test_main_menu_routes_add_hook() {
        // HOOK-01: Selecting 'Add co-author auto-strip hook' (index 2) opens HookAddList.
        // 06-02 stub sets Screen::Err("not yet implemented"); this test confirms replacement.
        let (_dir, mut app) = make_test_app_with_commits();
        // Navigate to index 2 (AddHook)
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(app.screen, Screen::HookAddList { .. }),
            "expected HookAddList, got something else"
        );
    }

    #[test]
    fn test_add_hook_happy_path() {
        // HOOK-01/HOOK-11: Selecting a co-author calls install_strip; success -> HookSuccess
        // with state populated from read_strip_list (not from cached data).
        let (_dir, mut app) = make_test_app_with_commits();
        app.screen = make_hook_add_list_screen(&[("Bob", "bob@example.com")]);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::HookSuccess { state } => match state {
                crate::hook::HookState::Managed { emails } => {
                    assert!(
                        emails.iter().any(|e| e == "bob@example.com"),
                        "strip list must contain bob@example.com"
                    );
                }
                _ => panic!("expected HookState::Managed, got other variant"),
            },
            _ => panic!("expected HookSuccess, got different screen"),
        }
    }

    #[test]
    fn test_add_hook_already_stripped() {
        // HOOK-01: Selecting a duplicate co-author -> HookAlreadyStripped { email }.
        let (_dir, mut app) = make_test_app_with_commits();
        // Pre-install bob@example.com via hook engine
        let result = crate::hook::install_strip(&app.repo, "bob@example.com");
        assert!(
            matches!(result, Ok(crate::hook::AddResult::Installed { .. })),
            "pre-install must succeed"
        );
        // Now attempt to add the same email via the TUI
        app.screen = make_hook_add_list_screen(&[("Bob", "bob@example.com")]);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::HookAlreadyStripped { email } => {
                assert_eq!(email, "bob@example.com");
            }
            _ => panic!("expected HookAlreadyStripped, got different screen"),
        }
    }

    #[test]
    fn test_hook_already_stripped_any_key_returns_to_menu() {
        // HOOK-01: Any key from HookAlreadyStripped returns to MainMenu at index 2.
        let (_dir, mut app) = make_test_app();
        app.screen = Screen::HookAlreadyStripped {
            email: "x@x".into(),
        };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(app.screen, Screen::MainMenu { selected: 2 }),
            "expected MainMenu {{ selected: 2 }}"
        );
    }

    #[test]
    fn test_hook_success_any_key_exits() {
        // HOOK-01: Any key from HookSuccess sets should_exit = true.
        let (_dir, mut app) = make_test_app();
        app.screen = Screen::HookSuccess {
            state: crate::hook::HookState::Absent,
        };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.should_exit, "HookSuccess any-key must set should_exit");
    }

    // ---- New tests for Plan 06-04 (Manage hook flow) ----

    fn make_hook_manage_list_screen(emails: &[&str]) -> Screen {
        let items: Vec<String> = emails.iter().map(|e| e.to_string()).collect();
        let mut nucleo = build_strip_nucleo(&items);
        let matched = apply_strip_filter(&mut nucleo, "");
        Screen::HookManageList {
            items,
            filter: String::new(),
            matched,
            nucleo,
            selected: 0,
        }
    }

    #[test]
    fn test_main_menu_routes_manage_hook_empty() {
        // HOOK-02: Selecting 'Manage auto-strip hook' (index 3) on a repo with no hook
        // shows HookSuccess { state: Absent } — empty state, no list screen.
        // FAILS with 06-02 stub (Screen::Err("not yet implemented")).
        let (_dir, mut app) = make_test_app_with_commits();
        // Navigate to index 3 (ManageHook)
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(
                app.screen,
                Screen::HookSuccess {
                    state: crate::hook::HookState::Absent
                }
            ),
            "expected HookSuccess(Absent) for ManageHook on repo with no hook"
        );
    }

    #[test]
    fn test_main_menu_routes_manage_hook_with_entries() {
        // HOOK-02: Selecting 'Manage' on a repo with hook entries shows HookManageList.
        // FAILS with 06-02 stub.
        let (_dir, mut app) = make_test_app_with_commits();
        crate::hook::install_strip(&app.repo, "bob@example.com").unwrap();
        // Navigate to index 3 (ManageHook)
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(app.screen, Screen::HookManageList { .. }),
            "expected HookManageList for ManageHook on repo with installed hook"
        );
    }

    #[test]
    fn test_manage_remove_single_entry() {
        // HOOK-09/HOOK-11: HookManageList with two emails; removing one -> HookSuccess(Managed)
        // with the remaining email (exercises UpdateResult::Updated + re-read path).
        // FAILS because HookManageList arm is a placeholder.
        let (_dir, mut app) = make_test_app_with_commits();
        crate::hook::install_strip(&app.repo, "bob@example.com").unwrap();
        crate::hook::install_strip(&app.repo, "carol@example.com").unwrap();
        app.screen = make_hook_manage_list_screen(&["bob@example.com", "carol@example.com"]);
        // Press Enter to remove the first (bob@example.com)
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::HookSuccess {
                state: crate::hook::HookState::Managed { emails },
            } => {
                assert!(
                    emails.iter().any(|e| e == "carol@example.com"),
                    "carol@example.com must remain after removing bob"
                );
                assert!(
                    !emails.iter().any(|e| e == "bob@example.com"),
                    "bob@example.com must not be in remaining list"
                );
            }
            _ => panic!(
                "expected HookSuccess(Managed) after removing one of two entries, got other screen"
            ),
        }
    }

    #[test]
    fn test_manage_remove_last_entry() {
        // HOOK-09/HOOK-11: HookManageList with one email; removing it -> HookRemoved
        // directly (HookDeleted path — no re-read; HOOK-11 exception).
        let (_dir, mut app) = make_test_app_with_commits();
        crate::hook::install_strip(&app.repo, "bob@example.com").unwrap();
        app.screen = make_hook_manage_list_screen(&["bob@example.com"]);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(app.screen, Screen::HookRemoved),
            "expected HookRemoved after removing the last entry"
        );
    }

    #[test]
    fn test_manage_remove_last_entry_shows_hook_removed_distinct_from_empty_state() {
        // SC4: removing the last strip entry must produce Screen::HookRemoved,
        // distinct from the never-installed empty state (Screen::HookSuccess { state: Absent }).
        //
        // Part A — HookDeleted path produces HookRemoved:
        let (_dir, mut app) = make_test_app_with_commits();
        crate::hook::install_strip(&app.repo, "bob@example.com").unwrap();
        app.screen = make_hook_manage_list_screen(&["bob@example.com"]);
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(app.screen, Screen::HookRemoved),
            "expected Screen::HookRemoved after removing the last entry, got {:?}",
            std::mem::discriminant(&app.screen)
        );

        // Part B — empty-state path (no hook ever installed) still produces HookSuccess(Absent):
        let (_dir2, mut app2) = make_test_app();
        // No hook installed; navigate to Manage (index 3) and Enter
        handle_key(&mut app2, KeyCode::Down, KeyModifiers::NONE); // 0->1
        handle_key(&mut app2, KeyCode::Down, KeyModifiers::NONE); // 1->2
        handle_key(&mut app2, KeyCode::Down, KeyModifiers::NONE); // 2->3
        handle_key(&mut app2, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            matches!(
                app2.screen,
                Screen::HookSuccess {
                    state: crate::hook::HookState::Absent
                }
            ),
            "expected HookSuccess(Absent) for never-installed empty state"
        );
    }

    #[test]
    fn test_manage_esc_returns_to_main_menu() {
        // HOOK-02: Esc from HookManageList returns to MainMenu { selected: 3 }.
        // FAILS because HookManageList arm is a placeholder.
        let (_dir, mut app) = make_test_app();
        app.screen = make_hook_manage_list_screen(&["bob@example.com"]);
        handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(
            matches!(app.screen, Screen::MainMenu { selected: 3 }),
            "expected MainMenu {{ selected: 3 }} after Esc from HookManageList"
        );
    }

    // ---- New tests for Plan 06-05 (HOOK-14 stash-bypass regression) ----

    #[test]
    fn test_add_hook_no_preflight_with_stash() {
        // HOOK-14 / HOOK-12: Add flow must NOT trigger the stash preflight (SAFE-01).
        // A repo with a stash ref must reach HookAddList or HookSuccess, not Screen::Err.
        let (_dir, mut app) = make_test_app_with_stash();
        // Navigate to Add (index 2): two Down presses
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // 0 -> 1
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // 1 -> 2
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::Err(msg) => {
                assert!(
                    !msg.contains("stash") && !msg.contains("Stash"),
                    "Add flow must not trigger stash preflight; got Err: {}",
                    msg
                );
            }
            Screen::HookAddList { .. } | Screen::HookSuccess { .. } => {} // both acceptable
            other => panic!(
                "unexpected screen variant after Add on stash repo: {:?}",
                std::mem::discriminant(other)
            ),
        }
    }

    #[test]
    fn test_manage_no_preflight_with_stash() {
        // HOOK-14 / HOOK-12: Manage flow must NOT trigger the stash preflight (SAFE-02).
        // A fresh repo with a stash ref must reach HookSuccess(Absent), not Screen::Err.
        let (_dir, mut app) = make_test_app_with_stash();
        // Navigate to Manage (index 3): three Down presses
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // 0 -> 1
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // 1 -> 2
        handle_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // 2 -> 3
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::Err(msg) => {
                assert!(
                    !msg.contains("stash") && !msg.contains("Stash"),
                    "Manage flow must not trigger stash preflight; got Err: {}",
                    msg
                );
            }
            Screen::HookSuccess { .. } | Screen::HookManageList { .. } => {} // both acceptable
            other => panic!(
                "unexpected screen variant after Manage on stash repo: {:?}",
                std::mem::discriminant(other)
            ),
        }
    }

    // ---- Behavior tests for 260524-kgx (3-way Tab, autofill, filter routing, empty list, source exclusion) ----

    #[test]
    fn test_rename_form_tab_cycles_through_list() {
        // The focus cycle must include List as a third target so users can reach
        // the embedded author list without leaving the form. Tab from Email -> List,
        // Tab from List -> Name (completes the 3-way cycle).
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[("Bob", "bob@x")],
        );
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::Name;
        }
        // Name -> Email
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert!(matches!(draft.focused, FormField::Email)),
            _ => panic!("expected RenameForm"),
        }
        // Email -> List
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert!(matches!(draft.focused, FormField::List)),
            _ => panic!("expected RenameForm"),
        }
        // List -> Name
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { draft, .. } => assert!(matches!(draft.focused, FormField::Name)),
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_list_enter_picks_author_and_submits() {
        // Enter on a highlighted list author fills both fields and submits in one
        // keystroke. A picked author is always a complete identity, so requiring a
        // Tab-then-Enter to proceed was redundant friction.
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[("Bob", "bob@example.com")],
        );
        // Focus the list
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::List;
        }
        // Enter on the first (only) matched author
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::Preview {
                op: PendingOp::Rename { new_name, new_email, .. },
                ..
            } => {
                assert_eq!(new_name, "Bob");
                assert_eq!(new_email, "bob@example.com");
            }
            _ => panic!("expected Preview after picking an author from the list"),
        }
    }

    #[test]
    fn test_rename_form_list_typing_filters_not_text_fields() {
        // When the List is focused, typed characters must update the filter and matched
        // items, NOT the name/email text fields. This ensures typing routes correctly.
        let (_dir, mut app) = make_test_app();
        app.screen = make_rename_form_screen(
            AuthorIdentity { name: "Alice".into(), email: "alice@x".into(), commit_count: 1 },
            &[("Bob", "bob@x"), ("Carol", "carol@x")],
        );
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::List;
        }
        handle_key(&mut app, KeyCode::Char('b'), KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { draft, filter, .. } => {
                assert_eq!(filter, "b", "filter must reflect typed char");
                assert!(draft.new_name.is_empty(), "new_name must not change when List focused");
                assert!(draft.new_email.is_empty(), "new_email must not change when List focused");
            }
            _ => panic!("expected RenameForm"),
        }
        handle_key(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { filter, .. } => {
                assert!(filter.is_empty(), "filter must shrink on Backspace");
            }
            _ => panic!("expected RenameForm"),
        }
    }

    #[test]
    fn test_rename_form_empty_excluded_list_still_submittable() {
        // When the source is the only author (excluded list is empty), List focus must
        // be reachable (fixed 3-way cycle) and Enter on List must be a no-op.
        // The form must still be submittable via Name/Email focus.
        let (_dir, mut app) = make_test_app_with_commits();
        // Navigate to AuthorList then select Alice -> RenameForm with empty excluded list
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // -> AuthorList (only Alice)
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // select Alice -> RenameForm
        // Verify we're on RenameForm
        assert!(matches!(app.screen, Screen::RenameForm { .. }), "should be on RenameForm");
        // Check the excluded list is empty (single-author repo)
        match &app.screen {
            Screen::RenameForm { matched, .. } => {
                assert!(matched.is_empty(), "excluded list must be empty for single-author repo");
            }
            _ => panic!("expected RenameForm"),
        }
        // Start from Name focus to exercise the documented 3-way cycle.
        if let Screen::RenameForm { draft, .. } = &mut app.screen {
            draft.focused = FormField::Name;
        }
        // Tab to List focus — must not panic or skip
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // Name -> Email
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // Email -> List
        match &app.screen {
            Screen::RenameForm { draft, .. } => {
                assert!(matches!(draft.focused, FormField::List), "should reach List focus");
            }
            _ => panic!("expected RenameForm"),
        }
        // Enter on empty list is a no-op — stay on RenameForm, fields unchanged
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(app.screen, Screen::RenameForm { .. }), "Enter on empty list must stay on RenameForm");
        // Return to Name focus and fill in both fields, then submit
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // List -> Name
        for c in "NewAlice".chars() {
            handle_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        handle_key(&mut app, KeyCode::Tab, KeyModifiers::NONE); // Name -> Email
        for c in "new@example.com".chars() {
            handle_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // submit (is_complete() -> true)
        assert!(
            matches!(app.screen, Screen::Preview { .. }),
            "complete form must submit to Preview even when list was empty"
        );
    }

    #[test]
    fn test_rename_form_excludes_source_author() {
        // The embedded list must never contain the source being renamed — deduplication
        // requires the list shows only OTHER authors to merge into.
        let (_dir, mut app) = make_test_app();
        app.screen = make_author_list_screen(&["Alice", "Bob", "Carol"]);
        // Alice is selected (index 0); transition to RenameForm
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        match &app.screen {
            Screen::RenameForm { source, items, matched, .. } => {
                let src_name = source.name.clone();
                let src_email = source.email.clone();
                assert!(
                    !items.iter().any(|a| a.name == src_name && a.email == src_email),
                    "source must not appear in items"
                );
                assert!(
                    !matched.iter().any(|a| a.name == src_name && a.email == src_email),
                    "source must not appear in matched"
                );
                assert_eq!(items.len(), 2, "two other authors remain after excluding source");
            }
            _ => panic!("expected RenameForm"),
        }
    }
}
