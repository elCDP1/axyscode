use crate::app::{App, AppMode, Focus, PendingAction};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_textarea::{CursorMove, Input, Key};

/// Returns `true` to signal the app should close.
pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    app.status_msg = None;

    match app.mode.clone() {
        AppMode::Help => {
            app.mode = AppMode::Normal;
            Ok(false)
        }
        AppMode::ConfirmDiscard(action) => handle_confirm(app, key, action),
        AppMode::Search => handle_search(app, key),
        AppMode::Normal => match app.focus {
            Focus::Tree => handle_tree(app, key),
            Focus::Editor => handle_editor(app, key),
        },
    }
}

// ─── ConfirmDiscard mode ────────────────────────────────────────────────────

fn handle_confirm(app: &mut App, key: KeyEvent, action: PendingAction) -> Result<bool> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            match app.editor.save_current() {
                Ok(()) => apply_pending_action(app, action),
                Err(e) => {
                    app.mode = AppMode::Normal;
                    app.status_msg = Some(format!("Could not save: {e}"));
                    Ok(false)
                }
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') => apply_pending_action(app, action),
        KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(false)
        }
        _ => Ok(false),
    }
}

/// Executes the pending action (either after saving, or after the user
/// decided to discard changes).
fn apply_pending_action(app: &mut App, action: PendingAction) -> Result<bool> {
    app.mode = AppMode::Normal;
    match action {
        PendingAction::Quit => Ok(true),
        PendingAction::CloseEditor => {
            app.editor.close();
            app.focus = Focus::Tree;
            Ok(false)
        }
        PendingAction::OpenFile(path) => {
            match app.editor.open_file(path) {
                Ok(()) => app.focus = Focus::Editor,
                Err(e) => app.status_msg = Some(format!("Could not open file: {e}")),
            }
            Ok(false)
        }
    }
}

// ─── Search mode ────────────────────────────────────────────────────────────

fn handle_search(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.editor.search_query.clear();
            if let Some(buf) = app.editor.buf_mut() {
                let _ = buf.textarea.set_search_pattern("");
            }
        }
        KeyCode::Enter => {
            if let Some(buf) = app.editor.buf_mut() {
                if !buf.textarea.search_forward(false) {
                    app.status_msg = Some("No more matches".to_string());
                }
            }
        }
        KeyCode::Backspace => {
            app.editor.search_query.pop();
            apply_search_pattern(app);
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.editor.search_query.push(c);
            apply_search_pattern(app);
        }
        _ => {}
    }
    Ok(false)
}

fn apply_search_pattern(app: &mut App) {
    let query = app.editor.search_query.clone();
    if let Some(buf) = app.editor.buf_mut() {
        let _ = buf.textarea.set_search_pattern(&query);
    }
}

// ─── Focus: Tree ────────────────────────────────────────────────────────────

fn handle_tree(app: &mut App, key: KeyEvent) -> Result<bool> {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Up) => {
            app.tree.move_up();
        }
        (KeyModifiers::NONE, KeyCode::Down) => {
            app.tree.move_down();
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            if let Some(node) = app.tree.current_node() {
                if node.is_dir {
                    if let Err(e) = app.tree.toggle_node() {
                        app.status_msg = Some(format!("Could not open directory: {e}"));
                    }
                } else {
                    let path = node.path.clone();
                    if app.editor.is_open_path(&path) {
                        app.focus = Focus::Editor;
                    } else if is_text_file(&path) {
                        if app.editor.has_unsaved() {
                            app.mode = AppMode::ConfirmDiscard(PendingAction::OpenFile(path));
                        } else {
                            match app.editor.open_file(path) {
                                Ok(()) => app.focus = Focus::Editor,
                                Err(e) => {
                                    app.status_msg =
                                        Some(format!("Could not open file: {e}"))
                                }
                            }
                        }
                    } else {
                        app.status_msg = Some("Binary file: cannot edit".to_string());
                    }
                }
            }
        }
        (KeyModifiers::NONE, KeyCode::Char('h')) => {
            if let Err(e) = app.tree.toggle_hidden() {
                app.status_msg = Some(format!("Could not read directory: {e}"));
            }
        }
        (KeyModifiers::NONE, KeyCode::Char('?')) => {
            app.mode = AppMode::Help;
        }
        (KeyModifiers::CONTROL, KeyCode::Tab) => {
            if app.editor.is_open() {
                app.focus = Focus::Editor;
            } else {
                app.status_msg = Some("No document open".to_string());
            }
        }
        (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Esc) => {
            return trigger_quit(app);
        }
        _ => {}
    }
    Ok(false)
}

