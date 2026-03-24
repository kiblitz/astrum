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

### welcome.rs (8 tests)
- `welcome_basic_80x24` — standard welcome screen with ASCII art, version, shortcuts
- `welcome_with_config_error` — config error message displayed below shortcuts
- `welcome_narrow_terminal` — 40x12 terminal, content truncated gracefully
- `welcome_very_short_terminal` — 80x6 terminal, ASCII art and shortcuts omitted
- `welcome_very_wide_terminal` — 160-column terminal, centered content
- `welcome_exactly_logo_width` — 50-column terminal, just wider than ASCII logo
- `welcome_config_error_long_message` — long config error string wrapping
- `welcome_minimum_height` — 4-row terminal, minimal layout

### tab_bar.rs (12 tests)
- `tab_bar_no_buffers` — no tab bar on welcome screen
- `tab_bar_single_buffer` — single tab with filename
- `tab_bar_single_modified` — modified marker `[+]` in tab
- `tab_bar_multiple_buffers` — multiple tabs separated by `│`
- `tab_bar_long_filename` — long filename doesn't overflow
- `tab_bar_scratch_buffer` — `[scratch]` name for unnamed buffers
- `tab_bar_modified_and_unmodified_mix` — mixed modified state across tabs
- `tab_bar_all_modified` — multiple buffers all marked modified
- `tab_bar_many_buffers` — five buffers in tab bar
- `tab_bar_no_extension` — filename without extension (Makefile)
- `tab_bar_very_narrow` — 20-column terminal with two tabs
- `tab_bar_single_char_names` — single-character buffer names

### status_line.rs (18 tests)
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
- `status_insert_mode_modified` — INSERT mode with modified flag
- `status_visual_mode_modified` — VISUAL mode with modified flag
- `status_cursor_far_col` — cursor at column 150 on long line
- `status_large_file` — 100-line file, cursor at line 50
- `status_very_long_filename` — filename that could overflow status
- `status_command_mode_modified` — COMMAND mode with modified buffer
- `status_very_narrow_20_cols` — 20-column terminal
- `status_search_backward_mode` — backward search `?` mode

### command_line.rs (18 tests)
- `cmdline_normal_default_help` — default help text in normal mode
- `cmdline_command_mode_prefix` — `:` prefix with command text
- `cmdline_command_mode_empty` — just `:` with no text
- `cmdline_search_forward` — `/` prefix for forward search
- `cmdline_search_backward` — `?` prefix for backward search
- `cmdline_status_message` — status message replaces help text
- `cmdline_which_key_hints` — which-key popup with pending key hints
- `cmdline_which_key_many_hints` — many hints, overflow truncation
- `cmdline_recording_macro` — recording indicator `recording @a`
- `cmdline_recording_plus_status` — recording indicator alongside status message
- `cmdline_very_long_command` — long substitute command
- `cmdline_which_key_single_hint` — single which-key hint
- `cmdline_which_key_narrow` — which-key hints in narrow 30-col terminal
- `cmdline_recording_with_command` — recording + command mode combined
- `cmdline_recording_with_search` — recording + search mode combined
- `cmdline_browser_filter_mode` — file browser filter input
- `cmdline_browser_new_file_mode` — file browser new-file input
- `cmdline_search_empty_buffer` — empty search pattern

### editor_content.rs (24 tests)
- `editor_single_line` — one line with line number
- `editor_multiple_lines` — several lines with line numbers
- `editor_empty_buffer` — empty buffer shows line 1 empty
- `editor_tilde_past_eof` — `~` markers for lines past end of file
- `editor_line_number_width_small` — gutter width for few lines
- `editor_current_line_highlighted` — cursor line distinguished
- `editor_scroll_offset` — scrolled view with correct line numbers
- `editor_long_line_no_wrap` — long lines truncated, not wrapped
- `editor_cursor_at_end` — cursor at end of line
- `editor_narrow_width` — narrow terminal truncates content
- `editor_with_search_highlights` — search matches on single line
- `editor_with_visual_selection` — visual mode selection
- `editor_blank_lines_only` — buffer with only newlines
- `editor_wide_line_numbers` — 3-digit line numbers (150 lines)
- `editor_cursor_middle_of_file` — cursor at line 3, col 2
- `editor_single_char_per_line` — minimal content per line
- `editor_visual_multiline` — visual selection spanning multiple lines
- `editor_visual_reverse_selection` — cursor before anchor (backwards)
- `editor_visual_single_char` — single-character visual selection
- `editor_search_multiple_per_line` — three matches on one line
- `editor_search_on_multiple_lines` — search matches across lines
- `editor_substitute_highlight` — substitute confirmation highlight
- `editor_very_wide_terminal` — 160-column terminal
- `editor_insert_mode_display` — insert mode status line

