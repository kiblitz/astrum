# Astrum — TUI Code Editor

> **Keep this file up to date.** When a design decision is made, a convention is established, or a known issue is resolved, update this file in the same commit. This is the source of truth for how the codebase should behave and evolve.

## Principles
- **Code cleanliness above all.** Prefer simple, principled solutions over quick patches. If a fix requires special-casing by context (e.g., "if browser then X, else Y"), step back and find the general rule. Avoid accumulating conditionals — when the same bug class recurs, the abstraction is wrong.
- **Understand before changing.** Read the relevant code paths and reason about all states before editing. Add logging to confirm hypotheses rather than guessing.
- **No unprincipled hacks.** If a proper solution isn't clear yet, leave the problem documented here rather than shipping a fragile workaround.

## Git Workflow
- Organize changes into logical commits — group related work, don't dump everything into one commit.
- Never amend, rebase, or force-push existing commits. Treat the commit log as append-only. If a fix is needed for something already committed, make a new commit.
- Commit messages should be concise and describe the "what" and "why."

## Build & Run
```
cargo build
cargo run -- [file...]
```

## Architecture
- **editor.rs** — Main event loop, action dispatch, orchestration
- **renderer.rs** — Ratatui TUI rendering (panes, overlays, status bar)
- **buffer.rs** — Text buffer with cursor, undo/redo, editing operations
- **pane.rs** — Split pane layout (tree of panes, each shows a buffer or welcome screen)
- **input.rs** — Modal input handler (Normal, Insert, Visual, Command modes)
- **keymap.rs** — Trie-based keymap with multi-key sequences
- **config.rs** — KDL config loading with default keybindings
- **syntax.rs** — Tree-sitter incremental syntax highlighting
- **file_browser.rs** — Tree-style file browser overlay on panes
- **swap.rs** — Swap file management and external change detection

## Key Design Decisions

### :q semantics
`:q` on a single pane **always means quit the app** (with double-quit confirmation). It does NOT matter what is currently displayed on the pane — file, file browser, or welcome screen. Do not add browser-specific or buffer-specific branching to `quit_current`. The logic is:
- Multiple panes → close the active pane
- Single pane → confirm double-quit, then exit

Buffer closing (`:bd`, `CloseBuffer` action) is a separate operation from quitting.

### Double-quit gate
`quit_pending` is set on the first `:q` and cleared by any key press that isn't part of the `:q` command flow (`:`, command buffer typing, Enter to execute). This lets `:q :q` work while any other key cancels.

### Overlays vs pane content
Find-file and command palette are **overlays** — they float above pane content and intercept all input. File browsers are **pane content** — they replace the buffer view on a specific pane. Overlays suppress cursor positioning (the `show_cursor` / `has_overlay` pattern in the renderer).

### Cursor management
Ratatui manages cursor visibility. Calling `set_cursor_position` shows the cursor; not calling it hides it. Never use manual `cursor::Show`/`cursor::Hide` after `terminal.draw()` — it fights with ratatui.

### Syntax highlighting
Uses **tree-sitter** for synchronous, incremental parsing. Each edit produces an `EditInfo` (byte offsets + row/col positions) that feeds `tree.edit()` + `parser.parse(source, Some(&old_tree))` for sub-millisecond re-parses. Full re-parse is used for undo/redo/paste where the tree state is invalidated.

- **HighlightEngine** owns per-buffer `BufferSyntaxState` (parser, tree, query)
- **LanguageRegistry** maps file extensions to grammar crates (rust, python, js, json, toml, c, cpp)
- Files with no grammar get plain (unstyled) highlights
- Color scheme: One Dark palette
- `with_buffer_edit` in editor.rs handles the edit→incremental parse→cache update pipeline

### Swap files and external change detection
Swap files persist unsaved edits to disk for crash recovery. External change detection warns the user if the file on disk was modified by another process.

**Architecture:**
- **SwapManager** (`swap.rs`) owns per-buffer `SwapEntry` state: source path, disk hash, last swap hash, swap file path
- Swap files live in `dirs::data_dir()/astrum/swap/` (not next to the source file)
- Swap file name: BLAKE3 hash of the absolute source path → `<hex>.swp` + `<hex>.meta` (meta stores the original path)
- Swap file content: plain text only (no cursor/undo state)
- **Content hashing:** `Buffer` has a lazy `content_hash: Option<[u8; 32]>` field. Every mutation sets it to `None`. Hash is only computed on demand (swap timer tick or before save) via BLAKE3 over ropey chunk iterator (no full string allocation).
- **Flush frequency:** 2-second tokio interval timer. Only writes if `content_hash != last_swap_hash`.
- **External change detection:** On save, hash the disk file and compare to `disk_hash` recorded at load time. If different, warn and abort (user can `:w!` to force).
- **Recovery:** On startup, scan swap dir for `.meta` files. If source file exists and swap content differs from disk, offer `:recover`.
- **Cleanup:** On normal exit or buffer close, delete the swap file.

**Commands:**
- `:w!` — force save even if external changes detected
- `:recover` — restore buffer content from swap file

## Style
- Inspired by spacemacs/vim. SPC-prefixed key chords for commands.
- Modes: Normal, Insert, Visual, Command (`:` prefix).
