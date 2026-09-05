use anyhow::{Context, Result};
use ratatui::style::{Color, Modifier, Style};
use std::{fs, path::PathBuf};
use tui_textarea::TextArea;

/// The open document with its editing state.
/// We use TextArea<'static> to avoid propagating lifetimes across the app
/// (we don't put any Block inside the TextArea; we paint it from ui.rs).
pub struct Buffer {
    pub path: PathBuf,
    pub textarea: TextArea<'static>,
    pub dirty: bool,
}

impl Buffer {
    pub fn open(path: PathBuf) -> Result<Self> {
// IMPORTANT: if reading fails (permissions, broken symlink, etc.) we must
// not swallow the error with unwrap_or_default(): that would open the buffer
// as if the file were empty, and a later Ctrl+S would overwrite the real
// file with empty content. We propagate the error so the caller decides
// (show a message, don't open).
let content = fs::read_to_string(&path)
    .with_context(|| format!("could not read {}", path.display()))?;
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };

        let mut textarea = TextArea::new(lines);

        // Cursor style
        textarea.set_cursor_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
        // Line where the cursor is: slightly different background
        textarea.set_cursor_line_style(Style::default().bg(Color::Rgb(40, 40, 55)));
        // Search highlight
        textarea.set_search_style(Style::default().bg(Color::Yellow).fg(Color::Black));
        // Line numbers on the left
        textarea.set_line_number_style(Style::default().fg(Color::DarkGray));

        Ok(Self {
            path,
            textarea,
            dirty: false,
        })
    }

    pub fn save(&mut self) -> Result<()> {
        let content = self.textarea.lines().join("\n");
        fs::write(&self.path, content)?;
        self.dirty = false;
        Ok(())
    }

    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }
}

/// Only one document is allowed open at a time: opening a new one replaces
/// the previous. The caller (input.rs) decides whether it needs to confirm
/// unsaved changes before invoking `open_file`/`close`.
pub struct Editor {
    pub buffer: Option<Buffer>,
    /// Active search query (empty = no search)
    pub search_query: String,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: None,
            search_query: String::new(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.buffer.is_some()
    }

    pub fn has_unsaved(&self) -> bool {
        self.buffer.as_ref().is_some_and(|b| b.dirty)
    }

    /// Is the open document exactly this file?
    pub fn is_open_path(&self, path: &std::path::Path) -> bool {
        self.buffer.as_ref().is_some_and(|b| b.path == path)
    }

    /// Opens (replacing the current document with) the given file.
    /// If it already is the open file, does nothing.
    pub fn open_file(&mut self, path: PathBuf) -> Result<()> {
        if self.is_open_path(&path) {
            return Ok(());
        }
        let buf = Buffer::open(path)?;
        self.buffer = Some(buf);
        Ok(())
    }

    /// Closes the current document. Does not check for unsaved changes: that
    /// decision is made beforehand, in input.rs.
    pub fn close(&mut self) {
        self.buffer = None;
        self.search_query.clear();
    }

    pub fn buf(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    pub fn buf_mut(&mut self) -> Option<&mut Buffer> {
        self.buffer.as_mut()
    }

    pub fn save_current(&mut self) -> Result<()> {
        if let Some(buf) = &mut self.buffer {
            buf.save()?;
        }
        Ok(())
    }
}
