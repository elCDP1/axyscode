use crate::app::{App, AppMode, Focus, PendingAction};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Widget},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Top area: content. Bottom area: per-panel status bar (Help for the
    // tree, Ln/Col for the editor) or a global status message.
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let body = root[0];
    let status = root[1];

    // Panels: tree (with a gap in the middle when the editor is open).
    let panels = if app.editor.is_open() {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Length(2), // central gap, like a Hyprland gap
                Constraint::Percentage(50),
            ])
            .split(body)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(body)
    };

    let tree_area = panels[0];
    render_tree(frame, app, tree_area);

    if let Some(editor_area) = panels.get(2) {
        render_editor(frame, app, *editor_area);
    }

    // Status bars below each panel.
    if let Some(msg) = &app.status_msg {
        let bar = Paragraph::new(format!(" {}", msg))
            .style(Style::default().bg(Color::Rgb(30, 30, 40)).fg(Color::Gray));
        frame.render_widget(bar, status);
    } else {
        render_help_bar(frame, tree_area);
        if let Some(editor_area) = panels.get(2) {
            render_cursor_bar(frame, app, *editor_area);
        }
    }

    if let AppMode::ConfirmDiscard(action) = &app.mode {
        render_confirm_dialog(frame, area, action);
    }
    if app.mode == AppMode::Help {
        render_help_popup(frame, area);
    }
}

// ─── Panel: tree ─────────────────────────────────────────────────────────────

fn render_tree(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Tree;

    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(
        " {} {} ",
        if focused { "▌" } else { " " },
        app.tree.root.display()
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let selected_style = if focused {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::Gray)
    };

    let items: Vec<ListItem> = app
        .tree
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let text = node.display();
            let style = if i == app.tree.cursor {
                selected_style
            } else if node.is_dir {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items);
    let mut state = ListState::default();
    state.select(if app.tree.nodes.is_empty() {
        None
    } else {
        Some(app.tree.cursor)
    });

    frame.render_stateful_widget(list, inner, &mut state);
}

// ─── Panel: editor ───────────────────────────────────────────────────────────

fn render_editor(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Editor;

    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let Some(buf) = app.editor.buf() else {
        let block = Block::default()
            .title(" Editor ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style);
        frame.render_widget(block, area);
        return;
    };

    let dirty_mark = if buf.dirty { " ●" } else { "" };
    let title = format!(
        " {} {}{} ",
        if focused { "▌" } else { " " },
        buf.file_name(),
        dirty_mark
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.mode == AppMode::Search {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        frame.render_widget(&buf.textarea, layout[0]);

        let search_bar = Paragraph::new(format!(" 🔍 {}▌", app.editor.search_query))
            .style(Style::default().bg(Color::Rgb(30, 30, 45)).fg(Color::Yellow));
        frame.render_widget(search_bar, layout[1]);
    } else {
        frame.render_widget(&buf.textarea, inner);
    }
}

// ─── Status bars ─────────────────────────────────────────────────────────────

/// "?: Help" bar below the tree panel. In split view it only occupies the
/// tree's column; on its own it spans the full width.
fn render_help_bar(frame: &mut Frame, tree_area: Rect) {
    let text_width = tree_area.width.saturating_sub(1) as usize;
    let text = format!("{:<width$}", "?: Help", width = text_width);
    let bar = Paragraph::new(text)
        .style(Style::default().bg(Color::Rgb(30, 30, 40)).fg(Color::Cyan));
    frame.render_widget(bar, Rect {
        x: tree_area.x,
        y: tree_area.y + tree_area.height,
        width: tree_area.width,
        height: 1,
    });
}

/// "Ln X  Col Y" bar below the editor panel.
fn render_cursor_bar(frame: &mut Frame, app: &App, editor_area: Rect) {
    let Some(buf) = app.editor.buf() else {
        return;
    };
    let (row, col) = buf.textarea.cursor();
    let text_width = editor_area.width.saturating_sub(1) as usize;
    let text = format!(
        "{:<width$}",
        format!("Ln {}  Col {}", row + 1, col + 1),
        width = text_width
    );
    let bar = Paragraph::new(text)
        .style(Style::default().bg(Color::Rgb(30, 30, 40)).fg(Color::Gray));
    frame.render_widget(bar, Rect {
        x: editor_area.x,
        y: editor_area.y + editor_area.height,
        width: editor_area.width,
        height: 1,
    });
}

// ─── Help popup ──────────────────────────────────────────────────────────────

fn render_help_popup(frame: &mut Frame, area: Rect) {
    let width: u16 = 72;
    let height: u16 = 24;
    let dialog = Rect {
        x: area.width.saturating_sub(width) / 2,
        y: area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    frame.render_widget(Clear, dialog);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(dialog);
    block.render(dialog, frame.buffer_mut());

    let text: Vec<Line> = vec![
        Line::from(Span::styled(
            "Tree",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑/↓            Move cursor"),
        Line::from("  Enter           Expand/collapse dir, or open file"),
        Line::from("  h               Toggle hidden files"),
        Line::from("  ?               Show this help"),
        Line::from("  Ctrl+Tab        Focus editor (if a document is open)"),
        Line::from("  q / Esc         Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Editor",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from("  Arrows          Move cursor"),
        Line::from("  Shift+Arrows    Select"),
        Line::from("  Home/End        Line start/end"),
        Line::from("  Ctrl+C/X/V      Copy / cut / paste"),
        Line::from("  Ctrl+Z/Y        Undo / redo"),
        Line::from("  Ctrl+S          Save"),
        Line::from("  Ctrl+F          Search (Enter: next match, Esc: close)"),
        Line::from("  Ctrl+W          Close editor"),
        Line::from("  Ctrl+Tab        Focus tree"),
        Line::from("  Esc             Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(text).style(Style::default().fg(Color::White));
    frame.render_widget(help, inner);
}

// ─── Confirm dialog (save / don't save / cancel) ────────────────────────────

fn render_confirm_dialog(frame: &mut Frame, area: Rect, action: &PendingAction) {
    let width: u16 = 64;
    let height: u16 = 6;
    let dialog = Rect {
        x: area.width.saturating_sub(width) / 2,
        y: area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    frame.render_widget(Clear, dialog);

    let (confirm_label, discard_label) = match action {
        PendingAction::Quit => ("Save and quit", "Discard changes and quit"),
        PendingAction::CloseEditor => ("Save and close", "Discard changes and close"),
        PendingAction::OpenFile(_) => ("Save and open another", "Discard changes and open"),
    };

    let block = Block::default()
        .title(" Unsaved changes ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let msg = Paragraph::new(vec![
        Line::from(Span::styled(
            "Do you want to save your changes before continuing?",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[Y] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}   ", confirm_label)),
            Span::styled(
                "[N] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}   ", discard_label)),
            Span::styled(
                "[C] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Cancel"),
        ]),
    ]);
    frame.render_widget(msg, inner);
}