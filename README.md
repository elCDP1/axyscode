# axyscode

TUI file explorer and programmer-oriented text editor in Rust with a `tree`-style file browser on the left and a
lightweight text editor on the right, in a Hyprland-like layout (two
independent panels with a gap between them, not a single shared box).

## Build / run

```
cargo run
```

Opens in the directory where the command is run from.

## Architecture

- `main.rs` — terminal setup/teardown (raw mode, alternate screen).
- `app.rs` — global state (`App`), and enums that control the flow:
  - `Focus`: which panel has focus (`Tree` | `Editor`).
  - `AppMode`: `Normal` | `Search` | `Help` | `ConfirmDiscard(PendingAction)`.
  - `PendingAction`: an action postponed because it would discard unsaved
    changes (`Quit` | `CloseEditor` | `OpenFile(path)`). The Y/N/C dialog
    is generic and works for all three.
- `tree.rs` — `FileTree`: flat list of nodes (`FlatNode`) with indentation
  by `depth`. Expanding/collapsing inserts/removes a contiguous range of
  the list. Reads the directory **before** mutating its own state, so it
  never ends up in a half-consistent state on failure (permissions, broken
  symlink...).
- `editor.rs` — `Buffer` (a document: `TextArea` from `tui-textarea` +
  path + `dirty` flag) and `Editor`, which only stores `Option<Buffer>`:
  **only one document can be open at a time**. Opening another replaces the
  current one. Line numbers enabled via `set_line_number_style` (native
  `tui-textarea` support).
- `input.rs` — all key handling, dispatched by `AppMode` and `Focus`.
  Recoverable I/O errors (permission denied, save failure...) are shown in
  `status_msg` and never propagate to kill the app.
- `ui.rs` — rendering with `ratatui`. Without a document open the tree
  occupies the full screen; when one is opened it splits into two panels
  with a central gap (`Constraint::Length(2)`), each with its own rounded
  border — like two terminals in a tiling layout. Each panel has its own
  bottom bar: "?: Help" for the tree, "Ln/Col" for the editor.

## Keymap

**Tree** (focus on the left panel)
| Key | Action |
|---|---|
| `↑ ↓` | Move cursor |
| `Enter` | Expand/collapse directory, or open the file |
| `h` | Toggle hidden files |
| `?` | Show help with all keybindings |
| `Ctrl+Tab` | Switch focus to the editor (if a document is open) |
| `q` / `Esc` | Quit (with confirmation if there are unsaved changes) |

**Editor** (focus on the right panel)
| Key | Action |
|---|---|
| `↑ ↓ ← →` | Move cursor |
| `Shift+arrows` | Selection |
| `Home / End` | Line start / end |
| `Ctrl+C` | Copy |
| `Ctrl+X` | Cut |
| `Ctrl+V` | Paste |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+S` | Save |
| `Ctrl+F` | Search (Enter = next match, Esc = close search) |
| `Ctrl+W` | Close the editor and return to tree-only view |
| `Ctrl+Tab` | Switch focus to the tree |
| `Esc` | Quit (with confirmation if there are unsaved changes) |

Note: `Esc` always quits the whole program. `Ctrl+W` closes only the
document/editor and returns to the tree view — they are intentionally
different shortcuts.

## Design decisions to remember

- **Only one document open.** No tabs. Opening a new file with unsaved
  changes in the current one triggers the confirmation dialog
  (`PendingAction::OpenFile`).
- **`input_without_shortcuts` does not move the cursor.** It only covers
  text input, tab, backspace, delete and enter. Movement (arrows,
  Home/End, selection with Shift) is implemented by hand in `input.rs`
  with `move_cursor` + `start_selection`/`cancel_selection`, to avoid
  inheriting the Emacs-style bindings that `TextArea::input()` applies by
  default.
- **Never use `unwrap_or_default()` when reading a file for editing.** If
  the read fails, the error must be propagated — treating an unreadable
  file as "empty" and then saving it would overwrite the real file with
  nothing.
- **`tui-textarea` needs the `search` feature explicitly.** Without it,
  `set_search_pattern`, `search_forward` and `set_search_style` don't exist
  (they are behind `#[cfg(feature = "search")]`, which enables the optional
  `regex` dependency). `Cargo.toml` already includes it (`features =
  ["crossterm", "search"]`) — if the `Cargo.toml` is ever recreated by hand,
  don't forget this flag or search will fail to compile with "method not
  found" errors.
- **`main.rs` uses a `TerminalGuard` with `Drop`.** Any `?` that aborts
  startup (reading the root directory, creating the `Terminal`...) still
  restores the user's terminal (raw mode, alternate screen, visible cursor)
  because the `Drop` always runs, no matter what. If more startup logic is
  added in the future there is no need to remember to "clean up at the end"
  — it is already covered.
- **The focused panel is distinguished twice:** the border changes color
  (`Cyan` vs `DarkGray`) and, in the tree, the selected row is dimmed to
  grey when that panel doesn't have focus — like an inactive window in a
  tiling manager.

## Pending / ideas for next steps

- File operations from the tree: rename, delete, create file/directory
  (explicitly deferred for now).
- Packaging for Arch Linux, once the code is mature.

## Build history

First compiled and run successfully with `rustc 1.98.1` (via `rustup` on
Arch Linux/WSL2) in 2026-09. It was debugged by reading real dependency
source in a sandbox with `rustc` 1.75 (which couldn't compile `ratatui`
0.28+ due to the `darling`/`instability` chain requiring 1.88+), so until
that first real compilation a failure went undetected: `tui-textarea` hides
all search functionality (`set_search_pattern`, `search_forward`,
`set_search_style`) behind the optional `search` feature (which enables the
optional `regex` dependency) that was missing from the original `Cargo.toml`.
Fixed — see the corresponding note above.
