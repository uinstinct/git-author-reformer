use crate::git::scan::RewritePreview;
use crate::git::types::{AuthorIdentity, CoAuthorEntry};
use crate::hook::HookState;
use crate::tui::app::{App, FormField, MenuChoice, PendingOp, RenameDraft, Screen};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &App) {
    match &app.screen {
        Screen::MainMenu { selected } => render_main_menu(frame, frame.area(), *selected),
        Screen::AuthorList {
            filter,
            matched,
            selected,
            ..
        } => render_author_list(frame, frame.area(), filter, matched, *selected),
        Screen::RenameForm {
            source,
            draft,
            filter,
            matched,
            selected,
            ..
        } => render_rename_form(frame, frame.area(), source, draft, filter, matched, *selected),
        Screen::Preview { op, scan } => render_preview(frame, frame.area(), op, scan),
        Screen::CoAuthorList {
            filter,
            matched,
            selected,
            ..
        } => render_coauthor_list(frame, frame.area(), filter, matched, *selected),
        Screen::Success {
            rewritten,
            remote_name,
            copied,
        } => render_success(frame, frame.area(), *rewritten, remote_name, *copied),
        Screen::HookAddList {
            current_strip,
            filter,
            matched,
            selected,
            ..
        } => render_hook_add_list(
            frame,
            frame.area(),
            current_strip,
            filter,
            matched,
            *selected,
        ),
        Screen::HookManageList {
            filter,
            matched,
            selected,
            ..
        } => render_hook_manage_list(frame, frame.area(), filter, matched, *selected),
        Screen::HookSuccess { state } => render_hook_success(frame, frame.area(), state),
        Screen::HookAlreadyStripped { email } => {
            render_hook_already_stripped(frame, frame.area(), email)
        }
        Screen::HookRemoved => render_hook_removed(frame, frame.area()),
        Screen::Err(msg) => render_err(frame, frame.area(), msg),
    }
}

fn render_main_menu(frame: &mut Frame, area: Rect, selected: usize) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    frame.render_widget(Paragraph::new("git-author-reformer"), header);

    let items: Vec<ListItem> = MenuChoice::all()
        .iter()
        .map(|c| ListItem::new(c.label()))
        .collect();
    let list = List::new(items)
        .block(Block::bordered().title("Main Menu"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(list, body, &mut state);

    frame.render_widget(
        Paragraph::new("\u{2191}/\u{2193} or j/k: move   Enter: select   q/Esc: quit"),
        footer,
    );
}

fn render_author_list(
    frame: &mut Frame,
    area: Rect,
    filter: &str,
    matched: &[AuthorIdentity],
    selected: usize,
) {
    let [filter_row, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Filter input row — cursor positioned at end of text
    let filter_text = format!("/ {}", filter);
    frame.render_widget(
        Paragraph::new(filter_text.as_str()).block(Block::bordered().title("Filter")),
        filter_row,
    );
    // Cursor: +1 for border, +2 for "/ " prefix, + filter length
    let cursor_x = filter_row.x + 1 + 2 + filter.chars().count() as u16;
    let cursor_y = filter_row.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));

    // Authors list
    let items: Vec<ListItem> = matched
        .iter()
        .map(|item| {
            ListItem::new(format!(
                "{:>4}  {} <{}>",
                item.commit_count, item.name, item.email
            ))
        })
        .collect();
    let list = List::new(items)
        .block(Block::bordered().title(format!("Authors ({} match)", matched.len())))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(if matched.is_empty() {
        None
    } else {
        Some(selected)
    });
    frame.render_stateful_widget(list, body, &mut state);

    frame.render_widget(
        Paragraph::new("type: filter   \u{2191}/\u{2193}: move   Enter: select   Esc: back"),
        footer,
    );
}

