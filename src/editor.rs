use crate::action::{Action, SearchDirection};
use crate::buffer::TextBuffer;
use crate::config::Config;
use crate::file_browser::{scan_directory, FileBrowser, FileBrowserResult};
use crate::input::{InputHandler, Mode};
use crate::pane::{FocusDirection, PaneContent, PaneLayout, SplitDirection};
use crate::renderer::Renderer;
use crate::swap::SwapManager;
use crate::syntax::{EditInfo, HighlightCache, HighlightEngine};
use crate::workspace;
use anyhow::Result;
use crossterm::event::{Event, KeyEventKind, MouseEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::collections::HashMap;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};

enum VisualOp { Delete, Change, Yank }

/// What we're waiting for after `awaiting_char` is set.
/// Esc cancels all variants generically by setting this to `None`.
enum AwaitingChar {
    /// Visual surround: waiting for the wrap character.
    Surround,
    /// Macro record: waiting for register char (a-z).
    MacroRecord,
    /// Macro play: waiting for register char (a-z or @).
    MacroPlay { count: usize },
}

/// State for the command palette (SPC SPC).
pub struct PaletteState {
    pub query: String,
    pub items: Vec<PaletteItem>,
    pub filtered: Vec<usize>,
    pub selected: usize,
}

#[derive(Clone)]
pub struct PaletteItem {
    pub action: Action,
    pub name: String,
    pub binding: String,
}

impl PaletteState {
    fn new(items: Vec<PaletteItem>) -> Self {
        let filtered: Vec<usize> = (0..items.len()).collect();
        Self {
            query: String::new(),
            items,
            filtered,
            selected: 0,
        }
    }

    fn refilter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.items.len()).collect();
        } else {
            let lower = self.query.to_lowercase();
            self.filtered = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    item.name.to_lowercase().contains(&lower)
                        || item.binding.to_lowercase().contains(&lower)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    fn selected_action(&self) -> Option<Action> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.items.get(idx))
            .map(|item| item.action.clone())
    }
}

/// State for the recent files picker (SPC b b).
pub struct RecentPickerState {
    pub query: String,
    /// All recent paths (most recent first), with display names.
    pub items: Vec<RecentItem>,
    pub filtered: Vec<usize>,
    pub selected: usize,
}

pub struct RecentItem {
    pub display: String,
    pub kind: RecentItemKind,
}

#[derive(Clone)]
pub enum RecentItemKind {
    File(PathBuf),
    Directory(PathBuf),
    Terminal { term_id: usize },
}

/// An entry in the recent panes list.
#[derive(Clone)]
enum RecentPane {
    Path(PathBuf),
    Terminal { term_id: usize, title: String },
}

impl RecentPickerState {
    fn new(items: Vec<RecentItem>) -> Self {
        let filtered: Vec<usize> = (0..items.len()).collect();
        Self {
            query: String::new(),
            items,
            filtered,
            selected: 0,
        }
    }

    fn refilter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.items.len()).collect();
        } else {
            let lower = self.query.to_lowercase();
            self.filtered = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.display.to_lowercase().contains(&lower))
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    fn selected_item(&self) -> Option<&RecentItem> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.items.get(idx))
    }
}

/// A popup overlay that floats above pane content and intercepts all input.
/// Only one popup can be active at a time.
pub enum Popup {
    FindFile(FindFileState),
    Palette(PaletteState),
    RecentPicker(RecentPickerState),
    CloneRepo(crate::workspace::CloneRepoState),
}

/// Spacemacs-style find-file minibuffer.
pub struct FindFileState {
    /// The directory currently being browsed.
    pub dir: PathBuf,
    /// User input (the part after the directory path).
    pub input: String,
    /// Entries in the current directory.
    pub entries: Vec<crate::file_browser::DirEntry>,
    /// Indices of entries matching the input.
    pub filtered: Vec<usize>,
    /// Selected index within filtered results.
    pub selected: usize,
    /// Whether the path/input line itself is selected (for file creation).
    /// When true, pressing Enter creates a file with the current input name.
    pub path_selected: bool,
    /// Remembered cursor positions for visited directories.
    pub dir_cursor_cache: HashMap<PathBuf, usize>,
}

impl FindFileState {
    fn new(dir: PathBuf) -> Self {
        let mut entries = vec![crate::file_browser::DirEntry {
            name: ".".to_string(),
            path: dir.clone(),
            is_dir: true,
            size: 0,
        }];
        entries.extend(crate::file_browser::scan_directory(&dir));
        let filtered: Vec<usize> = (0..entries.len()).collect();
        Self { dir, input: String::new(), entries, filtered, selected: 0, path_selected: false, dir_cursor_cache: HashMap::new() }
    }

    fn refilter(&mut self) {
        if self.input.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
        } else {
            let lower = self.input.to_lowercase();
            self.filtered = self.entries.iter().enumerate()
                .filter(|(_, e)| e.name.to_lowercase().contains(&lower))
                .map(|(i, _)| i)
                .collect();
        }
        if self.filtered.is_empty() {
            // No matches — select the path line for file creation.
            self.path_selected = true;
        } else {
            self.path_selected = false;
            if self.selected >= self.filtered.len() {
                self.selected = self.filtered.len().saturating_sub(1);
            }
        }
    }

    fn selected_entry(&self) -> Option<&crate::file_browser::DirEntry> {
        self.filtered.get(self.selected).map(|&i| &self.entries[i])
    }

    /// True if the selected entry is the "." (current directory) entry.
    fn is_dot_selected(&self) -> bool {
        self.selected_entry().map_or(false, |e| e.name == ".")
    }

    /// Navigate into a subdirectory, rescan, and clear input.
    /// Remembers cursor position for the current directory and restores it if revisited.
    fn enter_dir(&mut self, path: &std::path::Path) {
        // Save cursor for the directory we're leaving.
        self.dir_cursor_cache.insert(self.dir.clone(), self.selected);
        self.dir = path.to_path_buf();
        let mut entries = vec![crate::file_browser::DirEntry {
            name: ".".to_string(),
            path: self.dir.clone(),
            is_dir: true,
            size: 0,
        }];
        entries.extend(crate::file_browser::scan_directory(&self.dir));
        self.input.clear();
        self.entries = entries;
        self.filtered = (0..self.entries.len()).collect();
        // Restore cursor if we've been here before, clamped to entry count.
        let cached = self.dir_cursor_cache.get(&self.dir).copied().unwrap_or(0);
        self.selected = cached.min(self.filtered.len().saturating_sub(1));
        self.path_selected = false;
    }

    /// Display path shown in the input line.
    pub fn display_path(&self) -> String {
        let mut s = self.dir.to_string_lossy().to_string();
        if !s.ends_with('/') && !s.ends_with('\\') {
            s.push(std::path::MAIN_SEPARATOR);
        }
        s.push_str(&self.input);
        s
    }
}

// Jump history uses PaneContentSnapshot from pane.rs — every content type
// is automatically supported without per-type special casing here.
use crate::pane::PaneContentSnapshot;

/// Per-buffer search matches.
pub struct BufferSearchMatches {
    /// Positions of all matches: (line, col_start, col_end).
    pub matches: Vec<(usize, usize, usize)>,
    /// Index into `matches` of the current/closest match.
    pub current_match: Option<usize>,
}

/// Global search state plus per-buffer match caches.
pub struct SearchState {
    /// The last successfully executed search pattern.
    pub last_pattern: Option<String>,
    /// The direction of the last search.
    pub last_direction: SearchDirection,
    /// Per-buffer match results: buffer_id → matches.
    pub buffer_matches: HashMap<usize, BufferSearchMatches>,
}

impl SearchState {
    fn new() -> Self {
        Self {
            last_pattern: None,
            last_direction: SearchDirection::Forward,
            buffer_matches: HashMap::new(),
        }
    }
}

pub struct Editor {
    buffers: Vec<TextBuffer>,
    pane_layout: PaneLayout,
    input: InputHandler,
    renderer: Renderer,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    highlight_engine: HighlightEngine,
    highlight_cache: HighlightCache,
    status_message: String,
    should_quit: bool,
    clipboard: Option<arboard::Clipboard>,
    /// Per-pane jump history: pane_id → (back_stack, forward_stack).
    jump_history: HashMap<usize, (Vec<PaneContentSnapshot>, Vec<PaneContentSnapshot>)>,
    popup: Option<Popup>,
    swap_manager: SwapManager,
    search: SearchState,
    /// Pending operator awaiting a motion: (operator_action, count).
    pending_operator: Option<(Action, usize)>,
    /// Visual mode anchor: (line, col) where selection started.
    visual_anchor: Option<(usize, usize)>,
    /// Awaiting a single char input (surround char, macro register, etc.).
    /// Esc cancels generically by setting this to `None`.
    awaiting_char: Option<AwaitingChar>,
    /// Set after the first `:q` on the last pane. Cleared by any substantive action.
    quit_pending: bool,
    /// Interactive substitute confirmation state.
    substitute_confirm: Option<SubstituteConfirm>,
    /// Macro registers: register char → recorded actions.
    macro_registers: HashMap<char, Vec<Action>>,
    /// Currently recording macro: (register char, actions so far).
    recording_macro: Option<(char, Vec<Action>)>,
    /// Last played macro register (for `@@`).
    last_macro_register: Option<char>,
    /// File extension → comment syntax (from config).
    comment_syntax: HashMap<String, crate::config::CommentSyntax>,
    /// Config loading error to display on the welcome screen.
    config_error: Option<String>,
    /// Recently visited panes (most recent first) — files, directories, and terminals.
    recent_panes: Vec<RecentPane>,
    /// Awaiting second key of `!c` sequence in git status view.
    git_status_bang_pending: bool,
    /// Detached terminal sessions (not assigned to any pane). Key is terminal session id.
    detached_terminals: HashMap<usize, crate::terminal::TerminalSession>,
    /// Channel for receiving terminal output from background reader threads.
    terminal_rx: tokio::sync::mpsc::UnboundedReceiver<(usize, Vec<u8>)>,
    /// Sender side (cloned per terminal reader thread).
    terminal_tx: tokio::sync::mpsc::UnboundedSender<(usize, Vec<u8>)>,
}

/// Parsed `:s` or `:%s` substitute command.
pub struct Substitute {
    pub pattern: String,
    pub replacement: String,
    pub global: bool,      // /g flag: all occurrences per line
    pub whole_file: bool,  // %s: all lines (vs current line only)
    pub confirm: bool,     // /c flag: ask before each replacement
}

/// State for interactive substitute confirmation (`:s/pat/rep/gc`).
/// Holds the pending matches and steps through them one at a time.
struct SubstituteConfirm {
    replacement: String,
    pattern_char_len: usize,
    /// Remaining matches as (line_idx, col_start) — forward order.
    remaining: Vec<(usize, usize)>,
    current: usize,
    replaced: usize,
    undo_saved: bool,
}

/// Parse a substitute command string like `s/foo/bar/g` or `%s/foo/bar/`.
/// The delimiter is the character after `s` (usually `/`).
pub fn parse_substitute(cmd: &str) -> Option<Substitute> {
    let (whole_file, rest) = if let Some(r) = cmd.strip_prefix("%s") {
        (true, r)
    } else if let Some(r) = cmd.strip_prefix('s') {
        (false, r)
    } else {
        return None;
    };

    // The first character of `rest` is the delimiter.
    let mut chars = rest.chars();
    let delim = chars.next()?;
    if delim.is_alphanumeric() || delim == ' ' {
        return None; // Not a substitute — could be `:set` or similar.
    }
    let rest: String = chars.collect();

    // Split by delimiter into pattern, replacement, flags.
    let parts: Vec<&str> = rest.splitn(3, delim).collect();
    if parts.len() < 2 {
        return None;
    }
    let pattern = parts[0].to_string();
    let replacement = parts[1].to_string();
    let flags = if parts.len() > 2 { parts[2] } else { "" };
    let global = flags.contains('g');
    let confirm = flags.contains('c');

    if pattern.is_empty() {
        return None;
    }

    Some(Substitute { pattern, replacement, global, whole_file, confirm })
}

/// Apply a single motion to a buffer. This is the single source of truth for
/// mapping motion actions to buffer methods — used by both normal movement
/// (with repeat/early-break) and operator+motion composition.
/// Find all (possibly overlapping) substring matches in a rope.
/// Returns Vec<(line_idx, col_start, col_end)> where cols are char indices.
pub fn find_all_matches_in_rope(rope: &ropey::Rope, pattern: &str) -> Vec<(usize, usize, usize)> {
    let mut matches = Vec::new();
    for line_idx in 0..rope.len_lines() {
        let line = rope.line(line_idx);
        let line_str: String = line.chars().collect();
        let mut search_start = 0;
        while search_start < line_str.len() {
            if let Some(byte_pos) = line_str[search_start..].find(pattern) {
                let abs_byte = search_start + byte_pos;
                let col_start = line_str[..abs_byte].chars().count();
                let col_end = col_start + pattern.chars().count();
                matches.push((line_idx, col_start, col_end));
                // Advance by one character to find overlapping matches.
                let next_char_len = line_str[abs_byte..].chars().next().map_or(1, |c| c.len_utf8());
                search_start = abs_byte + next_char_len;
            } else {
                break;
            }
        }
    }
    matches
}

