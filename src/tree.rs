use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// A visible node in the tree (flat list).
/// Children are inserted/removed from the list as directories expand/collapse.
#[derive(Debug, Clone)]
pub struct FlatNode {
    pub name: String,
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
}

impl FlatNode {
    pub fn from_path(path: PathBuf, depth: usize) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let is_dir = path.is_dir();
        Self {
            name,
            path,
            depth,
            is_dir,
            expanded: false,
        }
    }

    /// Display text with arrow and indentation
    pub fn display(&self) -> String {
        let indent = "  ".repeat(self.depth);
        if self.is_dir {
            let arrow = if self.expanded { "▼ " } else { "▶ " };
            format!("{}{}{}/", indent, arrow, self.name)
        } else {
            format!("{}  {}", indent, self.name)
        }
    }
}

pub struct FileTree {
    pub root: PathBuf,
    /// Flat list of visible nodes
    pub nodes: Vec<FlatNode>,
    pub cursor: usize,
    pub show_hidden: bool,
}

impl FileTree {
    pub fn new(root: PathBuf, show_hidden: bool) -> Result<Self> {
        let nodes = read_dir_sorted(&root, 0, show_hidden)?;
        Ok(Self {
            root,
            nodes,
            cursor: 0,
            show_hidden,
        })
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.nodes.len() {
            self.cursor += 1;
        }
    }

    /// Toggles hidden files and rebuilds the tree (collapsed)
    pub fn toggle_hidden(&mut self) -> Result<()> {
        let new_show_hidden = !self.show_hidden;
        // Same as toggle_node: if re-reading fails, we don't want a
        // half-consistent state (flag changed but tree still showing old data).
        let nodes = read_dir_sorted(&self.root, 0, new_show_hidden)?;
        self.show_hidden = new_show_hidden;
        self.nodes = nodes;
        self.cursor = 0;
        Ok(())
    }

    /// Expands or collapses the directory under the cursor.
    /// Does nothing on a file.
    pub fn toggle_node(&mut self) -> Result<()> {
        let idx = self.cursor;
        let node = &self.nodes[idx];

        if !node.is_dir {
            return Ok(());
        }

        let depth = node.depth;
        let path = node.path.clone();
        let expanding = !node.expanded;

        if expanding {
            // Read the directory BEFORE touching state: if it fails
            // (e.g. permission denied), the node stays as it was instead of
            // being marked "expanded" with no children.
            let children = read_dir_sorted(&path, depth + 1, self.show_hidden)?;
            self.nodes[idx].expanded = true;
            for (i, child) in children.into_iter().enumerate() {
                self.nodes.insert(idx + 1 + i, child);
            }
        } else {
            self.nodes[idx].expanded = false;
            // Remove all descendants (depth > node.depth)
            let end = self.nodes[idx + 1..]
                .iter()
                .position(|n| n.depth <= depth)
                .map(|pos| idx + 1 + pos)
                .unwrap_or(self.nodes.len());
            self.nodes.drain(idx + 1..end);
        }

        Ok(())
    }

    pub fn current_node(&self) -> Option<&FlatNode> {
        self.nodes.get(self.cursor)
    }
}

/// Reads a directory and returns its entries as sorted FlatNode items:
/// directories first, then files, both alphabetically.
fn read_dir_sorted(path: &Path, depth: usize, show_hidden: bool) -> Result<Vec<FlatNode>> {
    let mut entries: Vec<_> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            show_hidden || !e.file_name().to_string_lossy().starts_with('.')
        })
        .collect();

    entries.sort_by(|a, b| {
        let a_dir = a.path().is_dir();
        let b_dir = b.path().is_dir();
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    Ok(entries
        .into_iter()
        .map(|e| FlatNode::from_path(e.path(), depth))
        .collect())
}
