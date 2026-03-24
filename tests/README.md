# Test Suite

All tests use the `expect-test` crate for snapshot-based assertions. Run `UPDATE_EXPECT=1 cargo test` to auto-update snapshots after intentional changes.

## Directory Structure

```
tests/
  vim/              Buffer operations, motions, editing (unit-level)
    main.rs
    comment_tests.rs
    editing_tests.rs
    motion_tests.rs
    operator_tests.rs
    search_tests.rs
    substitute_tests.rs
  ui/               Renderer snapshot tests (ratatui TestBackend)
    main.rs
    helpers.rs
    welcome.rs
    tab_bar.rs
    status_line.rs
    command_line.rs
    editor_content.rs
    pane_layout.rs
    file_browser.rs
    find_file.rs
    palette.rs
    recent_picker.rs
    integration.rs
```

## UI Tests (`tests/ui/`)

UI tests render the full TUI frame using ratatui's `TestBackend`, extract the cell grid as plain text, and compare against expect-test snapshots. This catches layout, padding, alignment, and content issues that buffer-level tests cannot.

### helpers.rs
Foundation for all UI tests. Provides:
- `RenderState` — builder struct with defaults for all 21 `renderer.render()` parameters. Builder methods: `.with_buffer()`, `.with_extra_buffer()`, `.with_mode()`, `.with_command()`, `.with_search()`, `.with_visual_anchor()`, `.with_recording_macro()`, `.with_config_error()`, `.with_pending_keys()`, `.with_file_browser()`, `.with_palette()`, `.with_find_file()`, `.with_recent_picker()`, `.with_search_matches()`, `.with_substitute_highlight()`
- `render_to_string(width, height, &state)` — renders a frame and returns the cell grid as a string
- `check(actual, expect)` — expect-test assertion wrapper
- `make_buffer(text, name)` / `make_buffer_at(text, name, line, col)` — quick buffer constructors
- `make_dir_entry(name, is_dir, size)` — DirEntry for browser/find-file tests
- `make_palette(items)` / `make_recent_picker(items)` / `make_find_file(dir, entries)` — overlay state constructors

### welcome.rs (4 tests)
- `welcome_basic_80x24` — standard welcome screen with ASCII art, version, shortcuts
- `welcome_with_config_error` — config error message displayed below shortcuts
- `welcome_narrow_terminal` — 40x12 terminal, content truncated gracefully
- `welcome_very_short_terminal` — 80x6 terminal, ASCII art and shortcuts omitted

### tab_bar.rs (7 tests)
- `tab_bar_no_buffers` — no tab bar on welcome screen
- `tab_bar_single_buffer` — single tab with filename
- `tab_bar_single_modified` — modified marker `[+]` in tab
- `tab_bar_multiple_buffers` — multiple tabs separated by `│`
- `tab_bar_long_filename` — long filename doesn't overflow
- `tab_bar_scratch_buffer` — `[scratch]` name for unnamed buffers
- `tab_bar_modified_and_unmodified_mix` — mixed modified state across tabs

### status_line.rs (10 tests)
- `status_normal_mode` — `NORMAL` indicator with filename and cursor position
- `status_insert_mode` — `INSERT` indicator
- `status_visual_mode` — `VISUAL` indicator
- `status_command_mode` — `COMMAND` indicator, command line shows `:w`
- `status_search_mode` — `SEARCH` indicator, command line shows `/hello`
- `status_modified_file` — `[+]` in status line for modified buffer
- `status_scratch_buffer` — `[scratch]` in status line
- `status_cursor_position_middle` — cursor position display at non-origin
- `status_welcome_screen` — `Welcome` in place of filename
- `status_narrow_terminal` — 40x12 terminal, status truncated

### command_line.rs (10 tests)
- `command_default_help` — default help text in normal mode
- `command_mode_prefix` — `:` prefix with command text
- `command_empty_command` — just `:` with no text
- `command_search_forward` — `/` prefix for forward search
- `command_search_backward` — `?` prefix for backward search
- `command_status_message` — status message replaces help text
- `command_which_key_hints` — which-key popup with pending key hints
- `command_many_hints` — many hints to test overflow/layout
- `command_recording_macro` — recording indicator `recording @a`
- `command_recording_and_status` — recording indicator alongside status message

