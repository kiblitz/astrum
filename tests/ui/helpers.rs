use astrum::action::SearchDirection;
use astrum::buffer::Buffer;
use astrum::editor::{
    BufferSearchMatches, FindFileState, PaletteItem, PaletteState, RecentItem, RecentItemKind, RecentPickerState,
    SearchState,
};
use astrum::file_browser::{DirEntry, FileBrowser};
use astrum::input::Mode;
use astrum::pane::{PaneContent, PaneLayout};
use astrum::renderer::Renderer;
use astrum::syntax::HighlightCache;
use astrum::terminal::TerminalSession;
use expect_test::Expect;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use ropey::Rope;
use std::collections::HashMap;
use std::path::PathBuf;

/// All state needed to call `renderer.render()`. Builder pattern with sensible defaults.
pub struct RenderState {
    pub buffers: Vec<Buffer>,
    pub pane_layout: PaneLayout,
    pub mode: Mode,
    pub command_buffer: String,
    pub status_message: String,
    pub pending_keys: String,
    pub pending_hints: Vec<(String, String)>,
    pub highlight_cache: HighlightCache,
    pub palette: Option<PaletteState>,
    pub find_file: Option<FindFileState>,
    pub search_state: SearchState,
    pub search_buffer: String,
    pub search_direction: SearchDirection,
    pub visual_anchor: Option<(usize, usize)>,
    pub substitute_highlight: Option<(usize, usize, usize)>,
    pub recording_macro: Option<char>,
    pub config_error: Option<String>,
    pub recent_picker: Option<RecentPickerState>,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            buffers: Vec::new(),
            pane_layout: PaneLayout::new(),
            mode: Mode::Normal,
            command_buffer: String::new(),
            status_message: String::new(),
            pending_keys: String::new(),
            pending_hints: Vec::new(),
            highlight_cache: HighlightCache::new(),
            palette: None,
            find_file: None,
            search_state: SearchState {
                last_pattern: None,
                last_direction: SearchDirection::Forward,
                buffer_matches: HashMap::new(),
            },
            search_buffer: String::new(),
            search_direction: SearchDirection::Forward,
            visual_anchor: None,
            substitute_highlight: None,
            recording_macro: None,
            config_error: None,
            recent_picker: None,
        }
    }
}

impl RenderState {
    /// Add a buffer and assign it to the active pane.
    pub fn with_buffer(mut self, buf: Buffer) -> Self {
        let buf_id = buf.id;
        self.buffers.push(buf);
        if let Some(pane) = self.pane_layout.pane_by_id_mut(self.pane_layout.active_id) {
            pane.content = PaneContent::Buffer(buf_id);
        }
        self
    }

    /// Add a buffer without assigning it to a pane (for tab bar tests).
    pub fn with_extra_buffer(mut self, buf: Buffer) -> Self {
        self.buffers.push(buf);
        self
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_command(mut self, cmd: &str) -> Self {
        self.command_buffer = cmd.to_string();
        self.mode = Mode::Command;
        self
    }

    pub fn with_status(mut self, msg: &str) -> Self {
        self.status_message = msg.to_string();
        self
    }

    pub fn with_search(mut self, pattern: &str, direction: SearchDirection) -> Self {
        self.search_buffer = pattern.to_string();
        self.search_direction = direction;
        self.mode = Mode::Search;
        self
    }

    pub fn with_search_matches(mut self, buf_id: usize, matches: Vec<(usize, usize, usize)>, current: Option<usize>) -> Self {
        self.search_state.buffer_matches.insert(buf_id, BufferSearchMatches {
            matches,
            current_match: current,
        });
        self
    }

    pub fn with_visual_anchor(mut self, line: usize, col: usize) -> Self {
        self.visual_anchor = Some((line, col));
        self.mode = Mode::Visual;
        self
    }

    pub fn with_recording_macro(mut self, reg: char) -> Self {
        self.recording_macro = Some(reg);
        self
    }

    pub fn with_config_error(mut self, err: &str) -> Self {
        self.config_error = Some(err.to_string());
        self
    }

    pub fn with_pending_keys(mut self, keys: &str, hints: Vec<(String, String)>) -> Self {
        self.pending_keys = keys.to_string();
        self.pending_hints = hints;
        self
    }

    pub fn with_file_browser(mut self, pane_id: usize, fb: FileBrowser) -> Self {
        if let Some(pane) = self.pane_layout.pane_by_id_mut(pane_id) {
            pane.content = PaneContent::FileBrowser(fb);
        }
        self
    }

    pub fn with_palette(mut self, palette: PaletteState) -> Self {
        self.palette = Some(palette);
        self
    }

    pub fn with_find_file(mut self, ff: FindFileState) -> Self {
        self.find_file = Some(ff);
        self
    }

    pub fn with_recent_picker(mut self, rp: RecentPickerState) -> Self {
        self.recent_picker = Some(rp);
        self
    }

    pub fn with_substitute_highlight(mut self, line: usize, start: usize, end: usize) -> Self {
        self.substitute_highlight = Some((line, start, end));
        self
    }

    /// Add a terminal session to a pane.
    pub fn with_terminal(mut self, pane_id: usize, term: TerminalSession) -> Self {
        if let Some(pane) = self.pane_layout.pane_by_id_mut(pane_id) {
            pane.content = PaneContent::Terminal(term);
        }
        self
    }
}

/// Render the state into a TestBackend and return the frame as a string.
/// Each line is trimmed of trailing whitespace. Lines are joined with newline.
pub fn render_to_string(width: u16, height: u16, state: &RenderState) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let renderer = Renderer::new();

