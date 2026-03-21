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
- **syntax.rs** — Async syntax highlighting via syntect + spawn_blocking
- **file_browser.rs** — Tree-style file browser overlay on panes

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

### Syntax highlighting (known issue)
Per-keystroke `spawn_blocking` for syntax highlighting causes lag on expensive grammars (e.g., markdown). The root cause is confirmed but not yet fixed. Do not apply unprincipled hacks (debouncing, generation counters) — a proper solution is needed.

## Style
- Inspired by spacemacs/vim. SPC-prefixed key chords for commands.
- Modes: Normal, Insert, Visual, Command (`:` prefix).