### pane_layout.rs (13 tests)
- `single_pane_full_width` — single pane fills terminal
- `vertical_split_two_panes` — vertical split with separator `│`
- `horizontal_split_two_panes` — horizontal split with separator `─`
- `vertical_split_active_indicator` — active pane indicated in status
- `three_way_vertical_split` — three panes side by side
- `pane_without_buffer` — split pane with no assigned buffer
- `nested_splits` — vertical + horizontal nested split
- `very_narrow_vertical_split` — 24-col terminal, vertical split
- `very_short_horizontal_split` — 8-row terminal, horizontal split
- `four_way_split` — 2x2 grid of panes
- `split_with_modified_buffers` — modified indicator in split panes
- `split_active_right_pane` — active pane on right side
- `three_horizontal_splits` — three horizontal panes stacked

### file_browser.rs (14 tests)
- `browser_empty_directory` — empty directory message
- `browser_with_entries` — directory listing with dirs first
- `browser_selected_not_first` — non-first item selected
- `browser_large_file_sizes` — file size formatting
- `browser_filtered` — filter mode with query
- `browser_narrow_width` — 30-column terminal
- `browser_many_entries_scrolled` — scroll offset with many entries
- `browser_new_file_mode` — new-file input mode display
- `browser_single_entry` — single file in directory
- `browser_only_dirs` — all entries are directories
- `browser_only_files` — all entries are files
- `browser_zero_byte_file` — 0-byte file size display
- `browser_last_entry_selected` — last entry selected
- `browser_with_buffer_in_split` — browser in split pane with buffer

### find_file.rs (15 tests)
- `find_file_with_entries` — popup with file entries
- `find_file_with_query` — query filtering entries
- `find_file_selected_second` — second item selected
- `find_file_no_matches` — no matches with path auto-selected
- `find_file_long_path` — long path display
- `find_file_small_terminal` — 40x12 terminal
- `find_file_many_entries` — 20 entries with scrolling
- `find_file_selected_last` — last item selected
- `find_file_dirs_and_files` — mixed dirs and files
- `find_file_with_input_and_entries` — query with filtered entries
- `find_file_very_wide_terminal` — 160-column terminal
- `find_file_empty_dir` — empty directory, no entries
- `find_file_path_selected_highlight` — path line highlighted for file creation
- `find_file_path_selected_with_matching_folder` — path selected even when folder matches name
- `find_file_no_matches_path_auto_selected` — path auto-selected when no matches

### palette.rs (11 tests)
- `palette_all_items` — all palette items visible
- `palette_with_query` — query filtering items
- `palette_selected_second` — second item selected
- `palette_many_items` — many items to test scroll
- `palette_small_terminal` — 40x12 terminal
- `palette_single_item` — single action in palette
- `palette_selected_last` — last item selected
- `palette_long_action_name` — long action name layout
- `palette_scrolled_to_bottom` — scrolled to item 25 of 30
- `palette_very_wide_terminal` — 160-column terminal
- `palette_with_long_query` — long query string

### recent_picker.rs (13 tests)
- `recent_empty_list` — "No recent files" message
- `recent_with_files` — files listed with paths
- `recent_with_dirs` — directories shown with `/` suffix
- `recent_selected_second` — second item selected
- `recent_with_query` — query filtering results
- `recent_small_terminal` — 40x12 terminal
- `recent_single_item` — single recent file
- `recent_all_dirs` — all items are directories
- `recent_no_matches_after_filter` — query filters all items away
- `recent_many_items` — 20 recent items
- `recent_selected_last` — last item selected
- `recent_long_paths` — long path display
- `recent_mixed_long_short` — mixed path lengths

### integration.rs (23 tests)
- `full_frame_welcome_80x24` — complete 80x24 welcome screen
- `full_frame_single_buffer` — complete frame with one buffer open
- `full_frame_two_panes_vertical` — vertical split full frame
- `full_frame_command_mode` — command mode full frame
- `full_frame_search_with_matches` — search mode with highlights
- `full_frame_visual_selection` — visual mode full frame
- `full_frame_file_browser` — file browser full frame
- `full_frame_find_file_overlay` — find-file overlay on buffer
- `full_frame_palette_overlay` — palette overlay on buffer
- `full_frame_recent_picker_overlay` — recent picker overlay
- `full_frame_recording_macro` — recording indicator
- `full_frame_status_message` — status message display
- `full_frame_horizontal_split` — horizontal split layout
- `full_frame_insert_mode` — insert mode full frame
- `full_frame_search_mode_active` — active search mode
- `full_frame_which_key_hints` — which-key hints display
- `full_frame_modified_file` — modified file indicator
- `full_frame_many_lines_scrolled` — scrolled 50-line buffer
- `full_frame_config_error_welcome` — config error on welcome
- `full_frame_narrow_40x12` — narrow terminal
- `full_frame_wide_160x24` — wide terminal
- `full_frame_split_with_search` — vertical split with search matches
- `full_frame_browser_in_split` — file browser in split pane

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
