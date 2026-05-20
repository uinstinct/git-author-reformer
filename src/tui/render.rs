use crate::tui::app::{App, MenuChoice, Screen};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

pub fn render(frame: &mut Frame, app: &App) {
    match &app.screen {
        Screen::MainMenu { selected } => render_main_menu(frame, frame.area(), *selected),
        Screen::NotImplemented(tag) => render_not_implemented(frame, frame.area(), tag),
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
