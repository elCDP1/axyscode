use crate::{editor::Editor, input, tree::FileTree, ui};
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Tree,
    Editor,
}

/// An action postponed because it would discard unsaved changes.
#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    Quit,
    CloseEditor,
    OpenFile(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Search,
    Help,
    ConfirmDiscard(PendingAction),
}

pub struct App {
    pub tree: FileTree,
    pub editor: Editor,
    pub focus: Focus,
    pub mode: AppMode,
    /// Temporary status message (bottom bar)
    pub status_msg: Option<String>,
}

impl App {
    pub fn new(root: PathBuf) -> Result<Self> {
        Ok(Self {
            tree: FileTree::new(root, false)?,
            editor: Editor::new(),
            focus: Focus::Tree,
            mode: AppMode::Normal,
            status_msg: None,
        })
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|frame| ui::render(frame, self))?;

            if let Event::Key(key) = event::read()? {
                // Ignore Release events (only process Press and Repeat)
                if key.kind == KeyEventKind::Release {
                    continue;
                }

                if input::handle_key(self, key)? {
                    // true means quit
                    break;
                }
            }
        }
        Ok(())
    }
}
