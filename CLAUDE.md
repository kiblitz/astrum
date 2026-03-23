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
cargo test
UPDATE_EXPECT=1 cargo test   # auto-update expect test snapshots
```

## Testing
Use **expect tests** (`expect-test` crate) for buffer operations, motions, and any logic that can be exercised without a terminal. Tests live in `tests/` as integration tests (e.g. `tests/buffer_tests.rs`). The crate exposes a library target (`src/lib.rs`) so integration tests can import `astrum::buffer::Buffer` etc. When adding or fixing buffer/motion/editing behavior, add or update expect tests to cover the change. This is the primary regression safety net.

**Every bug fix or behavior change must include tests in the same commit.** Do not wait to be asked — tests are mandatory, not optional.

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

### Search (`/`, `?`, `n`, `N`)
Per-buffer incremental search with live highlighting.

- **SearchState** in editor.rs owns `last_pattern`, `last_direction`, and `buffer_matches: HashMap<usize, BufferSearchMatches>`
- Each `BufferSearchMatches` stores `matches: Vec<(usize, usize, usize)>` (line, start_col, end_col) and `current_match: Option<usize>`
- Matches are **per-buffer**, not per-pane — switching panes preserves highlights for each file
- **Live search**: while in Search mode, matches are recomputed on every keystroke for the active buffer
- Renderer applies yellow background to all matches, orange to the current match (via `overlay_search_highlights`)
- Empty pattern on Enter repeats last search (`SearchNext`)
- `Mode::Search` in input.rs handles search buffer input; Esc cancels, Enter executes

### Search & replace (`:s`, `:%s`)
Vim-style substitute command.

- **Syntax**: `:s/pattern/replacement/[flags]` (current line) or `:%s/pattern/replacement/[flags]` (whole file)
- **Flags**: `g` replaces all occurrences per line (without it, only first match); `c` confirms each replacement interactively (y/n/a/q)
- **Delimiter**: first non-alphanumeric, non-space char after `s` (usually `/`, but any char works e.g. `s#pat#rep#`)
- `parse_substitute()` in editor.rs parses the command string into a `Substitute` struct
- `execute_substitute()` applies replacements in reverse order (lines then positions) to preserve offsets
- Non-overlapping matches (unlike search highlighting which shows overlapping matches)
- Single undo snapshot for the entire substitute operation

### Operator-pending state (`d`, `c`, `y`)
Vim-style operator + motion composition.

- `d`, `c`, `y` are **leaf bindings** in the keymap (not trie branches). They enter operator-pending state.
- `pending_operator: Option<(Action, usize)>` in editor.rs stores `(operator_action, count)`
- When a motion arrives while operator is pending, `execute_operator_motion()` computes the range:
  - Saves cursor, applies motion `count` times, reads new cursor, restores cursor
  - **Linewise motions** (`is_linewise_motion()`) expand to full line ranges
  - **Characterwise motions** use `char_idx_at()` for rope char index conversion
- Same operator repeated = linewise (dd, cc, yy) via `execute_linewise_operator()`
- Counts multiply: `2d3w` = delete 6 words (op_count * motion_count)
- Buffer methods: `delete_char_range(start, end)`, `text_in_char_range(start, end)`, `char_idx_at(line, col)`

### Word motions and char classes
Three character classes: **Word** (alphanumeric + `_`), **Punctuation** (non-blank non-word), **Whitespace**. Each class forms its own "word" — `w` on `main()` stops at `(`, not after `)`.

- `w`: skip current class → skip whitespace → land on next non-whitespace
- `b`: skip whitespace backward → skip class backward → land at start
- `e`: skip whitespace → advance through class → land at end

### Exclusive vs inclusive motions
Operator ranges respect vim's exclusive/inclusive semantics (`is_exclusive_motion()` on Action):
- **Exclusive** (`w`, `W`, `b`, `B`, `h`, `l`, `0`): destination char NOT included in range
- **Inclusive** (`e`, `E`, `$`): destination char IS included
- `apply_motion()` is the single dispatch table for all motion→buffer mappings

### Count prefix
- Digits accumulate in `input.count_prefix` in input.rs (normal mode only, `0` only counts if prefix already started)
- **Critical**: `execute_action` returns early for `Action::Noop` to avoid consuming the count before the actual command arrives (digits produce Noop while accumulating)
- `input.count_prefix` is consumed at the top of `execute_action` via `.take()`, not in input.rs
- All movement actions loop `count` times

### Jump history
Per-pane jump history with back/forward stacks.

- `jump_history: HashMap<usize, (Vec<JumpPosition>, Vec<JumpPosition>)>` — keyed by pane ID
- `JumpPosition` is either `Buffer { buffer_id, line, col }` or `Browser { dir, selected, scroll_offset }`
- History is **copied** when splitting a pane (user decision)
- History is removed when a pane is closed
- `push_jump()` records current position before navigation events (opening files, switching buffers, opening browser)
- `jump_back`/`jump_forward` only push current position to opposite stack if destination exists (prevents stack growth at end)

### Macros
Vim-style macro recording and playback.

- `q<reg>` starts recording into register `<reg>` (a-z), `q` stops recording
- `@<reg>` replays the macro in register `<reg>`, `@@` replays the last played macro
- **Records `Action` variants**, not raw key events — replay is trivial via `execute_action()`
- Counted actions (e.g. `3j`) are stored as 3 copies of the action for faithful replay
- `RecordMacro` and `PlayMacro` are never recorded into the macro buffer
- Recording indicator shown in status line (red "recording @a")
- `macro_registers: HashMap<char, Vec<Action>>` stores completed macros
- `recording_macro: Option<(char, Vec<Action>)>` holds the in-progress recording
- `last_macro_register: Option<char>` enables `@@`
- Uses `awaiting_char` + `awaiting_macro_record`/`awaiting_macro_play` flags for register selection
- `play_macro()` uses `Box::pin` to handle async recursion (macros can contain `@` to nest)

### Comment toggle (`SPC ; ;`)
Comment toggling for any file type.

- **Keybinding**: `SPC ; ;` in both normal and visual mode
- **Normal mode**: toggles line comment on the current line using `line` prefix (e.g. `//`, `#`)
- **Visual mode**: toggles block comment around the selection using `block` delimiters (e.g. `/* */`, `<!-- -->`); falls back to line comments if no block syntax defined
- **Comment syntax**: per-extension lookup from `comment_syntax: HashMap<String, CommentSyntax>` in Editor
- **Config**: `CommentSyntax { line: Option<String>, block: Option<(String, String)> }`
- **Default languages**: ~40 languages built into `default_comment_syntax()` in config.rs (C-family, Python, Ruby, Lua, Haskell, HTML, etc.)
- **User-configurable**: KDL config `languages { }` block lets users add/override comment syntax per extension
- **Line toggle logic**: if all non-empty lines in range are commented → uncomment; otherwise → comment all
- **Block toggle logic**: if selection starts with open and ends with close delimiter → unwrap; otherwise → wrap
- **Comment insertion**: uses minimum indentation of non-empty lines as insertion point (aligns prefixes)
- **Uncomment removal**: strips prefix + optional trailing space, preserving surrounding indentation
- **Empty lines**: skipped during line comment/uncomment (never modified)
- **Undo**: single `save_undo()` for the entire operation (atomic)
- **Buffer methods**: `toggle_line_comment(first_line, last_line, prefix)`, `toggle_block_comment(start, end, open, close)` in buffer.rs

## Style
- Inspired by spacemacs/vim. SPC-prefixed key chords for commands.
- Modes: Normal, Insert, Visual, Command, Search (`:` prefix for command, `/`/`?` for search).