    terminal
        .draw(|frame| {
            renderer.render(
                frame,
                &state.buffers,
                &state.pane_layout,
                &state.mode,
                &state.command_buffer,
                &state.status_message,
                &state.pending_keys,
                &state.pending_hints,
                &state.highlight_cache,
                &state.palette,
                &state.find_file,
                &state.search_state,
                &state.search_buffer,
                state.search_direction,
                state.visual_anchor,
                state.substitute_highlight,
                state.recording_macro,
                state.config_error.as_deref(),
                &state.recent_picker,
            );
        })
        .unwrap();

    let buf = terminal.backend().buffer();
    let mut lines = Vec::new();
    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buf[(x, y)];
            line.push_str(cell.symbol());
        }
        lines.push(line.trim_end().to_string());
    }
    // Remove trailing empty lines.
    while lines.last().map_or(false, |l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

/// Render two states sequentially on the same terminal backend (simulating a content
/// transition like terminal→buffer). Returns the string from the SECOND render.
/// This catches bugs where ratatui's double-buffer diffing leaves stale cells.
pub fn render_transition_to_string(width: u16, height: u16, before: &RenderState, after: &RenderState) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let renderer = Renderer::new();

    // First render (e.g., terminal content).
    terminal
        .draw(|frame| {
            renderer.render(
                frame,
                &before.buffers,
                &before.pane_layout,
                &before.mode,
                &before.command_buffer,
                &before.status_message,
                &before.pending_keys,
                &before.pending_hints,
                &before.highlight_cache,
                &before.palette,
                &before.find_file,
                &before.search_state,
                &before.search_buffer,
                before.search_direction,
                before.visual_anchor,
                before.substitute_highlight,
                before.recording_macro,
                before.config_error.as_deref(),
                &before.recent_picker,
            );
        })
        .unwrap();

    // Second render (e.g., buffer content).
    terminal
        .draw(|frame| {
            renderer.render(
                frame,
                &after.buffers,
                &after.pane_layout,
                &after.mode,
                &after.command_buffer,
                &after.status_message,
                &after.pending_keys,
                &after.pending_hints,
                &after.highlight_cache,
                &after.palette,
                &after.find_file,
                &after.search_state,
                &after.search_buffer,
                after.search_direction,
                after.visual_anchor,
                after.substitute_highlight,
                after.recording_macro,
                after.config_error.as_deref(),
                &after.recent_picker,
            );
        })
        .unwrap();

    let buf = terminal.backend().buffer();
    let mut lines = Vec::new();
    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            let cell = &buf[(x, y)];
            line.push_str(cell.symbol());
        }
        lines.push(line.trim_end().to_string());
    }
    while lines.last().map_or(false, |l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

pub fn check(actual: &str, expect: Expect) {
    expect.assert_eq(actual);
}

/// Create a buffer with text, name, and cursor at (0, 0).
pub fn make_buffer(text: &str, name: &str) -> Buffer {
    let mut b = Buffer::new_scratch();
    b.rope = Rope::from_str(text);
    b.name = name.to_string();
    b.path = Some(PathBuf::from(name));
    b
}

/// Create a buffer with text, name, and cursor at given position.
pub fn make_buffer_at(text: &str, name: &str, line: usize, col: usize) -> Buffer {
    let mut b = make_buffer(text, name);
    b.cursor.line = line;
    b.cursor.col = col;
    b
}

/// Create a DirEntry for testing file browser and find-file.
pub fn make_dir_entry(name: &str, is_dir: bool, size: u64) -> DirEntry {
    DirEntry {
        name: name.to_string(),
        path: PathBuf::from(name),
        is_dir,
        size,
    }
}

/// Create a PaletteState with given items.
pub fn make_palette(items: Vec<(&str, &str, astrum::action::Action)>) -> PaletteState {
    let palette_items: Vec<PaletteItem> = items
        .into_iter()
        .map(|(name, binding, action)| PaletteItem {
            name: name.to_string(),
            binding: binding.to_string(),
            action,
        })
        .collect();
    let filtered = (0..palette_items.len()).collect();
    PaletteState {
        query: String::new(),
        items: palette_items,
        filtered,
        selected: 0,
    }
}

/// Create a RecentPickerState with given items.
pub fn make_recent_picker(items: Vec<(&str, bool)>) -> RecentPickerState {
    let recent_items: Vec<RecentItem> = items
        .into_iter()
        .map(|(display, is_dir)| {
            let kind = if is_dir {
                RecentItemKind::Directory(PathBuf::from(display))
            } else {
                RecentItemKind::File(PathBuf::from(display))
            };
            RecentItem {
                display: display.to_string(),
                kind,
            }
        })
        .collect();
    let filtered = (0..recent_items.len()).collect();
    RecentPickerState {
        query: String::new(),
        items: recent_items,
        filtered,
        selected: 0,
    }
}

/// Create a FindFileState with given entries (without filesystem access).
pub fn make_find_file(dir: &str, entries: Vec<(&str, bool, u64)>) -> FindFileState {
    let dir_entries: Vec<DirEntry> = entries
        .into_iter()
        .map(|(name, is_dir, size)| make_dir_entry(name, is_dir, size))
        .collect();
    let filtered = (0..dir_entries.len()).collect();
    FindFileState {
        dir: PathBuf::from(dir),
        input: String::new(),
        entries: dir_entries,
        filtered,
        selected: 0,
        path_selected: false,
        dir_cursor_cache: HashMap::new(),
    }
}

/// Create a mock TerminalSession with pre-populated grid content.
pub fn make_terminal(rows: u16, cols: u16, lines: &[&str], title: &str) -> TerminalSession {
    TerminalSession::new_mock(rows, cols, lines, title)
}