// ─── Focus: Editor ──────────────────────────────────────────────────────────

fn handle_editor(app: &mut App, key: KeyEvent) -> Result<bool> {
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Tab) => {
            app.focus = Focus::Tree;
        }

        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            if app.editor.has_unsaved() {
                app.mode = AppMode::ConfirmDiscard(PendingAction::CloseEditor);
            } else {
                app.editor.close();
                app.focus = Focus::Tree;
            }
        }

        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            app.status_msg = match app.editor.save_current() {
                Ok(()) => Some("Saved.".to_string()),
                Err(e) => Some(format!("Could not save: {e}")),
            };
        }

        (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
            app.mode = AppMode::Search;
            app.editor.search_query.clear();
        }

        (KeyModifiers::CONTROL, KeyCode::Char('z')) => {
            if let Some(buf) = app.editor.buf_mut() {
                buf.textarea.undo();
            }
        }
        (KeyModifiers::CONTROL, KeyCode::Char('y')) => {
            if let Some(buf) = app.editor.buf_mut() {
                buf.textarea.redo();
            }
        }

        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            if let Some(buf) = app.editor.buf_mut() {
                buf.textarea.copy();
            }
        }

        (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
            if let Some(buf) = app.editor.buf_mut() {
                buf.textarea.copy();
                buf.textarea.input(Input {
                    key: Key::Delete,
                    ctrl: false,
                    alt: false,
                    shift: false,
                });
                buf.dirty = true;
            }
        }

        (KeyModifiers::CONTROL, KeyCode::Char('v')) => {
            if let Some(buf) = app.editor.buf_mut() {
                let modified = buf.textarea.paste();
                if modified {
                    buf.dirty = true;
                }
            }
        }

        (KeyModifiers::NONE, KeyCode::Esc) => {
            return trigger_quit(app);
        }

        (KeyModifiers::NONE, KeyCode::Left) => move_cursor(app, CursorMove::Back, false),
        (KeyModifiers::NONE, KeyCode::Right) => move_cursor(app, CursorMove::Forward, false),
        (KeyModifiers::NONE, KeyCode::Up) => move_cursor(app, CursorMove::Up, false),
        (KeyModifiers::NONE, KeyCode::Down) => move_cursor(app, CursorMove::Down, false),
        (KeyModifiers::NONE, KeyCode::Home) => move_cursor(app, CursorMove::Head, false),
        (KeyModifiers::NONE, KeyCode::End) => move_cursor(app, CursorMove::End, false),

        (KeyModifiers::SHIFT, KeyCode::Left) => move_cursor(app, CursorMove::Back, true),
        (KeyModifiers::SHIFT, KeyCode::Right) => move_cursor(app, CursorMove::Forward, true),
        (KeyModifiers::SHIFT, KeyCode::Up) => move_cursor(app, CursorMove::Up, true),
        (KeyModifiers::SHIFT, KeyCode::Down) => move_cursor(app, CursorMove::Down, true),
        (KeyModifiers::SHIFT, KeyCode::Home) => move_cursor(app, CursorMove::Head, true),
        (KeyModifiers::SHIFT, KeyCode::End) => move_cursor(app, CursorMove::End, true),

        _ => {
            if let Some(buf) = app.editor.buf_mut() {
                let modified = buf.textarea.input_without_shortcuts(key);
                if modified {
                    buf.dirty = true;
                }
            }
        }
    }
    Ok(false)
}

fn move_cursor(app: &mut App, m: CursorMove, extend_selection: bool) {
    if let Some(buf) = app.editor.buf_mut() {
        if extend_selection {
            if !buf.textarea.is_selecting() {
                buf.textarea.start_selection();
            }
        } else {
            buf.textarea.cancel_selection();
        }
        buf.textarea.move_cursor(m);
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn trigger_quit(app: &mut App) -> Result<bool> {
    if app.editor.has_unsaved() {
        app.mode = AppMode::ConfirmDiscard(PendingAction::Quit);
        Ok(false)
    } else {
        Ok(true)
    }
}

/// Simple heuristic: if the first 512 bytes have no nulls, it's text.
fn is_text_file(path: &std::path::Path) -> bool {
    use std::io::Read;

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 512];
    match file.read(&mut buf) {
        Ok(n) => !buf[..n].contains(&0u8),
        Err(_) => false,
    }
}