fn render_rename_form(
    frame: &mut Frame,
    area: Rect,
    source: &AuthorIdentity,
    draft: &RenameDraft,
    filter: &str,
    matched: &[AuthorIdentity],
    selected: usize,
) {
    let [header, name_field, email_field, filter_row, list_body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Header: source identity being renamed
    frame.render_widget(
        Paragraph::new(format!("Renaming: {} <{}>", source.name, source.email))
            .block(Block::bordered()),
        header,
    );

    // Name field
    let name_focused = matches!(draft.focused, FormField::Name);
    let name_style = if name_focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let name_title = if name_focused { "* New name" } else { "New name" };
    frame.render_widget(
        Paragraph::new(draft.new_name.as_str())
            .block(Block::bordered().title(name_title).border_style(name_style)),
        name_field,
    );

    // Email field
    let email_focused = matches!(draft.focused, FormField::Email);
    let email_style = if email_focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let email_title = if email_focused { "* New email" } else { "New email" };
    frame.render_widget(
        Paragraph::new(draft.new_email.as_str()).block(
            Block::bordered()
                .title(email_title)
                .border_style(email_style),
        ),
        email_field,
    );

    // Filter row for the embedded author list
    let list_focused = matches!(draft.focused, FormField::List);
    let filter_style = if list_focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let filter_title = if list_focused { "* Filter" } else { "Filter" };
    let filter_text = format!("/ {}", filter);
    frame.render_widget(
        Paragraph::new(filter_text.as_str())
            .block(Block::bordered().title(filter_title).border_style(filter_style)),
        filter_row,
    );

    // Embedded author list (excludes source)
    let items: Vec<ListItem> = matched
        .iter()
        .map(|item| {
            ListItem::new(format!(
                "{:>4}  {} <{}>",
                item.commit_count, item.name, item.email
            ))
        })
        .collect();
    let list = List::new(items)
        .block(Block::bordered().title(format!("Authors ({} match)", matched.len())))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(if matched.is_empty() { None } else { Some(selected) });
    frame.render_stateful_widget(list, list_body, &mut state);

    // Cursor: place in the focused zone
    match draft.focused {
        FormField::Name => {
            let cursor_x = name_field.x + 1 + draft.new_name.chars().count() as u16;
            let cursor_y = name_field.y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
        FormField::Email => {
            let cursor_x = email_field.x + 1 + draft.new_email.chars().count() as u16;
            let cursor_y = email_field.y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
        FormField::List => {
            let cursor_x = filter_row.x + 1 + 2 + filter.chars().count() as u16;
            let cursor_y = filter_row.y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    frame.render_widget(
        Paragraph::new("Tab: switch  type/Up/Down on list: pick  Enter: confirm/autofill  Esc: cancel"),
        footer,
    );
}

fn render_coauthor_list(
    frame: &mut Frame,
    area: Rect,
    filter: &str,
    matched: &[CoAuthorEntry],
    selected: usize,
) {
    let [filter_row, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Filter input row — cursor positioned at end of text
    let filter_text = format!("/ {}", filter);
    frame.render_widget(
        Paragraph::new(filter_text.as_str()).block(Block::bordered().title("Filter")),
        filter_row,
    );
    // Cursor: +1 for border, +2 for "/ " prefix, + filter length
    let cursor_x = filter_row.x + 1 + 2 + filter.chars().count() as u16;
    let cursor_y = filter_row.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));

    // Co-authors list
    let items: Vec<ListItem> = matched
        .iter()
        .map(|item| {
            ListItem::new(format!(
                "{:>4}  {} <{}>",
                item.commit_count, item.name, item.email
            ))
        })
        .collect();
    let list = List::new(items)
        .block(Block::bordered().title(format!("Co-authors ({} match)", matched.len())))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(if matched.is_empty() {
        None
    } else {
        Some(selected)
    });
    frame.render_stateful_widget(list, body, &mut state);

    frame.render_widget(
        Paragraph::new("type: filter   \u{2191}/\u{2193}: move   Enter: select   Esc: back"),
        footer,
    );
}

fn render_preview(frame: &mut Frame, area: Rect, op: &PendingOp, scan: &RewritePreview) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(area);

    // Header: one-line operation summary
    let header_text = match op {
        PendingOp::Rename {
            source,
            new_name,
            new_email,
        } => {
            format!(
                "Rename: {} <{}> \u{2192} {} <{}>",
                source.name, source.email, new_name, new_email
            )
        }
        PendingOp::Drop { target } => {
            format!("Drop co-author: {} <{}>", target.name, target.email)
        }
    };
    frame.render_widget(
        Paragraph::new(header_text).block(Block::bordered().title("Preview")),
        header,
    );

    // Body: affected count + conditional warnings + proceed prompt
    let mut lines = vec![
        format!("This will rewrite {} commit(s).", scan.affected_count),
        String::new(),
    ];
    if scan.signed_commit_count > 0 {
        lines.push(format!(
            "\u{26a0} {} commit(s) in the affected set are GPG/SSH-signed \u{2014} signatures will be invalidated.",
            scan.signed_commit_count
        ));
    }
    if !scan.annotated_tags_affected.is_empty() {
        lines.push(format!(
            "\u{26a0} Annotated tag(s) will be recreated: {}",
            scan.annotated_tags_affected.join(", ")
        ));
    }
    if scan.has_notes_ref {
        lines.push(
            "\u{26a0} refs/notes/commits exists \u{2014} notes reference old SHAs and will be orphaned by the rewrite."
                .to_string(),
        );
    }
    lines.push(String::new());
    lines.push(format!(
        "\u{26a0} This rewrites history. Collaborators will need to re-clone or force-reset. \
         Push with: git push --force-with-lease --all {}",
        scan.remote_name.as_deref().unwrap_or("<remote>")
    ));
    lines.push(String::new());
    lines.push("Proceed? (Y/N)".to_string());

    frame.render_widget(
        Paragraph::new(lines.join("\n"))
            .block(Block::bordered())
            .wrap(Wrap { trim: false }),
        body,
    );

    frame.render_widget(
        Paragraph::new("Y / Enter: confirm   N / Esc: cancel"),
        footer,
    );
}

fn render_success(
    frame: &mut Frame,
    area: Rect,
    rewritten: usize,
    remote_name: &Option<String>,
    copied: bool,
) {
    let remote = remote_name.as_deref().unwrap_or("<remote>");
    let copy_hint = if copied {
        "Copied!  |  Any key to exit"
    } else {
        "Press 'c' to copy  |  Any key to exit"
    };
    let text = format!(
        "\u{2714} Rewrote {} commit(s).\n\nRun the following to update the remote:\n\n  git push --force-with-lease --all {}\n\n{}",
        rewritten, remote, copy_hint
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::bordered().title("Success"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_err(frame: &mut Frame, area: Rect, msg: &str) {
    frame.render_widget(
        Paragraph::new(format!("\u{2717} Error\n\n{msg}\n\nPress any key to exit."))
            .block(Block::bordered().title("Error"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_hook_add_list(
    frame: &mut Frame,
    area: Rect,
    current_strip: &[String],
    filter: &str,
    matched: &[CoAuthorEntry],
    selected: usize,
) {
    // Three-zone layout: strip list header | filter input | co-author list | hint
    let [strip_header, filter_row, body, hint] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Zone 1: Current strip list
    let strip_text = if current_strip.is_empty() {
        "no entries yet".to_string()
    } else {
        current_strip.join("\n")
    };
    frame.render_widget(
        Paragraph::new(strip_text)
            .block(Block::bordered().title("Current strip list"))
            .wrap(Wrap { trim: false }),
        strip_header,
    );

    // Zone 2: Filter input
    let filter_text = format!("/ {}", filter);
    frame.render_widget(
        Paragraph::new(filter_text.as_str()).block(Block::bordered().title("Filter")),
        filter_row,
    );
    let cursor_x = filter_row.x + 1 + 2 + filter.chars().count() as u16;
    let cursor_y = filter_row.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));

    // Zone 3: Co-author list
    let items: Vec<ListItem> = matched
        .iter()
        .map(|item| {
            ListItem::new(format!(
                "{:>4}  {} <{}>",
                item.commit_count, item.name, item.email
            ))
        })
        .collect();
    let list = List::new(items)
        .block(Block::bordered().title(format!("Co-authors ({} match)", matched.len())))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(if matched.is_empty() {
        None
    } else {
        Some(selected)
    });
    frame.render_stateful_widget(list, body, &mut state);

    // Zone 4: Hint
    frame.render_widget(
        Paragraph::new("type: filter  up/down: move  Enter: select  Esc: back"),
        hint,
    );
}

fn render_hook_manage_list(
    frame: &mut Frame,
    area: Rect,
    filter: &str,
    matched: &[String],
    selected: usize,
) {
    let [filter_row, body, hint] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Zone 1: Filter input with cursor
    let filter_text = format!("/ {}", filter);
    frame.render_widget(
        Paragraph::new(filter_text.as_str()).block(Block::bordered().title("Filter")),
        filter_row,
    );
    let cursor_x = filter_row.x + 1 + 2 + filter.chars().count() as u16;
    let cursor_y = filter_row.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));

    // Zone 2: Strip email list
    let title = if matched.is_empty() {
        "Strip list (empty)".to_string()
    } else {
        format!("Strip list ({} entries)", matched.len())
    };
    let items: Vec<ListItem> = matched
        .iter()
        .map(|email| ListItem::new(email.as_str()))
        .collect();
    let list = List::new(items)
        .block(Block::bordered().title(title))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(if matched.is_empty() {
        None
    } else {
        Some(selected)
    });
    frame.render_stateful_widget(list, body, &mut state);

    // Zone 3: Hint
    frame.render_widget(
        Paragraph::new("type: filter  up/down: move  Enter: remove  Esc: back"),
        hint,
    );
}

fn render_hook_success(frame: &mut Frame, area: Rect, state: &HookState) {
    let text = match state {
        HookState::Absent => {
            "No hook installed — no emails configured.\n\nAny key to exit.".to_string()
        }
        HookState::Managed { emails } => {
            let list = emails
                .iter()
                .map(|e| format!("  {}", e))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Hook active — stripping {} email(s):\n{}\n\nAny key to exit.",
                emails.len(),
                list
            )
        }
        HookState::NotToolManaged(_) => {
            "Error: foreign hook (should not reach this screen).".to_string()
        }
    };
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::bordered().title("Hook Status"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_hook_already_stripped(frame: &mut Frame, area: Rect, email: &str) {
    let text = format!(
        "Already stripped: {}\n\nThis email is already in the strip list.\n\nAny key to return to menu.",
        email
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::bordered().title("No change"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_hook_removed(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new("Hook removed \u{2014} no entries remain.\n\nAny key to exit.")
            .block(Block::bordered().title("Hook Removed"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::tui::app::App;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn buffer_text(width: u16, height: u16) -> String {
        let dir = tempfile::TempDir::new().unwrap();
        let repo = git2::Repository::init_bare(dir.path()).unwrap();
        let mut app = App::new(repo);

        let source = AuthorIdentity {
            name: "Alice".into(),
            email: "alice@example.com".into(),
            commit_count: 3,
        };
        let others = [
            AuthorIdentity {
                name: "Bob".into(),
                email: "bob@example.com".into(),
                commit_count: 2,
            },
            AuthorIdentity {
                name: "Carol".into(),
                email: "carol@example.com".into(),
                commit_count: 1,
            },
        ];
        let mut nucleo = crate::tui::app::build_author_nucleo(&others);
        let matched = crate::tui::app::apply_filter(&mut nucleo, "");
        app.screen = Screen::RenameForm {
            source,
            draft: RenameDraft::default(),
            items: others.to_vec(),
            filter: String::new(),
            matched,
            nucleo,
            selected: 0,
        };

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();
        let buf = terminal.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn rename_form_renders_embedded_author_list() {
        // Regression guard: the embedded author list on the rename screen must paint
        // actual author rows, not just an empty bordered box. The prior unit test only
        // asserted items.len(); it could not catch an empty render.
        let text = buffer_text(80, 24);
        assert!(text.contains("Authors (2 match)"), "list title must paint; got:\n{text}");
        assert!(text.contains("Bob"), "author Bob must paint; got:\n{text}");
        assert!(text.contains("Carol"), "author Carol must paint; got:\n{text}");
    }
}