/// Strip Windows `\\?\` UNC prefix from canonical paths.
fn strip_unc_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// Compute the shortest unique suffix for each path in a list.
///
/// Starting from just the filename, extends with parent components until
/// no two paths share the same display string. Paths that are already unique
/// at a shorter suffix keep their short form.
fn shortest_unique_suffixes(paths: &[PathBuf]) -> Vec<String> {
    let components: Vec<Vec<String>> = paths
        .iter()
        .map(|p| {
            p.components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
        .collect();

    // Start each path at depth 1 (just the filename).
    let mut depths: Vec<usize> = vec![1; paths.len()];

    loop {
        // Build display strings at current depths.
        let displays: Vec<String> = components
            .iter()
            .zip(depths.iter())
            .map(|(comps, &depth)| {
                let start = comps.len().saturating_sub(depth);
                comps[start..].join(&std::path::MAIN_SEPARATOR.to_string())
            })
            .collect();

        // Find collisions: group indices by display string.
        let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, d) in displays.iter().enumerate() {
            groups.entry(d.as_str()).or_default().push(i);
        }

        // Extend depth for any group with duplicates.
        let mut changed = false;
        for indices in groups.values() {
            if indices.len() > 1 {
                for &i in indices {
                    if depths[i] < components[i].len() {
                        depths[i] += 1;
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            return displays;
        }
    }
}

fn apply_motion(b: &mut TextBuffer, action: &Action, viewport_height: usize) {
    match action {
        Action::MoveUp => b.move_up(),
        Action::MoveDown => b.move_down(),
        Action::MoveLeft => b.move_left(),
        Action::MoveRight => b.move_right(),
        Action::MoveWordForward => b.move_word_forward(),
        Action::MoveWordEnd => b.move_word_end(),
        Action::MoveWordBackward => b.move_word_backward(),
        Action::MoveBigWordForward => b.move_big_word_forward(),
        Action::MoveBigWordEnd => b.move_big_word_end(),
        Action::MoveBigWordBackward => b.move_big_word_backward(),
        Action::MoveToLineStart => b.move_to_line_start(),
        Action::MoveToLineEnd => b.move_to_line_end(),
        Action::MoveToFirstLine => b.move_to_first_line(),
        Action::MoveToLastLine => b.move_to_last_line(),
        Action::PageUp => b.page_up(viewport_height),
        Action::PageDown => b.page_down(viewport_height),
        Action::HalfPageUp => b.half_page_up(viewport_height),
        Action::HalfPageDown => b.half_page_down(viewport_height),
        Action::ScrollUp => b.scroll_up(viewport_height),
        Action::ScrollDown => b.scroll_down(viewport_height),
        Action::GotoLine(line) => b.goto_line(*line),
        _ => {}
    }
}

impl Editor {
    pub fn new(config: Config) -> Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            cursor::SetCursorStyle::BlinkingBlock
        )?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let input = InputHandler::new(config.keymap);
        let comment_syntax = config.comment_syntax;
        let highlight_engine = HighlightEngine::new();
        let swap_manager = SwapManager::new();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        Ok(Self {
            buffers: Vec::new(),
            pane_layout: PaneLayout::new(),
            input,
            renderer: Renderer::new(),
            terminal,
            highlight_engine,
            highlight_cache: HighlightCache::new(),
            status_message: String::new(),
            should_quit: false,
            clipboard: arboard::Clipboard::new().ok(),
            jump_history: HashMap::new(),
            popup: None,
            swap_manager,
            search: SearchState::new(),
            pending_operator: None,
            visual_anchor: None,
            awaiting_char: None,
            quit_pending: false,
            substitute_confirm: None,
            macro_registers: HashMap::new(),
            recording_macro: None,
            last_macro_register: None,
            comment_syntax,
            config_error: None,
            recent_panes: Vec::new(),
            git_status_bang_pending: false,
            detached_terminals: HashMap::new(),
            terminal_rx: rx,
            terminal_tx: tx,
        })
    }

    pub fn set_config_error(&mut self, err: String) {
        self.config_error = Some(err);
    }

    /// Record a path as recently visited (most recent first, deduplicated).
    fn push_recent(&mut self, path: PathBuf) {
        let canonical = std::fs::canonicalize(&path).unwrap_or(path);
        let clean = strip_unc_prefix(&canonical);
        self.recent_panes.retain(|r| !matches!(r, RecentPane::Path(p) if p == &clean));
        self.recent_panes.insert(0, RecentPane::Path(clean));
        self.recent_panes.truncate(100);
    }

    /// Record a terminal as recently visited.
    fn push_recent_terminal(&mut self, term_id: usize, title: String) {
        self.recent_panes.retain(|r| !matches!(r, RecentPane::Terminal { term_id: id, .. } if *id == term_id));
        self.recent_panes.insert(0, RecentPane::Terminal { term_id, title });
        self.recent_panes.truncate(100);
    }

    /// Build the list of recent items for the picker.
    fn build_recent_items(&self) -> Vec<RecentItem> {
        // Collect paths for shortest_unique_suffixes.
        let paths: Vec<PathBuf> = self.recent_panes.iter().filter_map(|r| {
            if let RecentPane::Path(p) = r { Some(p.clone()) } else { None }
        }).collect();
        let suffixes = shortest_unique_suffixes(&paths);

        let mut suffix_iter = suffixes.into_iter();
        self.recent_panes.iter().map(|r| {
            match r {
                RecentPane::Path(p) => {
                    let display = suffix_iter.next().unwrap_or_default();
                    let kind = if p.is_dir() {
                        RecentItemKind::Directory(p.clone())
                    } else {
                        RecentItemKind::File(p.clone())
                    };
                    RecentItem { display, kind }
                }
                RecentPane::Terminal { term_id, title: saved_title } => {
                    // Look up the live title from the actual terminal session.
                    let live_term = self.pane_layout.panes.iter()
                        .filter_map(|p| p.content.as_terminal())
                        .find(|t| t.id == *term_id)
                        .or_else(|| self.detached_terminals.get(term_id));
                    let (title, exited) = match live_term {
                        Some(t) => {
                            let t_title = if t.title.is_empty() { saved_title.as_str() } else { &t.title };
                            (t_title.to_string(), t.exited.is_some())
                        }
                        None => (saved_title.clone(), true),
                    };
                    let status = if exited { " (exited)" } else { "" };
                    RecentItem {
                        display: format!("[terminal] {}{}", title, status),
                        kind: RecentItemKind::Terminal { term_id: *term_id },
                    }
                }
            }
        }).collect()
    }

    // -- Helper methods --

    fn active_buffer(&self) -> Option<&TextBuffer> {
        let bid = self.pane_layout.active_pane().content.buffer_id()?;
        self.buffers.iter().find(|b| b.id == bid)
    }

    /// Get the active pane's viewport height (set by the renderer each frame).
    fn active_viewport_height(&self) -> usize {
        let h = self.pane_layout.active_pane().height.get();
        if h > 0 { h as usize } else {
            // Fallback before first render.
            self.terminal.size().map(|s| s.height.saturating_sub(4) as usize).unwrap_or(24)
        }
    }

    fn active_buffer_idx(&self) -> Option<usize> {
        let bid = self.pane_layout.active_pane().content.buffer_id()?;
        self.buffers.iter().position(|b| b.id == bid)
    }

    /// Open a file asynchronously. If a buffer for this path already exists,
    /// switch to it instead of creating a duplicate.
    pub async fn open_file(&mut self, path: &str) -> Result<()> {
        let path_buf = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
        self.push_recent(path_buf.clone());

        // Reuse existing buffer for this path.
        if let Some(existing) = self.buffers.iter().find(|b| {
            b.path.as_ref().map_or(false, |p| {
                std::fs::canonicalize(p).unwrap_or_else(|_| p.clone()) == path_buf
            })
        }) {
            let buf_id = existing.id;
            // Detach terminal right before switching — no async gap.
            self.detach_terminal_from_active_pane();
            self.pane_layout.active_pane_mut().switch_buffer(buf_id);
            return Ok(());
        }

        let path_clone = path_buf.clone();
        match tokio::task::spawn_blocking(move || std::fs::read_to_string(&path_clone))
            .await?
        {
            Ok(text) => {
                let mut buf = TextBuffer::from_text(&text, path_buf);
                let buf_id = buf.id;
                let hash = buf.content_hash();
                if let Some(ref p) = buf.path {
                    self.swap_manager.register(buf_id, p, hash);
                }
                self.buffers.push(buf);
                self.request_highlight(buf_id);
                // Detach terminal right before switching — no async gap.
                self.detach_terminal_from_active_pane();
                self.pane_layout.active_pane_mut().switch_buffer(buf_id);
            }
            Err(_) => {
                let mut buf = TextBuffer::new_for_path(path_buf);
                let buf_id = buf.id;
                let hash = buf.content_hash();
                if let Some(ref p) = buf.path {
                    self.swap_manager.register(buf_id, p, hash);
                }
                self.buffers.push(buf);
                // Detach terminal right before switching — no async gap.
                self.detach_terminal_from_active_pane();
                self.pane_layout.active_pane_mut().switch_buffer(buf_id);
                self.status_message = format!("New file: {}", path);
            }
        }
        Ok(())
    }

    pub fn new_scratch_buffer(&mut self) {
        let buf = TextBuffer::new_scratch();
        let buf_id = buf.id;
        self.buffers.push(buf);
        self.pane_layout.active_pane_mut().switch_buffer(buf_id);
    }

    /// Open a terminal in the active pane.
    fn open_terminal(&mut self, shell: Option<&str>) -> Result<()> {
        // Close any existing terminal on this pane.
        if let Some(mut old) = self.pane_layout.active_pane_mut().content.take_terminal() {
            old.kill();
        }
        // Use a default size; resize will correct it on the next render frame.
        let size = self.terminal.size()?;
        let rows = size.height.saturating_sub(3).max(1);
        let cols = size.width.max(1);
        match crate::terminal::TerminalSession::new(rows, cols, shell) {
            Ok(session) => {
                let term_id = session.id;
                // Take the reader and spawn a background thread to forward output.
                if let Some(mut reader) = session.take_reader() {
                    let tx = self.terminal_tx.clone();
                    std::thread::spawn(move || {
                        let mut buf = [0u8; 4096];
                        loop {
                            match reader.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    if tx.send((term_id, buf[..n].to_vec())).is_err() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }
                let tid = session.id;
                let title = session.title.clone();
                self.pane_layout.active_pane_mut().content = PaneContent::Terminal(session);
                self.push_recent_terminal(tid, title);
                self.input.mode = Mode::Insert;
                self.status_message = "Terminal opened".into();
            }
            Err(e) => {
                self.status_message = format!("Failed to open terminal: {}", e);
            }
        }
        Ok(())
    }

    fn request_highlight(&mut self, buf_id: usize) {
        if let Some(buf) = self.buffers.iter().find(|b| b.id == buf_id) {
            let source = buf.text_snapshot();
            let path = buf.path.clone();
            let highlights = self.highlight_engine.parse_full(buf_id, &source, path.as_deref());
            self.highlight_cache.insert(buf_id, highlights);
        }
    }

    async fn open_file_browser(&mut self, dir: PathBuf) -> Result<()> {
        // Detach any terminal on this pane (keep it alive for jump-back).
        self.detach_terminal_from_active_pane();
        let pane_id = self.pane_layout.active_id;
        self.push_recent(dir.clone());
        let mut fb = FileBrowser::new(dir.clone());

        let entries =
            tokio::task::spawn_blocking(move || scan_directory(&dir)).await?;
        fb.set_entries(entries);

        self.pane_layout.pane_by_id_mut(pane_id).unwrap().content = PaneContent::FileBrowser(fb);
        Ok(())
    }

    async fn scan_current_browser_dir(&mut self) -> Result<()> {
        if let Some(fb) = self.pane_layout.active_pane().content.as_browser() {
            let dir = fb.current_dir.clone();
            let entries =
                tokio::task::spawn_blocking(move || scan_directory(&dir)).await?;
            if let Some(fb) = self.pane_layout.active_pane_mut().content.as_browser_mut() {
                fb.set_entries(entries);
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut event_stream = crossterm::event::EventStream::new();
        let mut swap_interval = tokio::time::interval(std::time::Duration::from_secs(2));
        swap_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Check for crash recovery on startup.
        let stale = self.swap_manager.find_stale_swap_files();
        if !stale.is_empty() {
            let count = stale.len();
            for recovered in stale {
                let path = recovered.source_path.clone();
                let mut buf = TextBuffer::from_text(&recovered.content, path);
                buf.modified = true;
                let buf_id = buf.id;
                let hash = buf.content_hash();
                if let Some(ref p) = buf.path {
                    self.swap_manager.register(buf_id, p, hash);
                }
                self.buffers.push(buf);
                self.request_highlight(buf_id);
                self.pane_layout.active_pane_mut().switch_buffer(buf_id);
            }
            self.status_message = format!(
                "Recovered {} file(s) from swap. Review and :w to save.",
                count,
            );
        }

        loop {
            // --- Render ---
            {
                let buffers = &self.buffers;
                let pane_layout = &self.pane_layout;
                let mode = &self.input.mode;
                let cmd_buf = &self.input.command_buffer;
                let status = &self.status_message;
                let pending = &self.input.pending_display;
                let pending_hints = self.input.pending_hints();
                let hl_cache = &self.highlight_cache;
                let popup = &self.popup;
                let search_state = &self.search;
                let search_buf = &self.input.search_buffer;
                let search_dir = self.input.search_direction;
                let visual_anchor = self.visual_anchor;
                let sub_hl = self.substitute_confirm.as_ref().map(|sc| {
                    let (line, col) = sc.remaining[sc.current];
                    (line, col, col + sc.pattern_char_len)
                });
                let rec_macro = self.recording_macro.as_ref().map(|(reg, _)| *reg);
                let config_err = self.config_error.as_deref();
                self.terminal.draw(|frame| {
                    self.renderer.render(
                        frame, buffers, pane_layout, mode, cmd_buf, status, pending,
                        &pending_hints, hl_cache, popup,
                        search_state, search_buf, search_dir, visual_anchor,
                        sub_hl, rec_macro, config_err,
                    );
                })?;
            }

            // Resize terminals to match their actual pane dimensions (now known after render).
            for pane in &mut self.pane_layout.panes {
                if let Some(term) = pane.content.as_terminal_mut() {
                    let h = pane.height.get();
                    let w = pane.width.get();
                    if h > 0 && w > 0 && (term.rows != h || term.cols != w) {
                        term.resize(h, w);
                    }
                }
            }

            // Cursor style per mode.
            if self.pane_layout.active_pane().content.is_browser() {
                execute!(io::stdout(), cursor::SetCursorStyle::SteadyBlock)?;
            } else {
                match self.input.mode {
                    Mode::Insert => {
                        execute!(io::stdout(), cursor::SetCursorStyle::BlinkingBar)?;
                    }
                    _ => {
                        execute!(io::stdout(), cursor::SetCursorStyle::BlinkingBlock)?;
                    }
                }
            }

            // --- Event loop ---
            tokio::select! {
                maybe_event = event_stream.next() => {
                    if let Some(Ok(event)) = maybe_event {
                        match event {
                            Event::Key(key) if key.kind == KeyEventKind::Press => {
                                self.status_message.clear();

                                // Overlays intercept all input when open.
                                if self.substitute_confirm.is_some() {
                                    self.handle_substitute_confirm_key(key);
                                    continue;
                                }
                                if self.popup.is_some() {
                                    match self.popup {
                                        Some(Popup::FindFile(_)) => {
                                            self.handle_find_file_key(key).await?;
                                        }
                                        Some(Popup::RecentPicker(_)) => {
                                            self.handle_recent_picker_key(key).await?;
                                        }
                                        Some(Popup::Palette(_)) => {
                                            self.handle_palette_key(key).await?;
                                        }
                                        Some(Popup::CloneRepo(_)) => {
                                            self.handle_clone_repo_key(key).await?;
                                        }
                                        None => unreachable!(),
                                    }
                                    continue;
                                }

                                let active_content = &self.pane_layout.active_pane().content;
                                let on_terminal_pane = active_content.is_terminal();
                                let on_browser_pane = active_content.is_browser();
                                let on_git_status = matches!(active_content, PaneContent::GitStatus(_));
                                let on_workspace = matches!(active_content, PaneContent::Workspace(_));
                                let in_command_mode = self.input.mode == Mode::Command;
                                let in_search_mode = self.input.mode == Mode::Search;
                                let browser_navigate = on_browser_pane
                                    && !in_command_mode
                                    && active_content.as_browser()
                                        .map_or(false, |fb| fb.input_mode == crate::file_browser::BrowserInputMode::Navigate);

                                if on_terminal_pane && self.input.mode == Mode::Insert
                                    && !in_command_mode && !in_search_mode
                                {
                                    // In Insert mode on a terminal pane: send keys to PTY.
                                    // Esc goes back to Normal mode (handled by input handler).
                                    if key.code == crossterm::event::KeyCode::Esc {
                                        self.input.mode = Mode::Normal;
                                    } else if let Some(bytes) = crate::terminal::key_to_bytes(&key) {
                                        if let Some(term) = self.pane_layout.active_pane_mut().content.as_terminal_mut() {
                                            let _ = term.write_bytes(&bytes);
                                        }
                                    }
                                } else if on_git_status && !in_command_mode && !in_search_mode
                                    && self.input.mode == Mode::Normal
                                {
                                    self.handle_git_status_key(key).await?;
                                } else if on_workspace && !in_command_mode && !in_search_mode
                                    && self.input.mode == Mode::Normal
                                {
                                    self.handle_workspace_key(key).await?;
                                } else if browser_navigate {
                                    self.handle_file_browser_action(key).await?;
                                } else if on_browser_pane && !in_command_mode {
                                    self.quit_pending = false;
                                    // Filter or NewFile text input mode — raw keys
                                    self.handle_file_browser_raw_key(key).await?;
                                } else {
                                    let action = self.input.handle_key(key);
                                    self.execute_action(action).await?;

                                    // Live search: update highlights as the user types.
                                    if self.input.mode == Mode::Search {
                                        if !self.input.search_buffer.is_empty() {
                                            let pattern = self.input.search_buffer.clone();
                                            self.compute_search_matches_for_active(&pattern);
                                        } else if let Some(buf_id) = self.active_buffer().map(|b| b.id) {
                                            self.search.buffer_matches.remove(&buf_id);
                                        }
                                    }
                                }
                            }
                            Event::Mouse(mouse) => {
                                // Suppress mouse events when any overlay is open.
                                let has_overlay = self.substitute_confirm.is_some()
                                    || self.popup.is_some();
                                if !has_overlay {
                                    if let MouseEventKind::Down(crossterm::event::MouseButton::Left) = mouse.kind {
                                        self.handle_mouse_click(mouse.column, mouse.row).await?;
                                    }
                                }
                            }
                            Event::Resize(_w, _h) => {
                                // Terminal resize is handled after the next render frame,
                                // when actual per-pane dimensions are known.
                                // Detached terminals use the full window size as a fallback.
                                let size = self.terminal.size()?;
                                let term_rows = size.height.saturating_sub(3).max(1);
                                let term_cols = size.width.max(1);
                                for (_, term) in &mut self.detached_terminals {
                                    term.resize(term_rows, term_cols);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                // Terminal output from background reader threads.
                Some((term_id, data)) = self.terminal_rx.recv() => {
                    let mut found = false;
                    for pane in &mut self.pane_layout.panes {
                        if let Some(term) = pane.content.as_terminal_mut() {
                            if term.id == term_id {
                                term.process_output(&data);
                                term.check_exit();
                                found = true;
                                break;
                            }
                        }
                    }
                    if !found {
                        // Check detached terminals too.
                        if let Some(term) = self.detached_terminals.get_mut(&term_id) {
                            term.process_output(&data);
                            term.check_exit();
                        }
                    }
                }
                _ = swap_interval.tick() => {
                    self.flush_swap_files().await;
                }
            }

            if self.should_quit {
                break;
            }
        }

        self.cleanup();
        Ok(())
    }

    /// Handle a key in file browser navigate mode: resolve through browser keymap,
    /// then dispatch the resulting action.
    async fn handle_file_browser_action(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<()> {
        let action = self.input.handle_key_for_browser(key);
        if action == Action::Noop {
            return Ok(());
        }

        // Browser has a 2-line header (path + separator), so subtract that.
        let viewport_height = self.active_viewport_height().saturating_sub(2);
        let pane_id = self.pane_layout.active_id;
        let count = self.input.count_prefix.take().unwrap_or(1);

        let old_state = self.pane_layout.active_pane().content.as_browser()
            .map(|fb| (fb.current_dir.clone(), fb.selected, fb.scroll_offset));

        let result = if let Some(fb) = self.pane_layout.active_pane_mut().content.as_browser_mut() {
            let r = fb.handle_action(&action, count);
            fb.ensure_visible(viewport_height);
            r
        } else {
            return Ok(());
        };

        self.handle_browser_result(result, old_state, pane_id, Some(action)).await
    }

    /// Handle raw key events for browser text input modes (Filter, NewFile).
    async fn handle_file_browser_raw_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<()> {
        let viewport_height = self.active_viewport_height().saturating_sub(2);
        let pane_id = self.pane_layout.active_id;

        let old_state = self.pane_layout.active_pane().content.as_browser()
            .map(|fb| (fb.current_dir.clone(), fb.selected, fb.scroll_offset));

        let result = if let Some(fb) = self.pane_layout.active_pane_mut().content.as_browser_mut() {
            let r = fb.handle_key(key);
            fb.ensure_visible(viewport_height);
            r
        } else {
            return Ok(());
        };

        self.handle_browser_result(result, old_state, pane_id, None).await
    }

    /// Shared handler for FileBrowserResult from both navigate and raw key modes.
    async fn handle_browser_result(
        &mut self,
        result: FileBrowserResult,
        old_state: Option<(PathBuf, usize, usize)>,
        pane_id: usize,
        action: Option<Action>,
    ) -> Result<()> {
        // Any browser result other than PassThrough (which goes through
        // execute_action, which has its own quit_pending logic) is a
        // substantive browser action that should cancel a pending quit.
        if !matches!(result, FileBrowserResult::PassThrough | FileBrowserResult::Noop) {
            self.quit_pending = false;
        }
        match result {
            FileBrowserResult::OpenFile(path) => {
                if let Some((dir, sel, scroll)) = old_state {
                    self.push_browser_jump(dir, sel, scroll);
                }
                // open_file will set new content, replacing the browser
                let path_str = path.to_string_lossy().to_string();
                self.open_file(&path_str).await?;
            }
            FileBrowserResult::CreateFile(path) => {
                if let Some((dir, sel, scroll)) = old_state {
                    self.push_browser_jump(dir, sel, scroll);
                }
                let path_str = path.to_string_lossy().to_string();
                self.open_file(&path_str).await?;
                self.input.mode = Mode::Insert;
            }
            FileBrowserResult::Close => {
                if !self.pane_layout.is_single() {
                    self.pane_layout.close_active();
                    self.jump_history.remove(&pane_id);
                } else {
                    let pane_has_buffer = self.pane_layout.active_pane().content.buffer_id().is_some();
                    if pane_has_buffer || !self.buffers.is_empty() {
                        self.pane_layout.active_pane_mut().content = PaneContent::Welcome;
                    }
                }
            }
            FileBrowserResult::NeedsScan => {
                if let Some((dir, sel, scroll)) = old_state {
                    self.push_browser_jump(dir, sel, scroll);
                }
                // Track directory navigation in recent paths.
                if let Some(fb) = self.pane_layout.active_pane().content.as_browser() {
                    self.push_recent(fb.current_dir.clone());
                }
                self.scan_current_browser_dir().await?;
            }
            FileBrowserResult::PassThrough => {
                if let Some(action) = action {
                    self.execute_action(action).await?;
                }
            }
            FileBrowserResult::Noop => {}
        }

        Ok(())
    }

    async fn execute_action(&mut self, action: Action) -> Result<()> {
        // Don't consume count for Noop — digits accumulate across Noop actions.
        if action == Action::Noop {
            return Ok(());
        }
        // Consume count prefix from the input handler.
        let count = self.input.count_prefix.take().unwrap_or(1);

        // Clear quit_pending on any action that isn't part of the :q flow.
        if !matches!(
            action,
            Action::Noop
                | Action::Quit
                | Action::ForceQuit
                | Action::QuitAll
                | Action::ForceQuitAll
                | Action::EnterCommandMode
                | Action::ExecuteCommand(_)
        ) {
            self.quit_pending = false;
        }

        // Awaiting a single char: dispatch based on what we're waiting for.
        // Esc already cleared this to None via EnterNormalMode, so only
        // InsertChar(c) arrives here.
        if let Some(awaiting) = self.awaiting_char.take() {
            self.input.pending_display.clear();
            if let Action::InsertChar(c) = action {
                match awaiting {
                    AwaitingChar::Surround => {
                        self.visual_surround(c);
                    }
                    AwaitingChar::MacroRecord => {
                        if c.is_ascii_alphabetic() {
                            self.recording_macro = Some((c, Vec::new()));
                        } else {
                            self.status_message = "Macro register must be a-z".into();
                        }
                    }
                    AwaitingChar::MacroPlay { count: saved_count } => {
                        let reg = if c == '@' {
                            self.last_macro_register
                        } else {
                            Some(c)
                        };
                        if let Some(reg) = reg {
                            self.play_macro(reg, saved_count).await?;
                        } else {
                            self.status_message = "No previous macro".into();
                        }
                    }
                }
            }
            // Non-InsertChar (shouldn't happen — input.rs returns EnterNormalMode
            // for non-char keys, which clears awaiting_char before we get here).
            return Ok(());
        }

        // Record action into macro buffer (if recording and not a macro control action).
        if self.recording_macro.is_some()
            && !matches!(action, Action::RecordMacro | Action::PlayMacro)
        {
            let recorded = if count > 1 {
                std::iter::repeat(action.clone()).take(count).collect::<Vec<_>>()
            } else {
                vec![action.clone()]
            };
            if let Some((_, ref mut actions)) = self.recording_macro {
                actions.extend(recorded);
            }
        }

        // Operator-pending handling: compose operator + motion.
        if let Some((op, op_count)) = self.pending_operator.take() {
            // Same operator repeated = linewise (dd, cc, yy).
            if action == op {
                let total = op_count * count;
                self.execute_linewise_operator(&op, total);
                return Ok(());
            }
            // Motion = compose operator with motion.
            if action.is_motion() {
                let total = op_count * count;
                self.execute_operator_motion(&op, &action, total);
                return Ok(());
            }
            // Not a motion — cancel operator, fall through to execute action normally.
            self.input.clear_pending();
        }

        // Operator actions: enter pending state.
        if matches!(action, Action::OperatorDelete | Action::OperatorChange | Action::OperatorYank) {
            self.pending_operator = Some((action, count));
            self.input.pending_display = match &self.pending_operator.as_ref().unwrap().0 {
                Action::OperatorDelete => "d".to_string(),
                Action::OperatorChange => "c".to_string(),
                Action::OperatorYank => "y".to_string(),
                _ => unreachable!(),
            };
            return Ok(());
        }

        let viewport_height = self.active_viewport_height();

        // Movement — all motions and scroll commands share apply_motion,
        // repeated with early break when cursor stops moving.
        if action.is_motion() || matches!(action, Action::ScrollUp | Action::ScrollDown) {
            self.repeat_motion(count, &action, viewport_height);
            self.scroll_to_cursor(viewport_height);
            return Ok(());
        }

        match action {
            // Editing — incremental tree-sitter parse
            Action::InsertChar(c) => {
                self.with_buffer_edit(|b| b.insert_char(c));
            }
            Action::InsertNewline => {
                self.with_buffer_edit(|b| b.insert_newline());
            }
            Action::DeleteCharBackward => {
                self.with_buffer_edit(|b| b.delete_char_backward());
            }
            Action::DeleteWordBackward => {
                self.with_buffer_edit(|b| b.delete_word_backward());
            }
            Action::DeleteCharForward => {
                for _ in 0..count {
                    self.with_buffer_edit(|b| b.delete_char_forward());
                }
            }
            Action::DeleteLine => {
                for _ in 0..count {
                    self.with_buffer_edit(|b| b.delete_line());
                }
            }
            Action::DeleteToLineEnd | Action::ChangeToLineEnd => {
                let range = self.sync_buffer(|b| {
                    let start = b.char_idx_at(b.cursor.line, b.cursor.col);
                    let end = b.char_idx_at(b.cursor.line, b.current_line_len());
                    (start, end)
                });
                if let Some((start, end)) = range {
                    self.yank_range(start, end);
                    self.with_buffer_edit(|b| b.delete_char_range(start, end));
                }
                if action == Action::ChangeToLineEnd {
                    self.input.mode = Mode::Insert;
                }
            }

            // Undo/redo — full re-parse (tree state is invalidated)
            Action::Undo => {
                self.with_buffer(|b| b.undo());
                self.rehighlight_active();
            }
            Action::Redo => {
                self.with_buffer(|b| b.redo());
                self.rehighlight_active();
            }

            // Clipboard
            Action::YankLine => {
                let range = self.sync_buffer(|b| {
                    b.linewise_range(b.cursor.line, b.cursor.line)
                });
                if let Some((start, end)) = range {
                    self.yank_range(start, end);
                    self.status_message = "Yanked 1 line".into();
                }
            }
            Action::Paste => {
                let text = self.clipboard.as_mut()
                    .and_then(|cb| cb.get_text().ok());
                if let Some(text) = text {
                    self.with_buffer(|b| b.paste_after(&text));
                    self.rehighlight_active();
                } else {
                    self.status_message = "Clipboard empty".into();
                }
            }
            Action::PasteBefore => {
                let text = self.clipboard.as_mut()
                    .and_then(|cb| cb.get_text().ok());
                if let Some(text) = text {
                    self.with_buffer(|b| b.paste_before(&text));
                    self.rehighlight_active();
                } else {
                    self.status_message = "Clipboard empty".into();
                }
            }

            // Jump history
            Action::JumpBack => {
                self.jump_back();
            }
            Action::JumpForward => {
                self.jump_forward();
            }

            Action::InsertLineBelow => {
                self.with_buffer_edit(|b| b.insert_line_below());
                self.input.mode = Mode::Insert;
            }
            Action::InsertLineAbove => {
                self.with_buffer_edit(|b| b.insert_line_above());
                self.input.mode = Mode::Insert;
            }

            // Mode changes
            Action::EnterInsertMode => {
                self.input.mode = Mode::Insert;
            }
            Action::EnterInsertModeAppend => {
                self.with_buffer(|b| {
                    let line_len = b.current_line_len();
                    b.cursor.col = (b.cursor.col + 1).min(line_len);
                });
                self.input.mode = Mode::Insert;
            }
            Action::EnterInsertModeLineEnd => {
                self.with_buffer(|b| {
                    b.cursor.col = b.current_line_len();
                });
                self.input.mode = Mode::Insert;
            }
            Action::EnterInsertModeLineStart => {
                self.with_buffer(|b| b.move_to_line_start());
                self.input.mode = Mode::Insert;
            }
            Action::EnterNormalMode => {
                self.exit_visual_mode();
                self.awaiting_char = None;
                self.pending_operator = None;
                self.input.clear_pending();
                // Clamp cursor back from insert-mode append position.
                self.with_buffer(|b| {
                    let max_col = b.current_line_len().saturating_sub(1);
                    if b.cursor.col > max_col {
                        b.cursor.col = max_col;
                    }
                });
            }
            Action::EnterVisualMode => {
                if self.input.mode == Mode::Visual {
                    // Toggle off: v again exits visual mode.
                    self.exit_visual_mode();
                } else {
                    let pane = self.pane_layout.active_pane();
                    self.visual_anchor = Some((pane.cursor.line, pane.cursor.col));
                    self.input.mode = Mode::Visual;
                }
            }
            Action::EnterCommandMode => {
                self.input.mode = Mode::Command;
                self.input.command_buffer.clear();
            }

            // Search
            Action::EnterSearchForward => {
                self.input.mode = Mode::Search;
                self.input.search_direction = SearchDirection::Forward;
                self.input.search_buffer.clear();
            }
            Action::EnterSearchBackward => {
                self.input.mode = Mode::Search;
                self.input.search_direction = SearchDirection::Backward;
                self.input.search_buffer.clear();
            }
            Action::SearchExecute(pattern, direction) => {
                self.execute_search(pattern, direction);
            }
            Action::SearchNext => {
                self.search_next_prev(false);
            }
            Action::SearchPrev => {
                self.search_next_prev(true);
            }
            Action::SearchCancel => {
                if let Some(buf_id) = self.active_buffer().map(|b| b.id) {
                    self.search.buffer_matches.remove(&buf_id);
                }
            }

            // Buffer management
            Action::OpenRecentPicker => {
                if !matches!(self.popup, Some(Popup::RecentPicker(_))) {
                    let items = self.build_recent_items();
                    self.popup = Some(Popup::RecentPicker(RecentPickerState::new(items)));
                    execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
                }
            }
            Action::NextBuffer => {
                self.switch_buffer_by_offset(1);
            }
            Action::PrevBuffer => {
                self.switch_buffer_by_offset(-1);
            }
            Action::CloseBuffer => {
                self.close_buffer_and_pane();
            }

            // Window management
            Action::SplitVertical | Action::SplitHorizontal => {
                let dir = if action == Action::SplitVertical {
                    SplitDirection::Vertical
                } else {
                    SplitDirection::Horizontal
                };
                let old_id = self.pane_layout.active_id;
                let new_id = self.pane_layout.split(dir);
                if let Some(history) = self.jump_history.get(&old_id).cloned() {
                    self.jump_history.insert(new_id, history);
                }
            }
            Action::FocusPaneNext => {
                self.pane_layout.focus_next();
            }
            Action::FocusPanePrev => {
                self.pane_layout.focus_prev();
            }
            Action::FocusPaneLeft | Action::FocusPaneRight |
            Action::FocusPaneUp | Action::FocusPaneDown => {
                let dir = match action {
                    Action::FocusPaneLeft => FocusDirection::Left,
                    Action::FocusPaneRight => FocusDirection::Right,
                    Action::FocusPaneUp => FocusDirection::Up,
                    Action::FocusPaneDown => FocusDirection::Down,
                    _ => unreachable!(),
                };
                self.pane_layout.focus_direction(dir);
            }
            Action::MovePaneLeft | Action::MovePaneRight |
            Action::MovePaneUp | Action::MovePaneDown => {
                let dir = match action {
                    Action::MovePaneLeft => FocusDirection::Left,
                    Action::MovePaneRight => FocusDirection::Right,
                    Action::MovePaneUp => FocusDirection::Up,
                    Action::MovePaneDown => FocusDirection::Down,
                    _ => unreachable!(),
                };
                if !self.pane_layout.move_direction(dir) {
                    self.status_message = "Cannot move pane in that direction".into();
                }
            }
            Action::ClosePane => {
                let old_id = self.pane_layout.active_id;
                self.detach_terminal_from_active_pane();
                self.pane_layout.close_active();
                self.jump_history.remove(&old_id);
            }

            // File browser
            Action::OpenFileBrowser => {
                if !matches!(self.popup, Some(Popup::FindFile(_))) {
                    let dir = if let Some(fb) = self.pane_layout.active_pane().content.as_browser() {
                        // Start at the file browser's current directory.
                        fb.current_dir.clone()
                    } else if let Some(buf) = self.active_buffer() {
                        buf.path
                            .as_ref()
                            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
                    } else {
                        std::env::current_dir().unwrap_or_default()
                    };
                    self.popup = Some(Popup::FindFile(FindFileState::new(dir)));
                    execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
                }
            }
            Action::OpenFileBrowserHome => {
                if !matches!(self.popup, Some(Popup::FindFile(_))) {
                    let dir = dirs::home_dir().unwrap_or_else(|| {
                        std::env::current_dir().unwrap_or_default()
                    });
                    self.popup = Some(Popup::FindFile(FindFileState::new(dir)));
                    execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
                }
            }

            Action::OpenTerminal => {
                self.open_terminal(None)?;
            }

            // -- Workspace / Git --
            Action::OpenWorkspace => {
                self.open_workspace()?;
            }
            Action::OpenGitRepo => {
                self.open_git_repo()?;
            }
            Action::OpenGitStatus => {
                self.open_git_status()?;
            }
            Action::OpenCloneRepo => {
                if !matches!(self.popup, Some(Popup::CloneRepo(_))) {
                    let cloned = self.workspace_cloned_set();
                    self.popup = Some(Popup::CloneRepo(workspace::CloneRepoState::new(cloned)));
                    execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
                }
            }
            Action::GitAddAll => {
                self.git_add_all().await?;
            }

            Action::SaveBuffer => {
                self.save_current_buffer().await?;
            }
            Action::SaveBufferAs(path) => {
                self.save_current_buffer_as(&path).await?;
            }

            // Commands
            Action::ExecuteCommand(cmd) => {
                self.execute_command(&cmd).await?;
            }

            // Close current window (pane if splits, buffer if single)
            Action::Quit => {
                self.quit_current(false);
            }
            Action::ForceQuit => {
                self.quit_current(true);
            }

            // Exit the entire application
            Action::QuitAll => {
                if self.buffers.iter().any(|b| b.modified) {
                    self.status_message =
                        "Unsaved changes exist. Use :qa! to force quit".into();
                } else {
                    self.should_quit = true;
                }
            }
            Action::ForceQuitAll => {
                self.should_quit = true;
            }

            Action::CommandPalette => {
                self.open_palette();
            }

            // Browser actions are handled in handle_file_browser_action, not here.
            Action::BrowserOpen
            | Action::BrowserParentDir
            | Action::BrowserFilter
            | Action::BrowserNewFile
            | Action::BrowserClose
            | Action::BrowserHome => {}

            // Operators are handled above (pending state).
            Action::OperatorDelete | Action::OperatorChange | Action::OperatorYank => {}

            // Visual mode operations.
            Action::VisualDelete => {
                self.visual_operate(VisualOp::Delete);
            }
            Action::VisualYank => {
                self.visual_operate(VisualOp::Yank);
            }
            Action::VisualChange => {
                self.visual_operate(VisualOp::Change);
            }
            Action::VisualSurround => {
                self.enter_awaiting_char(AwaitingChar::Surround, "s");
            }

            // Macros
            Action::RecordMacro => {
                if self.recording_macro.is_some() {
                    // Stop recording.
                    let (reg, actions) = self.recording_macro.take().unwrap();
                    self.macro_registers.insert(reg, actions);
                    self.status_message = format!("Recorded @{}", reg);
                } else {
                    // Start recording — await register char.
                    self.enter_awaiting_char(AwaitingChar::MacroRecord, "q");
                }
            }
            Action::PlayMacro => {
                self.enter_awaiting_char(AwaitingChar::MacroPlay { count }, "@");
            }

            Action::CommentToggle => {
                self.toggle_comment();
            }

            Action::Noop => {}

            // Motions and scroll are handled above the match (early return).
            _ => {}
        }

        self.scroll_to_cursor(viewport_height);

        Ok(())
    }

    async fn save_current_buffer(&mut self) -> Result<()> {
        self.save_buffer_impl(false).await
    }

    async fn force_save_current_buffer(&mut self) -> Result<()> {
        self.save_buffer_impl(true).await
    }

    async fn save_buffer_impl(&mut self, force: bool) -> Result<()> {
        let buf_idx = match self.active_buffer_idx() {
            Some(idx) => idx,
            None => return Ok(()),
        };
        if let Some(buf) = self.buffers.get_mut(buf_idx) {
            if let Some(ref path) = buf.path {
                let buf_id = buf.id;

                // Check for external changes unless force-saving.
                if !force {
                    let check_path = path.clone();
                    if let Some(disk_hash) = tokio::task::spawn_blocking(move || {
                        crate::swap::hash_file(&check_path)
                    }).await? {
                        if self.swap_manager.disk_changed(buf_id, disk_hash) {
                            self.status_message =
                                "File changed on disk. Use :w! to force save.".into();
                            return Ok(());
                        }
                    }
                }

                let text = buf.text_snapshot();
                let path = path.clone();
                let name = buf.name.clone();
                match tokio::task::spawn_blocking(move || {
                    // Ensure parent directory exists (needed for newly created files).
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    // Guard: refuse to overwrite a directory with a file.
                    if path.is_dir() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("path is a directory: {}", path.display()),
                        ));
                    }
                    std::fs::write(&path, text)
                })
                    .await?
                {
                    Ok(()) => {
                        buf.modified = false;
                        let new_hash = buf.content_hash();
                        self.swap_manager.update_disk_hash(buf_id, new_hash);
                        self.swap_manager.unregister(buf_id);
                        // Re-register so future edits get tracked again.
                        if let Some(ref p) = self.buffers.iter().find(|b| b.id == buf_id)
                            .and_then(|b| b.path.clone())
                        {
                            self.swap_manager.register(buf_id, p, new_hash);
                        }
                        self.status_message = format!("Saved: {}", name);
                    }
                    Err(e) => {
                        let path_display = buf.path.as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        self.status_message = format!("Error saving {}: {} (path: {})", name, e, path_display);
                    }
                }
            } else {
                self.status_message = "No file path. Use :w <path>".into();
            }
        }
        Ok(())
    }

    async fn save_current_buffer_as(&mut self, path: &str) -> Result<()> {
        let buf_idx = match self.active_buffer_idx() {
            Some(idx) => idx,
            None => return Ok(()),
        };
        if let Some(buf) = self.buffers.get_mut(buf_idx) {
            let path_buf = PathBuf::from(path);
            let text = buf.text_snapshot();
            let write_path = path_buf.clone();
            match tokio::task::spawn_blocking(move || std::fs::write(&write_path, text))
                .await?
            {
                Ok(()) => {
                    buf.name = path_buf
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.to_string());
                    buf.path = Some(path_buf);
                    buf.modified = false;
                    self.status_message = format!("Saved as: {}", path);
                }
                Err(e) => {
                    self.status_message = format!("Error saving: {}", e);
                }
            }
        }
        Ok(())
    }

    async fn execute_command(&mut self, cmd: &str) -> Result<()> {
        let cmd = cmd.trim();

        // Substitute command: :s/pat/rep/[flags] or :%s/pat/rep/[flags]
        if let Some(sub) = parse_substitute(cmd) {
            self.execute_substitute(sub);
            return Ok(());
        }

        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        // Clear quit_pending for any command that isn't a quit variant.
        let is_quit_cmd = matches!(parts.first().copied(), Some("q" | "q!" | "qa" | "qa!" | "wq"));
        if !is_quit_cmd {
            self.quit_pending = false;
        }
        match parts.first().copied() {
            Some("q") => {
                self.quit_current(false);
            }
            Some("q!") => {
                self.quit_current(true);
            }
            Some("qa") => {
                if self.buffers.iter().any(|b| b.modified) {
                    self.status_message =
                        "Unsaved changes exist. Use :qa! to force quit".into();
                } else {
                    self.should_quit = true;
                }
            }
            Some("qa!") => {
                self.should_quit = true;
            }
            Some("w") => {
                if parts.len() > 1 {
                    self.save_current_buffer_as(parts[1]).await?;
                } else {
                    self.save_current_buffer().await?;
                }
            }
            Some("w!") => {
                if parts.len() > 1 {
                    self.save_current_buffer_as(parts[1]).await?;
                } else {
                    self.force_save_current_buffer().await?;
                }
            }
            Some("wq") => {
                self.save_current_buffer().await?;
                self.quit_current(false);
            }
            Some("e") => {
                if parts.len() > 1 {
                    self.push_jump();
                    let path = parts[1];
                    let pb = PathBuf::from(path);
                    if pb.is_dir() {
                        self.open_file_browser(pb).await?;
                    } else {
                        self.open_file(path).await?;
                    }
                } else {
                    // :e with no arg opens file browser in current dir
                    let dir = std::env::current_dir().unwrap_or_default();
                    self.open_file_browser(dir).await?;
                }
            }
            Some("files") | Some("browse") => {
                let dir = std::env::current_dir().unwrap_or_default();
                self.open_file_browser(dir).await?;
            }
            Some("bn") | Some("bnext") => {
                self.switch_buffer_by_offset(1);
            }
            Some("bp") | Some("bprev") => {
                self.switch_buffer_by_offset(-1);
            }
            Some("bd") => {
                self.close_current_buffer(false);
            }
            Some("new") => {
                self.new_scratch_buffer();
            }
            Some(c) => {
                if let Ok(n) = c.parse::<usize>() {
                    if n > 0 {
                        self.with_buffer(|b| b.goto_line(n - 1));
                    }
                } else {
                    self.status_message = format!("Unknown command: {}", c);
                }
            }
            None => {}
        }
        Ok(())
    }

    /// Returns true if the user has confirmed the double-quit (second :q with no
    /// substantive action in between). Otherwise sets the pending state and returns false.
    fn confirm_double_quit(&mut self) -> bool {
        if self.quit_pending {
            true
        } else {
            self.quit_pending = true;
            self.status_message = "Press :q again to quit".into();
            false
        }
    }

    /// `:q` behavior: close the pane if splits exist, otherwise quit the app.
    fn quit_current(&mut self, force: bool) {
        if !self.pane_layout.is_single() {
            let pane_id = self.pane_layout.active_id;
            self.detach_terminal_from_active_pane();
            self.pane_layout.close_active();
            self.jump_history.remove(&pane_id);
        } else if force || self.confirm_double_quit() {
            self.should_quit = true;
        }
    }

    /// `SPC b d` — close the pane and delete content from the system.
    /// Terminals are killed, buffers are removed, recents are cleaned up.
    fn close_buffer_and_pane(&mut self) {
        let pane_id = self.pane_layout.active_id;
        let mut blocked = false;
        let content = &self.pane_layout.active_pane().content;

        // Check if another pane has the same content.
        let other_has_same = self.pane_layout.panes.iter()
            .any(|p| p.id != pane_id && p.content.same_identity(content));

        match content {
            PaneContent::Terminal(_) if !other_has_same => {
                let mut term = self.pane_layout.active_pane_mut().content.take_terminal().unwrap();
                let term_id = term.id;
                term.kill();
                self.recent_panes.retain(|r| !matches!(r, RecentPane::Terminal { term_id: id, .. } if *id == term_id));
            }
            PaneContent::FileBrowser(_) if !other_has_same => {
                if let Some(fb) = self.pane_layout.active_pane().content.as_browser() {
                    let dir = fb.current_dir.clone();
                    let clean = strip_unc_prefix(
                        &std::fs::canonicalize(&dir).unwrap_or_else(|_| dir),
                    );
                    self.recent_panes.retain(|r| !matches!(r, RecentPane::Path(p) if p == &clean));
                }
            }
            PaneContent::Editor(buf_id) => {
                let buf_id = *buf_id;
                if let Some(buf_idx) = self.buffers.iter().position(|b| b.id == buf_id) {
                    if self.buffers[buf_idx].modified {
                        self.status_message = "Buffer has unsaved changes. Use :bd! to force close".into();
                        blocked = true;
                    } else if !other_has_same {
                        if let Some(ref path) = self.buffers[buf_idx].path.clone() {
                            let clean = strip_unc_prefix(
                                &std::fs::canonicalize(path).unwrap_or_else(|_| path.clone()),
                            );
                            self.recent_panes.retain(|r| !matches!(r, RecentPane::Path(p) if p == &clean));
                        }
                        let removed = self.buffers.remove(buf_idx);
                        let removed_id = removed.id;
                        self.highlight_cache.invalidate(removed_id);
                        self.highlight_engine.remove_buffer(removed_id);
                        self.swap_manager.unregister(removed_id);
                        for pane in &mut self.pane_layout.panes {
                            if let PaneContent::Editor(bid) = pane.content {
                                if bid == removed_id {
                                    pane.content = PaneContent::Welcome;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        if !blocked {
            self.pane_layout.close_active();
            self.jump_history.remove(&pane_id);
        }
    }

    /// Toggle line/block comments on the current line or visual selection.
    fn toggle_comment(&mut self) {
        let syntax = self.sync_buffer(|b| {
            b.path.as_ref().and_then(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_string())
            })
        }).flatten().and_then(|ext| {
            self.comment_syntax.get(&ext).cloned()
        });

        let Some(syntax) = syntax else {
            self.status_message = "No comment syntax for this file type".to_string();
            return;
        };

        if self.visual_anchor.is_some() {
            // Visual mode: block comment around selection.
            if let Some((open, close)) = &syntax.block {
                let range = self.visual_char_range();
                if let Some((start, end)) = range {
                    let open = open.clone();
                    let close = close.clone();
                    self.with_buffer_edit(|b| {
                        b.toggle_block_comment(start, end, &open, &close);
                        None
                    });
                    self.rehighlight_active();
                }
            } else if let Some(prefix) = &syntax.line {
                // Fallback to line comments if no block syntax.
                let cursor_line = self.pane_layout.active_pane().cursor.line;
                let anchor_line = self.visual_anchor.unwrap().0;
                let first = anchor_line.min(cursor_line);
                let last = anchor_line.max(cursor_line);
                let prefix = prefix.clone();
                self.with_buffer_edit(|b| {
                    b.toggle_line_comment(first, last, &prefix);
                    None
                });
                self.rehighlight_active();
            } else {
                self.status_message = "No comment syntax for this file type".to_string();
            }
            self.exit_visual_mode();
        } else {
            // Normal mode: line comment on current line.
            if let Some(prefix) = &syntax.line {
                let line = self.pane_layout.active_pane().cursor.line;
                let prefix = prefix.clone();
                self.with_buffer_edit(|b| {
                    b.toggle_line_comment(line, line, &prefix);
                    None
                });
                self.rehighlight_active();
            } else {
                self.status_message = "No line comment syntax for this file type".to_string();
            }
        }
    }

    fn close_current_buffer(&mut self, force: bool) {
        if self.buffers.is_empty() || self.active_buffer_idx().is_none() {
            if self.pane_layout.is_single() && self.confirm_double_quit() {
                self.should_quit = true;
            }
            return;
        }
        let buf_idx = self.active_buffer_idx().unwrap();
        if !force && self.buffers[buf_idx].modified {
            self.status_message = "Buffer has unsaved changes. Use :q! to force close".into();
            return;
        }
        // If this is the last buffer on the last pane, require double-quit
        // instead of removing the buffer (which would show the welcome screen).
        if self.buffers.len() == 1 && self.pane_layout.is_single() {
            if self.confirm_double_quit() {
                self.should_quit = true;
            }
            return;
        }
        let removed = self.buffers.remove(buf_idx);
        let removed_id = removed.id;
        self.highlight_cache.invalidate(removed_id);
        self.highlight_engine.remove_buffer(removed_id);
        self.swap_manager.unregister(removed_id);
        // Remove from recents.
        if let Some(ref path) = removed.path {
            let clean = strip_unc_prefix(
                &std::fs::canonicalize(path).unwrap_or_else(|_| path.clone()),
            );
            self.recent_panes.retain(|r| !matches!(r, RecentPane::Path(p) if p == &clean));
        }

        let active_id = self.pane_layout.active_id;

        // For the active pane, try to restore from jump history
        if self.pane_layout.active_pane().content.buffer_id() == Some(removed_id) {
            let mut restored = false;
            // Search jump_back for a valid destination (skip stale entries)
            let pane_id = self.pane_layout.active_id;
            let stacks = self.jump_history.entry(pane_id).or_insert_with(|| (Vec::new(), Vec::new()));
            while let Some(pos) = stacks.0.pop() {
                // Skip Editor snapshots for the removed buffer; accept everything else.
                if let PaneContentSnapshot::Editor { buffer_id, .. } = &pos {
                    if *buffer_id == removed_id
                        || !self.buffers.iter().any(|b| b.id == *buffer_id)
                    {
                        continue; // stale entry
                    }
                }
                self.restore_jump(pos);
                restored = true;
                break;
            }
            if !restored {
                if !self.buffers.is_empty() {
                    let pane = self.pane_layout.active_pane_mut();
                    let new_buf_id =
                        self.buffers[self.buffers.len().saturating_sub(1)].id;
                    pane.switch_buffer(new_buf_id);
                } else if self.pane_layout.is_single() {
                    // Last buffer on last pane — require a second :q to exit.
                    self.quit_pending = true;
                    self.status_message = "Press :q again to quit".into();
                    return;
                }
            }
        }

        // Update other panes that referenced this buffer
        for pane in &mut self.pane_layout.panes {
            if pane.id != active_id && pane.content.buffer_id() == Some(removed_id) {
                if !self.buffers.is_empty() {
                    let new_buf_id =
                        self.buffers[self.buffers.len().saturating_sub(1)].id;
                    pane.switch_buffer(new_buf_id);
                } else {
                    pane.content = PaneContent::Welcome;
                    pane.cursor = crate::buffer::Cursor::default();
                    pane.scroll_offset = 0;
                }
            }
        }
    }

    fn open_palette(&mut self) {
        use std::collections::HashMap as HMap;

        // Collect keybindings from the normal keymap.
        let bindings = self.input.keymap().normal.all_bindings();
        let mut binding_map: HMap<String, Vec<String>> = HMap::new();
        for (keys, action) in &bindings {
            let name = format!("{:?}", action);
            binding_map.entry(name).or_default().push(keys.clone());
        }

        let items: Vec<PaletteItem> = Action::all_configurable()
            .into_iter()
            .map(|action| {
                let name = action.display_name().to_string();
                let key = format!("{:?}", action);
                let binding = binding_map
                    .get(&key)
                    .map(|v| v.join(", "))
                    .unwrap_or_default();
                PaletteItem { action, name, binding }
            })
            .collect();

        self.popup = Some(Popup::Palette(PaletteState::new(items)));
        // Clear any pending key chain state.
        self.input.clear_pending();
    }

    async fn handle_palette_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                self.popup = None;
            }
            KeyCode::Enter => {
                let action = if let Some(Popup::Palette(ref p)) = self.popup {
                    p.selected_action()
                } else {
                    None
                };
                self.popup = None;
                if let Some(action) = action {
                    self.execute_action(action).await?;
                }
            }
            KeyCode::Up | KeyCode::Char('k') if key.code == KeyCode::Up || ctrl => {
                if let Some(Popup::Palette(ref mut p)) = self.popup {
                    p.selected = p.selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') if key.code == KeyCode::Down || ctrl => {
                if let Some(Popup::Palette(ref mut p)) = self.popup {
                    if p.selected + 1 < p.filtered.len() {
                        p.selected += 1;
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(Popup::Palette(ref mut p)) = self.popup {
                    p.query.pop();
                    p.refilter();
                }
            }
            KeyCode::Char(c) => {
                if let Some(Popup::Palette(ref mut p)) = self.popup {
                    p.query.push(c);
                    p.refilter();
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_recent_picker_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                self.popup = None;
                execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
            }
            KeyCode::Enter => {
                let selected = if let Some(Popup::RecentPicker(ref rp)) = self.popup {
                    rp.selected_item().map(|item| item.kind.clone())
                } else {
                    None
                };

                self.popup = None;
                execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;

                if let Some(kind) = selected {
                    self.push_jump();
                    match kind {
                        RecentItemKind::Directory(path) => {
                            self.open_file_browser(path).await?;
                        }
                        RecentItemKind::File(path) => {
                            self.open_file(path.to_string_lossy().as_ref()).await?;
                        }
                        RecentItemKind::Terminal { term_id } => {
                            self.reattach_terminal(term_id)?;
                        }
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') if key.code == KeyCode::Up || ctrl => {
                if let Some(Popup::RecentPicker(ref mut rp)) = self.popup {
                    rp.selected = rp.selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') if key.code == KeyCode::Down || ctrl => {
                if let Some(Popup::RecentPicker(ref mut rp)) = self.popup {
                    if rp.selected + 1 < rp.filtered.len() {
                        rp.selected += 1;
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(Popup::RecentPicker(ref mut rp)) = self.popup {
                    rp.query.pop();
                    rp.refilter();
                }
            }
            KeyCode::Char(c) => {
                if let Some(Popup::RecentPicker(ref mut rp)) = self.popup {
                    rp.query.push(c);
                    rp.refilter();
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_find_file_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                self.close_find_file()?;
            }
            KeyCode::Enter => {
                // Take the FindFile state out of the popup to avoid borrow issues.
                let mut ff = match self.popup.take() {
                    Some(Popup::FindFile(ff)) => ff,
                    other => { self.popup = other; return Ok(()); }
                };
                if ff.path_selected && !ff.input.is_empty() {
                    // Path line selected — create the file.
                    let new_path = ff.dir.join(&ff.input);
                    if new_path.exists() {
                        self.status_message = format!("\"{}\" already exists", ff.input);
                        self.popup = Some(Popup::FindFile(ff));
                    } else {
                        // Record jump BEFORE opening file so history captures browser state.
                        self.push_jump();
                        let path_str = new_path.to_string_lossy().to_string();
                        self.open_file(&path_str).await?;
                        self.input.mode = Mode::Insert;
                    }
                } else if ff.is_dot_selected() {
                    // "." → open tree browser at current directory.
                    let dir = ff.dir.clone();
                    self.push_jump();
                    self.open_file_browser(dir).await?;
                } else if let Some(entry) = ff.selected_entry().cloned() {
                    if entry.is_dir {
                        ff.enter_dir(&entry.path);
                        self.popup = Some(Popup::FindFile(ff));
                    } else {
                        // Record jump BEFORE opening file so history captures browser state.
                        self.push_jump();
                        let path_str = entry.path.to_string_lossy().to_string();
                        self.open_file(&path_str).await?;
                    }
                }
            }
            KeyCode::Tab => {
                if let Some(Popup::FindFile(ref mut ff)) = self.popup {
                    if let Some(entry) = ff.selected_entry().cloned() {
                        if entry.is_dir {
                            ff.enter_dir(&entry.path);
                        } else {
                            ff.input = entry.name.clone();
                            ff.refilter();
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(Popup::FindFile(ref mut ff)) = self.popup {
                    if ff.input.is_empty() {
                        if let Some(parent) = ff.dir.parent().map(|p| p.to_path_buf()) {
                            ff.enter_dir(&parent);
                        }
                    } else {
                        ff.input.pop();
                        ff.refilter();
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') if key.code == KeyCode::Up || ctrl => {
                if let Some(Popup::FindFile(ref mut ff)) = self.popup {
                    if ff.path_selected {
                        // Already at top (path line) — do nothing.
                    } else if ff.selected == 0 {
                        ff.path_selected = true;
                    } else {
                        ff.selected = ff.selected.saturating_sub(1);
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') if key.code == KeyCode::Down || ctrl => {
                if let Some(Popup::FindFile(ref mut ff)) = self.popup {
                    if ff.path_selected {
                        if !ff.filtered.is_empty() {
                            ff.path_selected = false;
                            ff.selected = 0;
                        }
                    } else if ff.selected + 1 < ff.filtered.len() {
                        ff.selected += 1;
                    }
                }
            }
            KeyCode::Char(c) => {
                if let Some(Popup::FindFile(ref mut ff)) = self.popup {
                    if c == '/' || c == '\\' {
                        if let Some(entry) = ff.selected_entry().cloned() {
                            if entry.is_dir {
                                ff.enter_dir(&entry.path);
                                return Ok(());
                            }
                        }
                    }
                    ff.input.push(c);
                    ff.refilter();
                }
            }
            _ => {}
        }
        // Disable mouse capture when find-file closes.
        if !matches!(self.popup, Some(Popup::FindFile(_))) {
            execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
        }
        Ok(())
    }

    fn close_find_file(&mut self) -> Result<()> {
        self.popup = None;
        execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Workspace / Git
    // ------------------------------------------------------------------

    fn open_workspace(&mut self) -> Result<()> {
        self.push_jump();
        let ws = workspace::Workspace::open()?;
        self.pane_layout.active_pane_mut().content = PaneContent::Workspace(ws);
        self.input.mode = Mode::Normal;
        Ok(())
    }

    fn open_git_repo(&mut self) -> Result<()> {
        self.push_jump();
        // Context-sensitive: if in a workspace repo, open GitRepo for that repo.
        // Otherwise, fall back to opening the workspace list.
        if let Some(repo_path) = self.active_workspace_repo() {
            let gr = workspace::GitRepo::open(repo_path)?;
            self.pane_layout.active_pane_mut().content = PaneContent::GitRepo(gr);
            self.input.mode = Mode::Normal;
        } else {
            // push_jump already called above; open_workspace would double-push,
            // so inline the workspace open logic here.
            let ws = workspace::Workspace::open()?;
            self.pane_layout.active_pane_mut().content = PaneContent::Workspace(ws);
            self.input.mode = Mode::Normal;
        }
        Ok(())
    }

    fn open_git_status(&mut self) -> Result<()> {
        if let Some(repo_path) = self.active_workspace_repo() {
            self.push_jump();
            let gs = workspace::GitStatus::open(repo_path)?;
            self.pane_layout.active_pane_mut().content = PaneContent::GitStatus(gs);
            self.input.mode = Mode::Normal;
        } else {
            self.status_message = "Not in a workspace repo".to_string();
        }
        Ok(())
    }

    async fn git_add_all(&mut self) -> Result<()> {
        let repo_path = match &self.pane_layout.active_pane().content {
            PaneContent::GitStatus(gs) => gs.repo_path.clone(),
            _ => {
                self.status_message = "Not on a git status view".to_string();
                return Ok(());
            }
        };
        workspace::git_add_all(&repo_path).await?;
        self.status_message = "Staged all files".to_string();
        // Jump back in history.
        Box::pin(self.execute_action(Action::JumpBack)).await?;
        Ok(())
    }

    /// Determine the workspace repo path from the current context.
    /// Checks: active pane content (GitStatus, GitRepo, Workspace selection,
    /// FileBrowser dir, Editor file path).
    fn active_workspace_repo(&self) -> Option<PathBuf> {
        match &self.pane_layout.active_pane().content {
            PaneContent::GitStatus(gs) => Some(gs.repo_path.clone()),
            PaneContent::GitRepo(gr) => Some(gr.repo_path.clone()),
            PaneContent::Workspace(ws) => {
                ws.repos.get(ws.selected).map(|r| r.path.clone())
            }
            PaneContent::FileBrowser(fb) => {
                workspace::workspace_repo_for_path(&fb.current_dir)
            }
            PaneContent::Editor(buf_id) => {
                self.buffers.iter()
                    .find(|b| b.id == *buf_id)
                    .and_then(|b| b.path.as_ref())
                    .and_then(|p| workspace::workspace_repo_for_path(p))
            }
            _ => None,
        }
    }

    /// Get the set of already-cloned repo names.
    fn workspace_cloned_set(&self) -> std::collections::HashSet<String> {
        workspace::Workspace::open()
            .map(|ws| ws.cloned_set())
            .unwrap_or_default()
    }

    async fn handle_workspace_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let PaneContent::Workspace(ref mut ws) = self.pane_layout.active_pane_mut().content {
                    if ws.selected + 1 < ws.repos.len() {
                        ws.selected += 1;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let PaneContent::Workspace(ref mut ws) = self.pane_layout.active_pane_mut().content {
                    ws.selected = ws.selected.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                let repo_path = if let PaneContent::Workspace(ref ws) = self.pane_layout.active_pane().content {
                    ws.repos.get(ws.selected).map(|r| r.path.clone())
                } else {
                    None
                };
                if let Some(path) = repo_path {
                    self.push_jump();
                    self.open_file_browser(path).await?;
                }
            }
            _ => {
                // Fall through to normal mode handling for SPC-prefixed keys etc.
                let action = self.input.handle_key(key);
                self.execute_action(action).await?;
            }
        }
        Ok(())
    }

    async fn handle_git_status_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;

        if self.git_status_bang_pending {
            self.git_status_bang_pending = false;
            if key.code == KeyCode::Char('c') {
                self.git_add_all().await?;
                return Ok(());
            }
            // Not 'c' — cancel the pending bang and process normally.
        }

        match key.code {
            KeyCode::Char('!') => {
                self.git_status_bang_pending = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let PaneContent::GitStatus(ref mut gs) = self.pane_layout.active_pane_mut().content {
                    if gs.selected + 1 < gs.files.len() {
                        gs.selected += 1;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let PaneContent::GitStatus(ref mut gs) = self.pane_layout.active_pane_mut().content {
                    gs.selected = gs.selected.saturating_sub(1);
                }
            }
            KeyCode::Char('r') => {
                // Refresh
                if let PaneContent::GitStatus(ref mut gs) = self.pane_layout.active_pane_mut().content {
                    let _ = gs.refresh();
                }
            }
            _ => {
                let action = self.input.handle_key(key);
                self.execute_action(action).await?;
            }
        }
        Ok(())
    }

    async fn handle_clone_repo_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;
        let ctrl = key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                self.popup = None;
                execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
            }
            KeyCode::Enter => {
                let selected = if let Some(Popup::CloneRepo(ref cr)) = self.popup {
                    cr.selected_repo().map(String::from)
                } else {
                    None
                };

                if let Some(repo) = selected {
                    // Close popup, clone in background.
                    self.popup = None;
                    execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
                    self.status_message = format!("Cloning {}...", repo);

                    match workspace::git_clone(&repo).await {
                        Ok(path) => {
                            self.status_message = format!("Cloned {} to {}", repo, path.display());
                            // Refresh workspace if it's currently shown.
                            if let PaneContent::Workspace(_) = self.pane_layout.active_pane().content {
                                self.open_workspace()?;
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Clone failed: {}", e);
                        }
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') if key.code == KeyCode::Up || ctrl => {
                if let Some(Popup::CloneRepo(ref mut cr)) = self.popup {
                    cr.selected = cr.selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') if key.code == KeyCode::Down || ctrl => {
                if let Some(Popup::CloneRepo(ref mut cr)) = self.popup {
                    if cr.selected + 1 < cr.filtered.len() {
                        cr.selected += 1;
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(Popup::CloneRepo(ref mut cr)) = self.popup {
                    cr.query.pop();
                    cr.refilter();
                }
            }
            KeyCode::Char(c) => {
                // Check if typing a slash triggers a user fetch.
                let should_fetch = if c == '/' {
                    if let Some(Popup::CloneRepo(ref cr)) = self.popup {
                        let user = cr.query.trim_end_matches('/').to_string();
                        !user.is_empty() && !cr.cache.users.contains_key(&user)
                    } else {
                        false
                    }
                } else {
                    false
                };

                if let Some(Popup::CloneRepo(ref mut cr)) = self.popup {
                    cr.query.push(c);
                    cr.refilter();
                }

                if should_fetch {
                    let user = if let Some(Popup::CloneRepo(ref mut cr)) = self.popup {
                        cr.loading = Some(cr.query.trim_end_matches('/').to_string());
                        cr.query.trim_end_matches('/').to_string()
                    } else {
                        String::new()
                    };

                    if !user.is_empty() {
                        match workspace::fetch_user_repos(&user).await {
                            Ok(repos) => {
                                if let Some(Popup::CloneRepo(ref mut cr)) = self.popup {
                                    cr.cache.add_user(user, repos);
                                    cr.loading = None;
                                    cr.error = None;
                                    cr.refilter();
                                }
                            }
                            Err(e) => {
                                if let Some(Popup::CloneRepo(ref mut cr)) = self.popup {
                                    cr.loading = None;
                                    cr.error = Some(format!("{}", e));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_mouse_click(&mut self, col: u16, row: u16) -> Result<()> {
        // Only handle clicks on find-file overlay for now.
        if let Some(Popup::FindFile(ref mut ff)) = self.popup {
            let size = self.terminal.size()?;
            // Replicate the popup geometry from render_find_file.
            let width = (size.width * 3 / 5).max(40).min(size.width.saturating_sub(4));
            let max_height = (size.height * 7 / 10).max(10).min(size.height.saturating_sub(2));
            let x = (size.width.saturating_sub(width)) / 2;
            let y = (size.height.saturating_sub(max_height)) / 2;

            // Inner area (1px border on each side).
            let inner_x = x + 1;
            let inner_y = y + 1;
            let inner_w = width.saturating_sub(2);
            let inner_h = max_height.saturating_sub(2);

            if inner_h < 2 || inner_w < 4 {
                return Ok(());
            }

            // List area starts 1 row below the input line.
            let list_y = inner_y + 1;
            let list_h = inner_h.saturating_sub(1);

            // Check if click is within the list area.
            if col >= inner_x && col < inner_x + inner_w
                && row >= list_y && row < list_y + list_h
            {
                let visible_count = list_h as usize;
                let scroll_offset = if ff.selected >= visible_count {
                    ff.selected - visible_count + 1
                } else {
                    0
                };
                let clicked_index = scroll_offset + (row - list_y) as usize;
                if clicked_index < ff.filtered.len() {
                    ff.selected = clicked_index;
                    // Simulate Enter on the clicked entry — take FindFile out.
                    let ff = match self.popup.take() {
                        Some(Popup::FindFile(ff)) => ff,
                        _ => unreachable!(),
                    };
                    if ff.is_dot_selected() {
                        let dir = ff.dir.clone();
                        self.push_jump();
                        self.open_file_browser(dir).await?;
                    } else if let Some(entry) = ff.selected_entry().cloned() {
                        if entry.is_dir {
                            let mut ff = ff;
                            ff.enter_dir(&entry.path);
                            self.popup = Some(Popup::FindFile(ff));
                        } else {
                            self.push_jump();
                            let path_str = entry.path.to_string_lossy().to_string();
                            self.open_file(&path_str).await?;
                        }
                    }
                }
            }
        }
        if !matches!(self.popup, Some(Popup::FindFile(_))) {
            execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
        }
        Ok(())
    }

    /// Sync pane↔buffer cursors around a buffer operation. Returns whatever
    /// the closure returns, or None if no active buffer exists.
    fn sync_buffer<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut TextBuffer) -> R,
    {
        let Self { buffers, pane_layout, .. } = self;
        let pane = pane_layout.active_pane_mut();
        let buf_id = pane.content.buffer_id()?;
        let buf = buffers.iter_mut().find(|b| b.id == buf_id)?;
        buf.cursor = pane.cursor;
        buf.scroll_offset = pane.scroll_offset;
        let result = f(buf);
        pane.cursor = buf.cursor;
        pane.scroll_offset = buf.scroll_offset;
        Some(result)
    }

    fn with_buffer<F>(&mut self, f: F)
    where
        F: FnOnce(&mut TextBuffer),
    {
        self.sync_buffer(f);
    }

    /// Repeat a motion `count` times, stopping early if the cursor doesn't move.
    fn repeat_motion(&mut self, count: usize, action: &Action, viewport_height: usize) {
        self.sync_buffer(|b| {
            for _ in 0..count {
                let prev = (b.cursor.line, b.cursor.col);
                apply_motion(b, action, viewport_height);
                if (b.cursor.line, b.cursor.col) == prev {
                    break;
                }
            }
        });
    }

    /// Adjust scroll offset so the cursor stays visible.
    fn scroll_to_cursor(&mut self, viewport_height: usize) {
        let pane = self.pane_layout.active_pane_mut();
        if pane.cursor.line < pane.scroll_offset {
            pane.scroll_offset = pane.cursor.line;
        }
        if pane.cursor.line >= pane.scroll_offset + viewport_height {
            pane.scroll_offset = pane.cursor.line - viewport_height + 1;
        }
    }

    /// Call a buffer edit method that returns Option<EditInfo>, then do
    /// incremental tree-sitter parse and update the highlight cache.
    fn with_buffer_edit<F>(&mut self, f: F)
    where
        F: FnOnce(&mut TextBuffer) -> Option<EditInfo>,
    {
        let Self { buffers, pane_layout, highlight_engine, highlight_cache, .. } = self;
        let pane = pane_layout.active_pane_mut();
        let buf_id = match pane.content.buffer_id() {
            Some(id) => id,
            None => return,
        };
        let buf = match buffers.iter_mut().find(|b| b.id == buf_id) {
            Some(b) => b,
            None => return,
        };
        buf.cursor = pane.cursor;
        buf.scroll_offset = pane.scroll_offset;
        let edit_info = f(buf);
        pane.cursor = buf.cursor;
        pane.scroll_offset = buf.scroll_offset;

        // Incremental re-highlight, falling back to full parse for
        // buffers with no grammar state (e.g. markdown/plain text).
        if let Some(edit) = edit_info {
            let source = buf.text_snapshot();
            let highlights = highlight_engine
                .parse_incremental(buf_id, &source, &edit)
                .unwrap_or_else(|| {
                    let path = buf.path.clone();
                    highlight_engine.parse_full(buf_id, &source, path.as_deref())
                });
            highlight_cache.insert(buf_id, highlights);
        }
    }

    fn switch_buffer_by_offset(&mut self, offset: isize) {
        if self.buffers.is_empty() {
            return;
        }
        self.push_jump();
        let cur = self.active_buffer_idx().unwrap_or(0) as isize;
        let len = self.buffers.len() as isize;
        let next = ((cur + offset) % len + len) % len;
        let next_id = self.buffers[next as usize].id;
        self.pane_layout.active_pane_mut().switch_buffer(next_id);
    }

    fn enter_awaiting_char(&mut self, state: AwaitingChar, display: &str) {
        self.awaiting_char = Some(state);
        self.input.awaiting_char = true;
        self.input.pending_display = display.to_string();
    }

    fn exit_visual_mode(&mut self) {
        self.input.mode = Mode::Normal;
        self.visual_anchor = None;
    }

    fn play_macro(&mut self, reg: char, count: usize) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + '_>> {
        Box::pin(async move {
            if let Some(actions) = self.macro_registers.get(&reg).cloned() {
                self.last_macro_register = Some(reg);
                for _ in 0..count {
                    for a in &actions {
                        self.execute_action(a.clone()).await?;
                    }
                }
            } else {
                self.status_message = format!("Macro @{} is empty", reg);
            }
            Ok(())
        })
    }

    fn rehighlight_active(&mut self) {
        if let Some(buf) = self.active_buffer() {
            let buf_id = buf.id;
            let source = buf.text_snapshot();
            let path = buf.path.clone();
            let highlights = self.highlight_engine.parse_full(buf_id, &source, path.as_deref());
            self.highlight_cache.insert(buf_id, highlights);
        }
    }

    /// Flush swap files for buffers whose content has changed since the last write.
    async fn flush_swap_files(&mut self) {
        let buf_ids = self.swap_manager.registered_buffer_ids();
        for buf_id in buf_ids {
            let buf = match self.buffers.iter_mut().find(|b| b.id == buf_id) {
                Some(b) => b,
                None => continue,
            };
            if !buf.modified {
                continue;
            }
            let hash = buf.content_hash();
            if !self.swap_manager.needs_swap_write(buf_id, hash) {
                continue;
            }
            let content = buf.text_snapshot();
            let (swap_path, source_path) = match self.swap_manager.swap_info(buf_id) {
                Some((s, p)) => (s.to_path_buf(), p.to_path_buf()),
                None => continue,
            };
            let meta_path = match self.swap_manager.meta_path(buf_id) {
                Some(p) => p.to_path_buf(),
                None => continue,
            };
            tokio::task::spawn_blocking(move || {
                SwapManager::write_swap_file(&swap_path, &meta_path, &source_path, &content);
            })
            .await
            .ok();
            self.swap_manager.update_swap_hash(buf_id, hash);
        }
    }

    /// Capture the current state as a jump position.
    /// Reattach a terminal to the active pane. If the terminal is on another pane,
    /// the two panes swap content (unique content rule). If detached, it's placed
    /// directly. If not found, a new terminal is opened.
    fn reattach_terminal(&mut self, term_id: usize) -> Result<()> {
        // Already on the active pane — nothing to do.
        if self.pane_layout.active_pane().content.as_terminal()
            .map_or(false, |t| t.id == term_id)
        {
            self.input.mode = Mode::Insert;
            return Ok(());
        }

        // On another pane — swap content (unique content can only be on one pane).
        let donor_id = self.pane_layout.panes.iter()
            .find(|p| p.id != self.pane_layout.active_id
                    && p.content.as_terminal().map_or(false, |t| t.id == term_id))
            .map(|p| p.id);
        if let Some(donor_id) = donor_id {
            self.pane_layout.swap_content(self.pane_layout.active_id, donor_id);
            self.input.mode = Mode::Insert;
            return Ok(());
        }

        // Detached — place directly (detach current content first).
        if let Some(term) = self.detached_terminals.remove(&term_id) {
            self.detach_terminal_from_active_pane();
            self.pane_layout.active_pane_mut().content = PaneContent::Terminal(term);
            self.input.mode = Mode::Insert;
            return Ok(());
        }

        // Terminal no longer exists — open a new one.
        self.open_terminal(None)
    }

    /// Detach the terminal from the active pane, moving it to detached storage.
    fn detach_terminal_from_active_pane(&mut self) {
        if let Some(term) = self.pane_layout.active_pane_mut().content.take_terminal() {
            self.detached_terminals.insert(term.id, term);
        }
    }

    fn current_jump_position(&self) -> PaneContentSnapshot {
        let pane = self.pane_layout.active_pane();
        pane.content.snapshot(&pane.cursor)
    }

    /// Get the (back, forward) jump stacks for the active pane.
    fn jump_stacks_mut(&mut self) -> &mut (Vec<PaneContentSnapshot>, Vec<PaneContentSnapshot>) {
        let pane_id = self.pane_layout.active_id;
        self.jump_history.entry(pane_id).or_insert_with(|| (Vec::new(), Vec::new()))
    }

    /// Push a specific browser directory as a jump position.
    fn push_browser_jump(&mut self, dir: PathBuf, selected: usize, scroll_offset: usize) {
        let stacks = self.jump_stacks_mut();
        stacks.0.push(PaneContentSnapshot::FileBrowser { dir, selected, scroll_offset });
        stacks.1.clear();
        if stacks.0.len() > 100 {
            stacks.0.remove(0);
        }
    }

    /// Record the current position in the jump-back stack.
    fn push_jump(&mut self) {
        let pos = self.current_jump_position();
        let stacks = self.jump_stacks_mut();
        stacks.0.push(pos);
        stacks.1.clear();
        if stacks.0.len() > 100 {
            stacks.0.remove(0);
        }
    }

    fn jump_back(&mut self) {
        let current = self.current_jump_position();
        let stacks = self.jump_stacks_mut();
        if let Some(pos) = stacks.0.pop() {
            stacks.1.push(current);
            self.restore_jump(pos);
        } else {
            self.status_message = "No older jump position".into();
        }
    }

    fn jump_forward(&mut self) {
        let current = self.current_jump_position();
        let stacks = self.jump_stacks_mut();
        if let Some(pos) = stacks.1.pop() {
            stacks.0.push(current);
            self.restore_jump(pos);
        } else {
            self.status_message = "No newer jump position".into();
        }
    }

    fn restore_jump(&mut self, pos: PaneContentSnapshot) {
        match pos {
            PaneContentSnapshot::Welcome => {
                self.pane_layout.active_pane_mut().content = PaneContent::Welcome;
            }
            PaneContentSnapshot::Editor { buffer_id, line, col } => {
                if self.buffers.iter().any(|b| b.id == buffer_id) {
                    let pane = self.pane_layout.active_pane_mut();
                    pane.switch_buffer(buffer_id);
                    pane.cursor.line = line;
                    pane.cursor.col = col;
                    if let Some(buf) = self.buffers.iter().find(|b| b.id == buffer_id) {
                        let line_count = buf.line_count();
                        let pane = self.pane_layout.active_pane_mut();
                        if pane.cursor.line >= line_count {
                            pane.cursor.line = line_count.saturating_sub(1);
                        }
                    }
                } else {
                    self.status_message = "Buffer no longer exists".into();
                }
            }
            PaneContentSnapshot::FileBrowser { dir, selected, scroll_offset } => {
                let mut fb = crate::file_browser::FileBrowser::new(dir.clone());
                let entries = crate::file_browser::scan_directory(&dir);
                fb.set_entries(entries);
                let count = fb.visible_count();
                fb.selected = if selected < count { selected } else { count.saturating_sub(1) };
                fb.scroll_offset = scroll_offset;
                self.pane_layout.active_pane_mut().content = PaneContent::FileBrowser(fb);
            }
            PaneContentSnapshot::Terminal { term_id } => {
                let _ = self.reattach_terminal(term_id);
            }
            PaneContentSnapshot::Workspace { selected } => {
                match workspace::Workspace::open() {
                    Ok(mut ws) => {
                        ws.selected = selected.min(ws.repos.len().saturating_sub(1));
                        self.pane_layout.active_pane_mut().content = PaneContent::Workspace(ws);
                    }
                    Err(e) => self.status_message = format!("Failed to open workspace: {e}"),
                }
            }
            PaneContentSnapshot::GitRepo { repo_path } => {
                match workspace::GitRepo::open(repo_path) {
                    Ok(gr) => {
                        self.pane_layout.active_pane_mut().content = PaneContent::GitRepo(gr);
                    }
                    Err(e) => self.status_message = format!("Failed to open git repo: {e}"),
                }
            }
            PaneContentSnapshot::GitStatus { repo_path, selected } => {
                match workspace::GitStatus::open(repo_path) {
                    Ok(mut gs) => {
                        gs.selected = selected.min(gs.files.len().saturating_sub(1));
                        self.pane_layout.active_pane_mut().content = PaneContent::GitStatus(gs);
                    }
                    Err(e) => self.status_message = format!("Failed to open git status: {e}"),
                }
            }
        }
    }

    // -- Visual mode --

    /// Get the visual selection range as char indices (start, end) where start <= end.
    /// The selection is inclusive of the anchor and cursor positions.
    fn visual_char_range(&self) -> Option<(usize, usize)> {
        let (anchor_line, anchor_col) = self.visual_anchor?;
        let pane = self.pane_layout.active_pane();
        let buf = self.active_buffer()?;
        let anchor_idx = buf.char_idx_at(anchor_line, anchor_col);
        let cursor_idx = buf.char_idx_at(pane.cursor.line, pane.cursor.col);
        let start = anchor_idx.min(cursor_idx);
        // +1 to make selection inclusive of the character under cursor.
        let end = (anchor_idx.max(cursor_idx) + 1).min(buf.rope.len_chars());
        Some((start, end))
    }

    fn visual_operate(&mut self, op: VisualOp) {
        let range = self.visual_char_range();
        let (start, end) = match range {
            Some(r) if r.0 < r.1 => r,
            _ => {
                self.exit_visual_mode();
                return;
            }
        };

        match op {
            VisualOp::Delete => {
                self.yank_range(start, end);
                self.with_buffer_edit(|b| b.delete_char_range(start, end));
                self.rehighlight_active();
            }
            VisualOp::Change => {
                self.yank_range(start, end);
                self.with_buffer_edit(|b| b.delete_char_range(start, end));
                self.rehighlight_active();
                self.input.mode = Mode::Insert;
                self.visual_anchor = None;
                return;
            }
            VisualOp::Yank => {
                self.yank_range(start, end);
                self.status_message = format!("{} chars yanked", end - start);
            }
        }
        self.exit_visual_mode();
    }

    /// Surround the visual selection with a character pair.
    /// Brackets get paired (e.g. '(' wraps with '(' and ')'),
    /// other characters use the same char on both sides.
    fn visual_surround(&mut self, c: char) {
        let range = self.visual_char_range();
        let (start, end) = match range {
            Some(r) if r.0 < r.1 => r,
            _ => {
                self.exit_visual_mode();
                return;
            }
        };

        let (open, close) = match c {
            '(' | ')' => ('(', ')'),
            '[' | ']' => ('[', ']'),
            '{' | '}' => ('{', '}'),
            '<' | '>' => ('<', '>'),
            _ => (c, c),
        };

        // Insert close at end first (preserves start index), then open at start.
        // Single undo snapshot so the whole surround undoes as one operation.
        self.sync_buffer(|b| {
            b.save_undo();
            let close_str = close.to_string();
            let open_str = open.to_string();
            b.rope.insert(end, &close_str);
            b.rope.insert(start, &open_str);
            b.mark_modified();
        });
        self.rehighlight_active();
        self.exit_visual_mode();
    }

    // -- Substitute --

    fn execute_substitute(&mut self, sub: Substitute) {
        let line_range = self.sync_buffer(|b| {
            if sub.whole_file {
                0..b.rope.len_lines()
            } else {
                b.cursor.line..b.cursor.line + 1
            }
        });
        let line_range = match line_range {
            Some(r) => r,
            None => return,
        };

        // Collect all match positions (line, col_start) in forward order.
        let matches = self.sync_buffer(|b| {
            let mut all = Vec::new();
            for line_idx in line_range.clone() {
                if line_idx >= b.rope.len_lines() {
                    continue;
                }
                let line_str: String = b.rope.line(line_idx).chars().collect();
                let mut search_start = 0;
                while search_start < line_str.len() {
                    if let Some(byte_pos) = line_str[search_start..].find(&sub.pattern) {
                        let abs = search_start + byte_pos;
                        let col_start = line_str[..abs].chars().count();
                        all.push((line_idx, col_start));
                        if !sub.global {
                            break;
                        }
                        search_start = abs + sub.pattern.len();
                    } else {
                        break;
                    }
                }
            }
            all
        }).unwrap_or_default();

        if matches.is_empty() {
            self.status_message = format!("Pattern not found: {}", sub.pattern);
            return;
        }

        if sub.confirm {
            // Enter interactive confirm mode — step through matches one at a time.
            let (first_line, first_col) = matches[0];
            self.sync_buffer(|b| {
                b.cursor.line = first_line;
                b.cursor.col = first_col;
            });
            self.substitute_confirm = Some(SubstituteConfirm {
                replacement: sub.replacement,
                pattern_char_len: sub.pattern.chars().count(),
                remaining: matches,
                current: 0,
                replaced: 0,
                undo_saved: false,
            });
            self.status_message = "Replace? (y)es (n)o (a)ll (q)uit".into();
            return;
        }

        // Non-interactive: replace all at once.
        let total = self.sync_buffer(|b| {
            b.save_undo();
            let mut total_replacements = 0usize;

            // Process in reverse so edits don't shift earlier offsets.
            for &(line_idx, col_start) in matches.iter().rev() {
                let line_char_start = b.rope.line_to_char(line_idx);
                let char_start = line_char_start + col_start;
                let char_end = char_start + sub.pattern.chars().count();
                b.rope.remove(char_start..char_end);
                b.rope.insert(char_start, &sub.replacement);
                total_replacements += 1;
            }

            if total_replacements > 0 {
                b.invalidate_hash();
                b.modified = true;
            }
            total_replacements
        }).unwrap_or(0);

        self.rehighlight_active();
        self.status_message = format!("{} substitution(s) made", total);
    }

    fn handle_substitute_confirm_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        let action = match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => 'y',
            KeyCode::Char('n') | KeyCode::Char('N') => 'n',
            KeyCode::Char('a') | KeyCode::Char('A') => 'a',
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => 'q',
            _ => return, // Ignore other keys.
        };

        let state = match self.substitute_confirm.as_mut() {
            Some(s) => s,
            None => return,
        };

        match action {
            'y' => {
                // Replace the current match and advance.
                let (line_idx, col_start) = state.remaining[state.current];
                let replacement = state.replacement.clone();
                let pat_len = state.pattern_char_len;
                let save_undo = !state.undo_saved;
                self.sync_buffer(|b| {
                    if save_undo {
                        b.save_undo();
                    }
                    let line_char_start = b.rope.line_to_char(line_idx);
                    let char_start = line_char_start + col_start;
                    let char_end = char_start + pat_len;
                    b.rope.remove(char_start..char_end);
                    b.rope.insert(char_start, &replacement);
                    b.invalidate_hash();
                    b.modified = true;
                });
                let state = self.substitute_confirm.as_mut().unwrap();
                state.undo_saved = true;
                state.replaced += 1;

                // Adjust remaining matches on the same line after this one,
                // since the replacement may have changed their column offsets.
                let col_delta = state.replacement.chars().count() as isize - state.pattern_char_len as isize;
                for i in (state.current + 1)..state.remaining.len() {
                    if state.remaining[i].0 == line_idx && state.remaining[i].1 > col_start {
                        state.remaining[i].1 = (state.remaining[i].1 as isize + col_delta) as usize;
                    }
                }

                state.current += 1;
                self.substitute_confirm_advance();
            }
            'n' => {
                // Skip this match, advance to next.
                let state = self.substitute_confirm.as_mut().unwrap();
                state.current += 1;
                self.substitute_confirm_advance();
            }
            'a' => {
                // Replace all remaining matches at once.
                let state = self.substitute_confirm.take().unwrap();
                let remaining: Vec<(usize, usize)> = state.remaining[state.current..].to_vec();
                let replacement = state.replacement.clone();
                let pat_len = state.pattern_char_len;
                let save_undo = !state.undo_saved;
                let mut count = state.replaced;

                self.sync_buffer(|b| {
                    if save_undo {
                        b.save_undo();
                    }
                    // Apply in reverse to preserve offsets.
                    for &(line_idx, col_start) in remaining.iter().rev() {
                        let line_char_start = b.rope.line_to_char(line_idx);
                        let char_start = line_char_start + col_start;
                        let char_end = char_start + pat_len;
                        b.rope.remove(char_start..char_end);
                        b.rope.insert(char_start, &replacement);
                        count += 1;
                    }
                    if count > 0 {
                        b.invalidate_hash();
                        b.modified = true;
                    }
                });

                self.rehighlight_active();
                self.status_message = format!("{} substitution(s) made", count);
            }
            'q' => {
                // Quit confirmation, keep replacements already made.
                let state = self.substitute_confirm.take().unwrap();
                self.rehighlight_active();
                if state.replaced > 0 {
                    self.status_message = format!("{} substitution(s) made", state.replaced);
                } else {
                    self.status_message = "Substitution cancelled".into();
                }
            }
            _ => {}
        }
    }

    /// Advance to the next match in substitute confirm, or finish if done.
    fn substitute_confirm_advance(&mut self) {
        let done = {
            let state = match self.substitute_confirm.as_ref() {
                Some(s) => s,
                None => return,
            };
            state.current >= state.remaining.len()
        };

        if done {
            let state = self.substitute_confirm.take().unwrap();
            self.rehighlight_active();
            self.status_message = format!("{} substitution(s) made", state.replaced);
        } else {
            // Move cursor to the next match.
            let (line, col) = {
                let state = self.substitute_confirm.as_ref().unwrap();
                state.remaining[state.current]
            };
            self.sync_buffer(|b| {
                b.cursor.line = line;
                b.cursor.col = col;
            });
            self.rehighlight_active();
            self.status_message = "Replace? (y)es (n)o (a)ll (q)uit".into();
        }
    }

    // -- Operators --

    /// Yank a char range from the active buffer into the clipboard.
    /// If the yanked text has no visible content (only newlines), clears
    /// the clipboard instead.
    fn yank_range(&mut self, start: usize, end: usize) {
        let text = self.sync_buffer(|b| b.text_in_char_range(start, end))
            .unwrap_or_default();
        if let Some(cb) = &mut self.clipboard {
            if text.trim().is_empty() {
                let _ = cb.clear();
            } else {
                let _ = cb.set_text(text);
            }
        }
    }

    /// Execute a linewise operator (dd, cc, yy) with count.
    fn execute_linewise_operator(&mut self, op: &Action, count: usize) {
        self.input.clear_pending();
        match op {
            Action::OperatorDelete => {
                let range = self.sync_buffer(|b| {
                    let last = (b.cursor.line + count - 1).min(b.line_count().saturating_sub(1));
                    b.linewise_range(b.cursor.line, last)
                });
                if let Some((start, end)) = range {
                    self.yank_range(start, end);
                    self.with_buffer_edit(|b| b.delete_char_range(start, end));
                    self.rehighlight_active();
                }
            }
            Action::OperatorChange => {
                // Yank the lines being changed (consistent with dd).
                let range = self.sync_buffer(|b| {
                    let last = (b.cursor.line + count - 1).min(b.line_count().saturating_sub(1));
                    b.linewise_range(b.cursor.line, last)
                });
                if let Some((start, end)) = range {
                    self.yank_range(start, end);
                    // Single undo entry: delete range and insert blank line atomically.
                    self.with_buffer_edit(|b| {
                        b.save_undo();
                        let end = end.min(b.rope.len_chars());
                        if start < end {
                            b.rope.remove(start..end);
                        }
                        let pos = start.min(b.rope.len_chars());
                        b.rope.insert_char(pos, '\n');
                        b.cursor.line = b.rope.char_to_line(pos);
                        b.cursor.col = 0;
                        b.mark_modified();
                        None
                    });
                    self.rehighlight_active();
                }
                self.input.mode = Mode::Insert;
            }
            Action::OperatorYank => {
                let range = self.sync_buffer(|b| {
                    let last = (b.cursor.line + count - 1).min(b.line_count().saturating_sub(1));
                    b.linewise_range(b.cursor.line, last)
                });
                if let Some((start, end)) = range {
                    self.yank_range(start, end);
                    self.status_message = format!("{} lines yanked", count);
                }
            }
            _ => {}
        }
    }

    /// Execute an operator composed with a motion.
    fn execute_operator_motion(&mut self, op: &Action, motion: &Action, count: usize) {
        self.input.clear_pending();
        let viewport_height = self.active_viewport_height();
        let motion = motion.clone();
        let linewise = motion.is_linewise_motion();
        let exclusive = motion.is_exclusive_motion();

        // Compute the range by saving cursor, applying motion, reading new cursor.
        // Returns ((start, end), (orig_line, orig_col)).
        let range = self.sync_buffer(|b| {
            let orig_line = b.cursor.line;
            let orig_col = b.cursor.col;
            let start_line = b.cursor.line;
            let start_col = b.cursor.col;

            // Apply motion `count` times, with early break when cursor stops moving.
            for _ in 0..count {
                let prev = (b.cursor.line, b.cursor.col);
                apply_motion(b, &motion, viewport_height);
                if (b.cursor.line, b.cursor.col) == prev {
                    break;
                }
            }

            let end_line = b.cursor.line;
            let end_col = b.cursor.col;

            if linewise {
                // Linewise: expand to full lines.
                let first_line = start_line.min(end_line);
                let last_line = start_line.max(end_line);
                let (start, end) = b.linewise_range(first_line, last_line);
                // Restore cursor to start of range for deletion.
                b.cursor.line = first_line;
                b.cursor.col = 0;
                ((start, end), (orig_line, orig_col))
            } else {
                // Characterwise: range depends on exclusive/inclusive.
                // Exclusive motions don't include the destination character.
                // Inclusive motions include it.
                let start = b.char_idx_at(start_line, start_col);
                let end = b.char_idx_at(end_line, end_col);
                let (min, max) = if start <= end { (start, end) } else { (end, start) };
                let mut to = if exclusive { max } else { max + 1 };
                // When an exclusive motion is clamped at a boundary (cursor
                // didn't move), the range is empty. Forward motions should
                // still act on one character (e.g. `dl` at end of line
                // deletes the last char). Backward motions correctly do
                // nothing (e.g. `dh` at start of line). On empty lines,
                // do nothing — there's no character to act on.
                if min == to && exclusive && !motion.is_backward_motion()
                    && b.current_line_len() > 0
                {
                    to = (min + 1).min(b.rope.len_chars());
                }
                let (from, to) = (min, to.min(b.rope.len_chars()));
                // Restore cursor to start of range.
                if start <= end {
                    b.cursor.line = start_line;
                    b.cursor.col = start_col;
                } else {
                    b.cursor.line = end_line;
                    b.cursor.col = end_col;
                }
                ((from, to), (orig_line, orig_col))
            }
        });

        let ((start, end), (orig_line, orig_col)) = match range {
            Some(r) => r,
            None => return,
        };

        if start >= end {
            // Empty range: nothing to delete/change, but all operators
            // clear the clipboard (delete/change yank before acting).
            self.yank_range(start, end);
            return;
        }

        match op {
            Action::OperatorDelete => {
                // Temporarily restore original cursor so save_undo captures it.
                self.with_buffer(|b| { b.cursor.line = orig_line; b.cursor.col = orig_col; });
                self.yank_range(start, end);
                self.with_buffer_edit(|b| b.delete_char_range(start, end));
                self.rehighlight_active();
            }
            Action::OperatorChange => {
                self.with_buffer(|b| { b.cursor.line = orig_line; b.cursor.col = orig_col; });
                self.yank_range(start, end);
                self.with_buffer_edit(|b| b.delete_char_range(start, end));
                self.rehighlight_active();
                self.input.mode = Mode::Insert;
            }
            Action::OperatorYank => {
                self.yank_range(start, end);
                // Restore cursor to original position — yank doesn't move.
                self.with_buffer(|b| {
                    b.cursor.line = orig_line;
                    b.cursor.col = orig_col;
                });
                self.status_message = if linewise {
                    let line_count = self.sync_buffer(|b| {
                        let sl = b.rope.char_to_line(start);
                        let el = b.rope.char_to_line(end.saturating_sub(1).max(start));
                        el - sl + 1
                    }).unwrap_or(1);
                    format!("{} lines yanked", line_count)
                } else {
                    format!("{} chars yanked", end - start)
                };
            }
            _ => {}
        }
    }

    // -- Search --

    /// Find all occurrences of `pattern` in the active buffer.
    fn find_all_matches(&self, pattern: &str) -> Vec<(usize, usize, usize)> {
        let buf = match self.active_buffer() {
            Some(b) => b,
            None => return Vec::new(),
        };
        find_all_matches_in_rope(&buf.rope, pattern)
    }

    /// Execute a search: store pattern, find matches, jump to first match.
    fn execute_search(&mut self, pattern: String, direction: SearchDirection) {
        self.search.last_pattern = Some(pattern.clone());
        self.search.last_direction = direction;
        self.compute_search_matches_for_active(&pattern);

        let buf_id = self.active_buffer().map(|b| b.id);
        let matches = buf_id.and_then(|id| self.search.buffer_matches.get(&id));
        if matches.map_or(true, |m| m.matches.is_empty()) {
            self.status_message = format!("Pattern not found: {}", pattern);
            return;
        }

        self.jump_to_next_match(direction);
    }

    /// Compute and store search matches for the active buffer.
    fn compute_search_matches_for_active(&mut self, pattern: &str) {
        if let Some(buf_id) = self.active_buffer().map(|b| b.id) {
            let matches = self.find_all_matches(pattern);
            self.search.buffer_matches.insert(buf_id, BufferSearchMatches {
                matches,
                current_match: None,
            });
        }
    }

    /// Jump to the next (or previous if `reverse` is true) match.
    fn search_next_prev(&mut self, reverse: bool) {
        if self.search.last_pattern.is_none() {
            self.status_message = "No previous search".into();
            return;
        }

        // Recompute matches (buffer may have changed since last search).
        let pattern = self.search.last_pattern.clone().unwrap();
        self.compute_search_matches_for_active(&pattern);

        let buf_id = self.active_buffer().map(|b| b.id);
        let matches = buf_id.and_then(|id| self.search.buffer_matches.get(&id));
        if matches.map_or(true, |m| m.matches.is_empty()) {
            self.status_message = format!("Pattern not found: {}", pattern);
            return;
        }

        let direction = if reverse {
            self.search.last_direction.opposite()
        } else {
            self.search.last_direction
        };
        self.jump_to_next_match(direction);
    }

    /// Jump cursor to the next match in `direction` from the current cursor position.
    fn jump_to_next_match(&mut self, direction: SearchDirection) {
        let pane = self.pane_layout.active_pane();
        let cur_line = pane.cursor.line;
        let cur_col = pane.cursor.col;
        let buf_id = match pane.content.buffer_id() {
            Some(id) => id,
            None => return,
        };

        let (idx, line, col, match_count) = {
            let bm = match self.search.buffer_matches.get(&buf_id) {
                Some(bm) if !bm.matches.is_empty() => bm,
                _ => return,
            };
            let matches = &bm.matches;

            let idx = match direction {
                SearchDirection::Forward => {
                    matches
                        .iter()
                        .position(|&(line, col, _)| line > cur_line || (line == cur_line && col > cur_col))
                        .unwrap_or(0)
                }
                SearchDirection::Backward => {
                    matches
                        .iter()
                        .rposition(|&(line, col, _)| line < cur_line || (line == cur_line && col < cur_col))
                        .unwrap_or(matches.len() - 1)
                }
            };

            let (line, col, _) = matches[idx];
            (idx, line, col, matches.len())
        };

        if let Some(bm) = self.search.buffer_matches.get_mut(&buf_id) {
            bm.current_match = Some(idx);
        }

        let pane = self.pane_layout.active_pane_mut();
        pane.cursor.line = line;
        pane.cursor.col = col;

        let wrapped = match direction {
            SearchDirection::Forward => idx == 0 && (cur_line > line || (cur_line == line && cur_col >= col)),
            SearchDirection::Backward => idx == match_count - 1 && (cur_line < line || (cur_line == line && cur_col <= col)),
        };
        let wrap_msg = if wrapped { " [wrapped]" } else { "" };
        self.status_message = format!(
            "[{}/{}]{}",
            idx + 1,
            match_count,
            wrap_msg,
        );
    }

    fn cleanup(&mut self) {
        for pane in &mut self.pane_layout.panes {
            if let Some(mut term) = pane.content.take_terminal() {
                term.kill();
            }
        }
        for (_, mut term) in self.detached_terminals.drain() {
            term.kill();
        }
        self.swap_manager.cleanup_all();
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            crossterm::event::DisableMouseCapture,
            LeaveAlternateScreen,
            cursor::SetCursorStyle::DefaultUserShape
        );
        let _ = self.terminal.show_cursor();
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
