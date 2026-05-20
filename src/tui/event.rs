use crate::tui::app::{App, Screen};
use crossterm::event::KeyCode;

pub fn handle_key(_app: &mut App, _key: KeyCode) {
    // Stub — implemented in GREEN step
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_app() -> (TempDir, App) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init_bare(dir.path()).unwrap();
        (dir, App::new(repo))
    }

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
    fn test_main_menu_enter_with_selected_0_transitions_to_rename_placeholder() {
        // CORE-01 + RENAME-01: selecting Rename advances state.
        let (_dir, mut app) = make_test_app();
        handle_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.screen, Screen::NotImplemented("rename")));
    }

    #[test]
    fn test_main_menu_enter_with_selected_1_transitions_to_drop_placeholder() {
        // CORE-01 + DROP-01: selecting Drop advances state.
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
}
