use crate::git::types::AuthorIdentity;
use crate::tui::app::{App, FormField, MenuChoice, PendingOp, RenameDraft, Screen};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

pub fn render(frame: &mut Frame, app: &App) {
    match &app.screen {
        Screen::MainMenu { selected } => render_main_menu(frame, frame.area(), *selected),
        Screen::NotImplemented(tag) => render_not_implemented(frame, frame.area(), tag),
        Screen::AuthorList { filter, matched, selected, .. } => {
            render_author_list(frame, frame.area(), filter, matched, *selected)
        }
        Screen::RenameForm { source, draft } => {
            render_rename_form(frame, frame.area(), source, draft)
        }
        Screen::Preview(op) => render_preview_placeholder(frame, frame.area(), op),
        // Stub: full implementation in Task 3 (Plan 03-04)
        Screen::CoAuthorList { .. } => {}
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

fn render_not_implemented(frame: &mut Frame, area: Rect, tag: &str) {
    let msg = format!("'{tag}' flow not implemented yet — press Esc/q to return");
    frame.render_widget(
        Paragraph::new(msg).block(Block::bordered().title("TODO")),
        area,
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
        .block(
            Block::bordered()
                .title(format!("Authors ({} match)", matched.len())),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(if matched.is_empty() { None } else { Some(selected) });
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
) {
    let [header, name_field, email_field, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
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
        Paragraph::new(draft.new_email.as_str())
            .block(Block::bordered().title(email_title).border_style(email_style)),
        email_field,
    );

    // Cursor: place in the focused field
    if name_focused {
        let cursor_x = name_field.x + 1 + draft.new_name.chars().count() as u16;
        let cursor_y = name_field.y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    } else {
        let cursor_x = email_field.x + 1 + draft.new_email.chars().count() as u16;
        let cursor_y = email_field.y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    frame.render_widget(
        Paragraph::new("Tab: switch field   Enter: confirm   Esc: cancel"),
        footer,
    );
}

fn render_preview_placeholder(frame: &mut Frame, area: Rect, op: &PendingOp) {
    // Plan 03-05 REPLACES this body with the real warnings + confirmation render.
    frame.render_widget(
        Paragraph::new(format!(
            "Preview placeholder — Plan 03-05 will render scan results for {:?}",
            op
        ))
        .block(Block::bordered().title("Preview (WIP)")),
        area,
    );
}
