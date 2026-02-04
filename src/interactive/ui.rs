use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui_toolkit::TreeView;

use super::app::{App, Mode, TreeItem};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let title = Line::from(vec![
        Span::styled("scte35-editor", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::raw("Interactive"),
    ]);

    let tree = TreeView::new(app.tree_nodes.clone())
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue))
        .render_fn(|item: &TreeItem, _state| Line::from(item.label.clone()));
    frame.render_stateful_widget(tree, chunks[0], &mut app.tree_state);

    let input = match app.mode {
        Mode::Edit => format!("edit: {}", app.input_buffer),
        Mode::Select => selection_line(app),
        Mode::ConfirmWrite => "confirm write: y/n".to_string(),
        Mode::Browse => "browse: press E to edit, W to write, Q/Esc to exit".to_string(),
    };
    let input_paragraph =
        Paragraph::new(input).block(Block::default().title("Input").borders(Borders::ALL));
    frame.render_widget(input_paragraph, chunks[1]);

    let status = Paragraph::new(status_line(app))
        .block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(status, chunks[2]);

    let meta_text = selection_meta_text(app);
    let meta =
        Paragraph::new(meta_text).block(Block::default().title("Selection").borders(Borders::ALL));
    frame.render_widget(meta, chunks[3]);

    let footer =
        Paragraph::new(footer_hints()).block(Block::default().title("Hints").borders(Borders::ALL));
    frame.render_widget(footer, chunks[4]);
}

fn selection_meta_text(app: &App) -> String {
    let Some(item) = app.selected_item() else {
        return "no selection".to_string();
    };
    let path = item.path.as_deref().unwrap_or("-");
    let value = item.value.as_deref().unwrap_or("-");
    let (value_type, constraints, example) = match item.path.as_deref() {
        Some(path) => match crate::core::patch_meta(path) {
            Some(meta) => (
                format!("{:?}", meta.value_type),
                meta.constraints.to_string(),
                meta.example.to_string(),
            ),
            None => ("-".to_string(), "-".to_string(), "-".to_string()),
        },
        None => ("-".to_string(), "-".to_string(), "-".to_string()),
    };
    format!(
        "path: {path}\nvalue: {value}\ntype: {value_type}\nconstraints: {constraints}\nexample: {example}"
    )
}

fn status_line(app: &App) -> String {
    let selected = app
        .selected_item()
        .and_then(|item| item.path.as_deref())
        .unwrap_or("-");
    if app.status.is_empty() {
        format!("selected: {selected}")
    } else {
        format!("{} | selected: {}", app.status, selected)
    }
}

fn footer_hints() -> String {
    "Arrows navigate | E edit | C create | D delete | W write | Q/Esc exit".to_string()
}

fn selection_line(app: &App) -> String {
    if app.select_options.is_empty() {
        return "select: no options".to_string();
    }
    let value = app
        .select_options
        .get(app.select_index)
        .map(|value| value.as_str())
        .unwrap_or("-");
    format!("select: {} (Up/Down, Enter apply, Esc cancel)", value)
}
