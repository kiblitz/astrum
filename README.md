# Astrum

A terminal code editor inspired by Spacemacs and Vim, built in Rust.

## Features

- **Modal editing** — Vim-style normal, insert, visual, and command modes
- **Spacemacs-style leader keys** — `SPC f f` to browse files, `SPC w /` to split, etc.
- **Trie-based keymap** — key chains with which-key hints, fully configurable via TOML
- **N-ary pane splits** — split vertically/horizontally with equal distribution (not nested binary halves)
- **Pane movement** — tree-restructuring moves, not content swaps
- **Syntax highlighting** — async via syntect, doesn't block the event loop
- **System clipboard** — yank/paste synced with OS clipboard
- **File browser** — per-pane directory navigation with filtering and file creation
- **Jump history** — navigate back/forward across buffers and directories

## Architecture

```
main.rs           Entry point — tokio runtime, config load, Editor::run()
  |
editor.rs         Async event loop and action dispatch
  |               Owns all state: buffers, pane layout, file browsers, input handler
  |
  +-- input.rs    Key event normalization and trie-based lookup
  |     |         Handles pending key chains, count prefixes, mode dispatch
  |     |
  |     +-- keymap.rs    Trie data structure (ModeKeymap per vim mode)
  |                      Collision-detected bind(), lookup via KeyInput
  |
  +-- action.rs   Flat enum of every editor operation
  |               Unit variants = configurable, data variants = runtime-only
  |
  +-- buffer.rs   Text storage (Vec<String>), cursor, scroll, undo/redo
  |               All text manipulation: insert, delete, movement, clipboard
  |
  +-- pane.rs     N-ary layout tree (Leaf | Split { direction, children })
  |               Directional focus (geometry-based overlap check)
  |               Tree-restructuring pane moves, normalize after mutation
  |
  +-- file_browser.rs   Per-pane directory browser
  |                     Navigate / Filter / NewFile input modes
  |
  +-- renderer.rs       ratatui-based rendering
  |                     Recursive pane tree traversal, syntax-highlighted buffers
  |                     Status bar, command line, which-key hints
  |
  +-- config.rs   TOML config (~/.config/astrum/config.toml)
  |               Default spacemacs keymap, user overlay via trie merge
  |
  +-- syntax.rs   Async syntax highlighting via syntect
                  Channel-based: request → highlight → cache
```

### Data flow

1. **Key press** arrives via crossterm's async event stream
2. **InputHandler** normalizes it to `KeyInput`, walks the active mode's trie
3. Trie yields an `Action` (or stores pending state for multi-key chains)
4. **Editor::execute_action()** dispatches — mutates buffers, pane tree, browsers, etc.
5. **Renderer** reads the full editor state and draws the frame

### Pane layout

The pane tree is n-ary: splitting a pane that's already in a same-direction split adds a sibling (3 equal columns), not a nested binary split (1/2 + 1/4 + 1/4). Moving a pane restructures the tree — removing it from one position and reinserting at another — then normalizes (collapses single-child nodes, merges same-direction parent/child).

## Configuration

Config lives at `~/.config/astrum/config.toml` (created with defaults on first run).

```toml
[general]
theme = "base16-eighties.dark"

[keymap.normal]
"h" = "MoveLeft"
"j" = "MoveDown"
"space f f" = "OpenFileBrowser"
"d d" = "DeleteLine"
# ...
```

Key notation: `C-a` (ctrl), `M-a` (alt), `space`, `esc`, `enter`, `tab`, arrow keys. Chains are space-separated: `"g g"`, `"space w /"`.

## Building

```
cargo build --release
```

## Usage

```
astrum [file ...]
```