### editor_content.rs (12 tests)
- `content_single_line` — one line with line number
- `content_multiple_lines` — several lines with line numbers
- `content_empty_buffer` — empty buffer shows line 1 empty
- `content_tilde_past_eof` — `~` markers for lines past end of file
- `content_line_number_width` — line number gutter width adapts to file size
- `content_wide_line_numbers` — 4-digit line numbers
- `content_current_line_highlight` — cursor line distinguished
- `content_scroll_offset` — scrolled view with correct line numbers
- `content_long_line_no_wrap` — long lines truncated, not wrapped
- `content_cursor_at_end` — cursor at last line
- `content_narrow_width` — narrow terminal truncates content
- `content_search_highlights` — search matches visible in content
- `content_visual_selection` — visual mode selection visible

### pane_layout.rs (7 tests)
- `single_pane_full_width` — single pane fills terminal
- `vertical_split_two_panes` — vertical split with separator `│`
- `horizontal_split_two_panes` — horizontal split with separator `─`
- `vertical_split_active_indicator` — active pane indicated in status
- `three_way_vertical_split` — three panes side by side
- `pane_without_buffer` — split pane with no assigned buffer
- `nested_splits` — vertical + horizontal nested split

### file_browser.rs (7 tests)
- `browser_empty_directory` — empty directory message
- `browser_with_entries` — directory listing with dirs first
- `browser_selected_not_first` — non-first item selected
- `browser_large_file_sizes` — file size formatting
- `browser_filtered` — filter mode with query
- `browser_narrow` — 40x12 terminal
- `browser_many_entries_scrolled` — scroll offset with many entries

### find_file.rs (6 tests)
- `find_file_with_entries` — popup with file entries
- `find_file_with_query` — query filtering entries
- `find_file_selected_second` — second item selected
- `find_file_no_matches` — no matches shows create hint
- `find_file_long_path` — long path display
- `find_file_small_terminal` — 40x16 terminal

### palette.rs (5 tests)
- `palette_all_items` — all palette items visible
- `palette_with_query` — query filtering items
- `palette_selected_second` — second item selected
- `palette_many_items` — many items to test scroll
- `palette_small_terminal` — 40x16 terminal

### recent_picker.rs (6 tests)
- `recent_empty_list` — "No recent files" message
- `recent_with_files` — files listed with paths
- `recent_with_dirs` — directories shown with `/` suffix
- `recent_selected_second` — second item selected
- `recent_with_query` — query filtering results
- `recent_small_terminal` — 40x12 terminal

### integration.rs (8 tests)
- `full_frame_welcome` — complete 80x24 welcome screen
- `full_frame_single_buffer` — complete frame with one buffer open
- `full_frame_two_panes_vertical` — vertical split full frame
- `full_frame_command_mode` — command mode full frame
- `full_frame_search_with_matches` — search mode with highlights
- `full_frame_visual_selection` — visual mode full frame
- `full_frame_file_browser` — file browser full frame
- `full_frame_find_file_overlay` — find-file overlay on buffer

## Vim Tests (`tests/vim/`)

Buffer-level tests that exercise motions, operators, editing commands, search, substitution, and comments without any rendering.

### comment_tests.rs
Line and block comment toggling for various languages.

### editing_tests.rs
Insert/delete/undo/redo, paste, text object operations.

### motion_tests.rs
Cursor movement: `h`, `j`, `k`, `l`, `w`, `b`, `e`, `W`, `B`, `E`, `0`, `$`, `gg`, `G`, `f`, `t`, `%`, etc.

### operator_tests.rs
Operator + motion composition: `d`, `c`, `y` with various motions, linewise operators, count multiplication.

### search_tests.rs
`/`, `?`, `n`, `N` search with match tracking.

### substitute_tests.rs
`:s` and `:%s` with flags (`g`, `c`), delimiter variations, edge cases.
