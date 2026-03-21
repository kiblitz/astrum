use crate::action::Action;
use crate::buffer::Buffer;
use crate::config::Config;
use crate::file_browser::{scan_directory, FileBrowser, FileBrowserResult};
use crate::input::{InputHandler, Mode};
use crate::pane::{FocusDirection, PaneLayout, SplitDirection};
use crate::renderer::Renderer;
use crate::syntax::{HighlightCache, HighlightEngine, HighlightResult};
use anyhow::Result;
use crossterm::event::{Event, KeyEventKind, MouseEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::collections::HashMap;
use std::io::{self, Stdout};
use std::path::PathBuf;
use tokio::sync::mpsc;

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
        Self { dir, input: String::new(), entries, filtered, selected: 0 }
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
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
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
    fn enter_dir(&mut self, path: &std::path::Path) {
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
        self.selected = 0;
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

/// A position in the jump history.
#[derive(Debug, Clone)]
enum JumpPosition {
    /// Cursor position in a buffer.
    Buffer {
        buffer_id: usize,
        line: usize,
        col: usize,
    },
    /// File browser open on a directory.
    Browser {
        dir: PathBuf,
        selected: usize,
        scroll_offset: usize,
    },
}

pub struct Editor {
    buffers: Vec<Buffer>,
    pane_layout: PaneLayout,
    input: InputHandler,
    renderer: Renderer,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    highlight_engine: HighlightEngine,
    highlight_cache: HighlightCache,
    highlight_rx: mpsc::Receiver<HighlightResult>,
    /// Per-pane file browsers. Key is the pane id.
    file_browsers: HashMap<usize, FileBrowser>,
    status_message: String,
    should_quit: bool,
    clipboard: Option<arboard::Clipboard>,
    jump_back: Vec<JumpPosition>,
    jump_forward: Vec<JumpPosition>,
    palette: Option<PaletteState>,
    find_file: Option<FindFileState>,
    /// Set after the first `:q` on the last pane. Cleared by any substantive action.
    quit_pending: bool,
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

        let (highlight_tx, highlight_rx) = mpsc::channel(32);

        let input = InputHandler::new(config.keymap);
        let highlight_engine = HighlightEngine::new(&config.general.theme, highlight_tx);

        Ok(Self {
            buffers: Vec::new(),
            pane_layout: PaneLayout::new(),
            input,
            renderer: Renderer::new(),
            terminal,
            highlight_engine,
            highlight_cache: HighlightCache::new(),
            highlight_rx,
            file_browsers: HashMap::new(),
            status_message: String::new(),
            should_quit: false,
            clipboard: arboard::Clipboard::new().ok(),
            jump_back: Vec::new(),
            jump_forward: Vec::new(),
            palette: None,
            find_file: None,
            quit_pending: false,
        })
    }

    // -- Helper methods --

    fn active_buffer(&self) -> Option<&Buffer> {
        let bid = self.pane_layout.active_pane().buffer_id?;
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
        let bid = self.pane_layout.active_pane().buffer_id?;
        self.buffers.iter().position(|b| b.id == bid)
    }

    /// Open a file asynchronously. If a buffer for this path already exists,
    /// switch to it instead of creating a duplicate.
    pub async fn open_file(&mut self, path: &str) -> Result<()> {
        let path_buf = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));

        // Reuse existing buffer for this path.
        if let Some(existing) = self.buffers.iter().find(|b| {
            b.path.as_ref().map_or(false, |p| {
                std::fs::canonicalize(p).unwrap_or_else(|_| p.clone()) == path_buf
            })
        }) {
            let buf_id = existing.id;
            self.pane_layout.active_pane_mut().switch_buffer(buf_id);
            return Ok(());
        }

        let path_clone = path_buf.clone();
        match tokio::task::spawn_blocking(move || std::fs::read_to_string(&path_clone))
            .await?
        {
            Ok(text) => {
                let buf = Buffer::from_text(&text, path_buf);
                let buf_id = buf.id;
                self.request_highlight(&buf);
                self.buffers.push(buf);
                self.pane_layout.active_pane_mut().switch_buffer(buf_id);
            }
            Err(_) => {
                let buf = Buffer::new_for_path(path_buf);
                let buf_id = buf.id;
                self.buffers.push(buf);
                self.pane_layout.active_pane_mut().switch_buffer(buf_id);
                self.status_message = format!("New file: {}", path);
            }
        }
        Ok(())
    }

    pub fn new_scratch_buffer(&mut self) {
        let buf = Buffer::new_scratch();
        let buf_id = buf.id;
        self.buffers.push(buf);
        self.pane_layout.active_pane_mut().switch_buffer(buf_id);
    }

    fn request_highlight(&self, buf: &Buffer) {
        self.highlight_engine.request_highlight(
            buf.id,
            buf.text_snapshot(),
            buf.path.clone(),
        );
    }

    async fn open_file_browser(&mut self, dir: PathBuf) -> Result<()> {
        let mut fb = FileBrowser::new(dir.clone());

        let entries =
            tokio::task::spawn_blocking(move || scan_directory(&dir)).await?;
        fb.set_entries(entries);

        let pane_id = self.pane_layout.active_id;
        self.file_browsers.insert(pane_id, fb);
        Ok(())
    }

    async fn scan_current_browser_dir(&mut self) -> Result<()> {
        let pane_id = self.pane_layout.active_id;
        if let Some(ref fb) = self.file_browsers.get(&pane_id) {
            let dir = fb.current_dir.clone();
            let entries =
                tokio::task::spawn_blocking(move || scan_directory(&dir)).await?;
            if let Some(ref mut fb) = self.file_browsers.get_mut(&pane_id) {
                fb.set_entries(entries);
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut event_stream = crossterm::event::EventStream::new();

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
                let file_browsers = &self.file_browsers;

                let palette = &self.palette;
                let find_file = &self.find_file;
                self.terminal.draw(|frame| {
                    self.renderer.render(
                        frame, buffers, pane_layout, mode, cmd_buf, status, pending,
                        &pending_hints, hl_cache, file_browsers, palette, find_file,
                    );
                })?;
            }

            // Cursor style per mode.
            if self.file_browsers.contains_key(&self.pane_layout.active_id) {
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
                                if self.find_file.is_some() {
                                    self.handle_find_file_key(key).await?;
                                    continue;
                                }
                                if self.palette.is_some() {
                                    self.handle_palette_key(key).await?;
                                    continue;
                                }

                                let active_id = self.pane_layout.active_id;
                                let on_browser_pane = self.file_browsers.contains_key(&active_id);
                                let in_command_mode = self.input.mode == Mode::Command;
                                let browser_navigate = on_browser_pane
                                    && !in_command_mode
                                    && self.file_browsers.get(&active_id)
                                        .map_or(false, |fb| fb.input_mode == crate::file_browser::BrowserInputMode::Navigate);

                                if browser_navigate {
                                    self.handle_file_browser_action(key).await?;
                                } else if on_browser_pane && !in_command_mode {
                                    self.quit_pending = false;
                                    // Filter or NewFile text input mode — raw keys
                                    self.handle_file_browser_raw_key(key).await?;
                                } else {
                                    let action = self.input.handle_key(key);
                                    self.execute_action(action).await?;
                                }
                            }
                            Event::Mouse(mouse) => {
                                if let MouseEventKind::Down(crossterm::event::MouseButton::Left) = mouse.kind {
                                    self.handle_mouse_click(mouse.column, mouse.row).await?;
                                }
                            }
                            Event::Resize(_, _) => {}
                            _ => {}
                        }
                    }
                }
                Some(result) = self.highlight_rx.recv() => {
                    self.highlight_cache.insert(result.buffer_id, result.highlights);
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

        let old_state = self.file_browsers.get(&pane_id)
            .map(|fb| (fb.current_dir.clone(), fb.selected, fb.scroll_offset));

        let result = if let Some(ref mut fb) = self.file_browsers.get_mut(&pane_id) {
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

        let old_state = self.file_browsers.get(&pane_id)
            .map(|fb| (fb.current_dir.clone(), fb.selected, fb.scroll_offset));

        let result = if let Some(ref mut fb) = self.file_browsers.get_mut(&pane_id) {
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
                self.file_browsers.remove(&pane_id);
                let path_str = path.to_string_lossy().to_string();
                self.open_file(&path_str).await?;
            }
            FileBrowserResult::CreateFile(path) => {
                if let Some((dir, sel, scroll)) = old_state {
                    self.push_browser_jump(dir, sel, scroll);
                }
                self.file_browsers.remove(&pane_id);
                let path_str = path.to_string_lossy().to_string();
                self.open_file(&path_str).await?;
                self.input.mode = Mode::Insert;
            }
            FileBrowserResult::Close => {
                if !self.pane_layout.is_single() {
                    self.pane_layout.close_active();
                    self.file_browsers.remove(&pane_id);
                } else {
                    let pane_has_buffer = self.pane_layout.active_pane().buffer_id.is_some();
                    if pane_has_buffer || !self.buffers.is_empty() {
                        self.file_browsers.remove(&pane_id);
                    }
                }
            }
            FileBrowserResult::NeedsScan => {
                if let Some((dir, sel, scroll)) = old_state {
                    self.push_browser_jump(dir, sel, scroll);
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
        // Clear quit_pending on any action that isn't part of the :q flow.
        // EnterCommandMode (`:`), ExecuteCommand (the enter after `:q`),
        // and the quit actions themselves must NOT clear it.
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
        let viewport_height = self.active_viewport_height();
        let mut needs_rehighlight = false;

        match action {
            // Movement
            Action::MoveUp => self.with_buffer(|b| b.move_up()),
            Action::MoveDown => self.with_buffer(|b| b.move_down()),
            Action::MoveLeft => self.with_buffer(|b| b.move_left()),
            Action::MoveRight => self.with_buffer(|b| b.move_right()),
            Action::MoveWordForward => self.with_buffer(|b| b.move_word_forward()),
            Action::MoveWordBackward => self.with_buffer(|b| b.move_word_backward()),
            Action::MoveToLineStart => self.with_buffer(|b| b.move_to_line_start()),
            Action::MoveToLineEnd => self.with_buffer(|b| b.move_to_line_end()),
            Action::MoveToFirstLine => {
                self.with_buffer(|b| b.move_to_first_line());
            }
            Action::MoveToLastLine => {
                self.with_buffer(|b| b.move_to_last_line());
            }
            Action::PageUp => {
                self.with_buffer(|b| b.page_up(viewport_height));
            }
            Action::PageDown => {
                self.with_buffer(|b| b.page_down(viewport_height));
            }
            Action::HalfPageUp => {
                self.with_buffer(|b| b.half_page_up(viewport_height));
            }
            Action::HalfPageDown => {
                self.with_buffer(|b| b.half_page_down(viewport_height));
            }
            Action::ScrollUp => self.with_buffer(|b| b.scroll_up(viewport_height)),
            Action::ScrollDown => self.with_buffer(|b| b.scroll_down(viewport_height)),
            Action::GotoLine(line) => {
                self.with_buffer(|b| b.goto_line(line));
            }

            // Editing
            Action::InsertChar(c) => {
                self.with_buffer(|b| b.insert_char(c));
                needs_rehighlight = true;
            }
            Action::InsertNewline => {
                self.with_buffer(|b| b.insert_newline());
                needs_rehighlight = true;
            }
            Action::DeleteCharBackward => {
                self.with_buffer(|b| b.delete_char_backward());
                needs_rehighlight = true;
            }
            Action::DeleteCharForward => {
                self.with_buffer(|b| b.delete_char_forward());
                needs_rehighlight = true;
            }
            Action::DeleteLine => {
                self.with_buffer(|b| b.delete_line());
                needs_rehighlight = true;
            }
            Action::Undo => {
                self.with_buffer(|b| b.undo());
                needs_rehighlight = true;
            }
            Action::Redo => {
                self.with_buffer(|b| b.redo());
                needs_rehighlight = true;
            }

            // Clipboard
            Action::YankLine => {
                if let Some(buf) = self.active_buffer() {
                    let line_text = buf.current_line_text();
                    if let Some(ref mut cb) = self.clipboard {
                        if let Err(e) = cb.set_text(&line_text) {
                            self.status_message = format!("Clipboard error: {}", e);
                        } else {
                            self.status_message = "Yanked 1 line".into();
                        }
                    } else {
                        self.status_message = "Clipboard not available".into();
                    }
                }
            }
            Action::Paste => {
                let text = self.clipboard.as_mut()
                    .and_then(|cb| cb.get_text().ok());
                if let Some(text) = text {
                    self.with_buffer(|b| b.paste_after(&text));
                    needs_rehighlight = true;
                } else {
                    self.status_message = "Clipboard empty".into();
                }
            }
            Action::PasteBefore => {
                let text = self.clipboard.as_mut()
                    .and_then(|cb| cb.get_text().ok());
                if let Some(text) = text {
                    self.with_buffer(|b| b.paste_before(&text));
                    needs_rehighlight = true;
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
                self.with_buffer(|b| b.insert_line_below());
                self.input.mode = Mode::Insert;
                needs_rehighlight = true;
            }
            Action::InsertLineAbove => {
                self.with_buffer(|b| b.insert_line_above());
                self.input.mode = Mode::Insert;
                needs_rehighlight = true;
            }

            // Mode changes
            Action::EnterInsertMode => {
                self.input.mode = Mode::Insert;
            }
            Action::EnterInsertModeAppend => {
                self.with_buffer(|b| b.move_right());
                self.input.mode = Mode::Insert;
            }
            Action::EnterInsertModeLineEnd => {
                self.with_buffer(|b| b.move_to_line_end());
                self.input.mode = Mode::Insert;
            }
            Action::EnterInsertModeLineStart => {
                self.with_buffer(|b| b.move_to_line_start());
                self.input.mode = Mode::Insert;
            }
            Action::EnterNormalMode => {
                self.input.mode = Mode::Normal;
            }
            Action::EnterVisualMode => {
                self.input.mode = Mode::Visual;
            }
            Action::EnterCommandMode => {
                self.input.mode = Mode::Command;
                self.input.command_buffer.clear();
            }

            // Buffer management
            Action::NextBuffer => {
                if !self.buffers.is_empty() {
                    self.push_jump();
                    let cur_idx = self.active_buffer_idx().unwrap_or(0);
                    let next_idx = (cur_idx + 1) % self.buffers.len();
                    let next_id = self.buffers[next_idx].id;
                    self.pane_layout.active_pane_mut().switch_buffer(next_id);
                }
            }
            Action::PrevBuffer => {
                if !self.buffers.is_empty() {
                    self.push_jump();
                    let cur_idx = self.active_buffer_idx().unwrap_or(0);
                    let prev_idx = if cur_idx == 0 {
                        self.buffers.len() - 1
                    } else {
                        cur_idx - 1
                    };
                    let prev_id = self.buffers[prev_idx].id;
                    self.pane_layout.active_pane_mut().switch_buffer(prev_id);
                }
            }
            Action::CloseBuffer => {
                self.close_current_buffer(false);
            }

            // Window management
            Action::SplitVertical => {
                let old_id = self.pane_layout.active_id;
                let new_id = self.pane_layout.split(SplitDirection::Vertical);
                if let Some(fb) = self.file_browsers.get(&old_id).cloned() {
                    self.file_browsers.insert(new_id, fb);
                }
            }
            Action::SplitHorizontal => {
                let old_id = self.pane_layout.active_id;
                let new_id = self.pane_layout.split(SplitDirection::Horizontal);
                if let Some(fb) = self.file_browsers.get(&old_id).cloned() {
                    self.file_browsers.insert(new_id, fb);
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
                self.pane_layout.close_active();
                self.file_browsers.remove(&old_id);
            }

            // File browser
            Action::OpenFileBrowser => {
                if self.find_file.is_none() {
                    let pane_id = self.pane_layout.active_id;
                    let dir = if let Some(fb) = self.file_browsers.get(&pane_id) {
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
                    self.find_file = Some(FindFileState::new(dir));
                    execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
                }
            }
            Action::OpenFileBrowserHome => {
                if self.find_file.is_none() {
                    let dir = dirs::home_dir().unwrap_or_else(|| {
                        std::env::current_dir().unwrap_or_default()
                    });
                    self.find_file = Some(FindFileState::new(dir));
                    execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
                }
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

            Action::Noop => {}
        }

        // Re-highlight on edits.
        if needs_rehighlight {
            if let Some(buf) = self.active_buffer() {
                let buf_id = buf.id;
                let text = buf.text_snapshot();
                let path = buf.path.clone();
                self.highlight_engine.request_highlight(buf_id, text, path);
            }
        }

        // Scroll to keep cursor visible (on the pane directly).
        {
            let pane = self.pane_layout.active_pane_mut();
            if pane.cursor.line < pane.scroll_offset {
                pane.scroll_offset = pane.cursor.line;
            }
            if pane.cursor.line >= pane.scroll_offset + viewport_height {
                pane.scroll_offset = pane.cursor.line - viewport_height + 1;
            }
        }

        Ok(())
    }

    async fn save_current_buffer(&mut self) -> Result<()> {
        let buf_idx = match self.active_buffer_idx() {
            Some(idx) => idx,
            None => return Ok(()),
        };
        if let Some(buf) = self.buffers.get_mut(buf_idx) {
            if let Some(ref path) = buf.path {
                let text = buf.text_snapshot();
                let path = path.clone();
                let name = buf.name.clone();
                match tokio::task::spawn_blocking(move || std::fs::write(&path, text))
                    .await?
                {
                    Ok(()) => {
                        buf.modified = false;
                        self.status_message = format!("Saved: {}", name);
                    }
                    Err(e) => {
                        self.status_message = format!("Error saving: {}", e);
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
        let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
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
                if !self.buffers.is_empty() {
                    let cur_idx = self.active_buffer_idx().unwrap_or(0);
                    let next_idx = (cur_idx + 1) % self.buffers.len();
                    let next_id = self.buffers[next_idx].id;
                    self.pane_layout.active_pane_mut().switch_buffer(next_id);
                }
            }
            Some("bp") | Some("bprev") => {
                if !self.buffers.is_empty() {
                    let cur_idx = self.active_buffer_idx().unwrap_or(0);
                    let prev_idx = if cur_idx == 0 {
                        self.buffers.len() - 1
                    } else {
                        cur_idx - 1
                    };
                    let prev_id = self.buffers[prev_idx].id;
                    self.pane_layout.active_pane_mut().switch_buffer(prev_id);
                }
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
            self.pane_layout.close_active();
            self.file_browsers.remove(&pane_id);
        } else if force || self.confirm_double_quit() {
            self.should_quit = true;
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

        let active_id = self.pane_layout.active_id;

        // For the active pane, try to restore from jump history
        if self.pane_layout.active_pane().buffer_id == Some(removed_id) {
            let mut restored = false;
            // Search jump_back for a valid destination (skip stale entries)
            while let Some(pos) = self.jump_back.pop() {
                match &pos {
                    JumpPosition::Buffer { buffer_id, .. } => {
                        if *buffer_id != removed_id
                            && self.buffers.iter().any(|b| b.id == *buffer_id)
                        {
                            self.restore_jump(pos);
                            restored = true;
                            break;
                        }
                        // else: stale entry, continue searching
                    }
                    JumpPosition::Browser { .. } => {
                        self.restore_jump(pos);
                        restored = true;
                        break;
                    }
                }
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
            if pane.id != active_id && pane.buffer_id == Some(removed_id) {
                if !self.buffers.is_empty() {
                    let new_buf_id =
                        self.buffers[self.buffers.len().saturating_sub(1)].id;
                    pane.switch_buffer(new_buf_id);
                } else {
                    pane.buffer_id = None;
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

        self.palette = Some(PaletteState::new(items));
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
                self.palette = None;
            }
            KeyCode::Enter => {
                if let Some(ref palette) = self.palette {
                    if let Some(action) = palette.selected_action() {
                        self.palette = None;
                        self.execute_action(action).await?;
                    } else {
                        self.palette = None;
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') if key.code == KeyCode::Up || ctrl => {
                if let Some(ref mut palette) = self.palette {
                    palette.selected = palette.selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') if key.code == KeyCode::Down || ctrl => {
                if let Some(ref mut palette) = self.palette {
                    if palette.selected + 1 < palette.filtered.len() {
                        palette.selected += 1;
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut palette) = self.palette {
                    palette.query.pop();
                    palette.refilter();
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut palette) = self.palette {
                    palette.query.push(c);
                    palette.refilter();
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
                if let Some(mut ff) = self.find_file.take() {
                    if ff.is_dot_selected() {
                        // "." → open tree browser at current directory.
                        let dir = ff.dir.clone();
                        self.push_jump();
                        self.open_file_browser(dir).await?;
                    } else if let Some(entry) = ff.selected_entry().cloned() {
                        if entry.is_dir {
                            ff.enter_dir(&entry.path);
                            self.find_file = Some(ff);
                        } else {
                            // Close any file browser on this pane before opening the file.
                            let pane_id = self.pane_layout.active_id;
                            self.file_browsers.remove(&pane_id);
                            self.push_jump();
                            let path_str = entry.path.to_string_lossy().to_string();
                            self.open_file(&path_str).await?;
                        }
                    } else if !ff.input.is_empty() {
                        let new_path = ff.dir.join(&ff.input);
                        if let Some(parent) = new_path.parent() {
                            std::fs::create_dir_all(parent).ok();
                        }
                        let pane_id = self.pane_layout.active_id;
                        self.file_browsers.remove(&pane_id);
                        self.push_jump();
                        let path_str = new_path.to_string_lossy().to_string();
                        self.open_file(&path_str).await?;
                        self.input.mode = Mode::Insert;
                    }
                }
            }
            KeyCode::Tab => {
                // Tab completes to the selected entry.
                if let Some(ref mut ff) = self.find_file {
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
                if let Some(ref mut ff) = self.find_file {
                    if ff.input.is_empty() {
                        // Go up a directory.
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
                if let Some(ref mut ff) = self.find_file {
                    ff.selected = ff.selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') if key.code == KeyCode::Down || ctrl => {
                if let Some(ref mut ff) = self.find_file {
                    if ff.selected + 1 < ff.filtered.len() {
                        ff.selected += 1;
                    }
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut ff) = self.find_file {
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
        if self.find_file.is_none() {
            execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
        }
        Ok(())
    }

    fn close_find_file(&mut self) -> Result<()> {
        self.find_file = None;
        execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
        Ok(())
    }

    async fn handle_mouse_click(&mut self, col: u16, row: u16) -> Result<()> {
        // Only handle clicks on find-file overlay for now.
        if let Some(ref mut ff) = self.find_file {
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
                    // Simulate Enter on the clicked entry.
                    let ff = self.find_file.take().unwrap();
                    if ff.is_dot_selected() {
                        let dir = ff.dir.clone();
                        self.push_jump();
                        self.open_file_browser(dir).await?;
                    } else if let Some(entry) = ff.selected_entry().cloned() {
                        if entry.is_dir {
                            let mut ff = ff;
                            ff.enter_dir(&entry.path);
                            self.find_file = Some(ff);
                        } else {
                            let pane_id = self.pane_layout.active_id;
                            self.file_browsers.remove(&pane_id);
                            self.push_jump();
                            let path_str = entry.path.to_string_lossy().to_string();
                            self.open_file(&path_str).await?;
                        }
                    }
                }
            }
        }
        if self.find_file.is_none() {
            execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
        }
        Ok(())
    }

    fn with_buffer<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Buffer),
    {
        let Self { buffers, pane_layout, .. } = self;
        let pane = pane_layout.active_pane_mut();
        let buf_id = match pane.buffer_id {
            Some(id) => id,
            None => return,
        };
        let buf = match buffers.iter_mut().find(|b| b.id == buf_id) {
            Some(b) => b,
            None => return,
        };
        // Sync pane cursor → buffer
        buf.cursor = pane.cursor;
        buf.scroll_offset = pane.scroll_offset;
        f(buf);
        // Sync buffer cursor → pane
        pane.cursor = buf.cursor;
        pane.scroll_offset = buf.scroll_offset;
    }

    /// Capture the current state as a jump position.
    fn current_jump_position(&self) -> Option<JumpPosition> {
        let pane_id = self.pane_layout.active_id;
        if let Some(fb) = self.file_browsers.get(&pane_id) {
            Some(JumpPosition::Browser {
                dir: fb.current_dir.clone(),
                selected: fb.selected,
                scroll_offset: fb.scroll_offset,
            })
        } else {
            let pane = self.pane_layout.active_pane();
            pane.buffer_id.map(|buf_id| JumpPosition::Buffer {
                buffer_id: buf_id,
                line: pane.cursor.line,
                col: pane.cursor.col,
            })
        }
    }

    /// Push a specific browser directory as a jump position.
    fn push_browser_jump(&mut self, dir: PathBuf, selected: usize, scroll_offset: usize) {
        self.jump_back.push(JumpPosition::Browser { dir, selected, scroll_offset });
        self.jump_forward.clear();
        if self.jump_back.len() > 100 {
            self.jump_back.remove(0);
        }
    }

    /// Record the current position in the jump-back stack.
    fn push_jump(&mut self) {
        if let Some(pos) = self.current_jump_position() {
            self.jump_back.push(pos);
            self.jump_forward.clear();
            if self.jump_back.len() > 100 {
                self.jump_back.remove(0);
            }
        }
    }

    fn jump_back(&mut self) {
        if let Some(pos) = self.current_jump_position() {
            self.jump_forward.push(pos);
        }
        if let Some(pos) = self.jump_back.pop() {
            self.restore_jump(pos);
        } else {
            self.status_message = "No older jump position".into();
        }
    }

    fn jump_forward(&mut self) {
        if let Some(pos) = self.current_jump_position() {
            self.jump_back.push(pos);
        }
        if let Some(pos) = self.jump_forward.pop() {
            self.restore_jump(pos);
        } else {
            self.status_message = "No newer jump position".into();
        }
    }

    fn restore_jump(&mut self, pos: JumpPosition) {
        let pane_id = self.pane_layout.active_id;
        match pos {
            JumpPosition::Buffer { buffer_id, line, col } => {
                // Close any file browser on this pane
                self.file_browsers.remove(&pane_id);
                if self.buffers.iter().any(|b| b.id == buffer_id) {
                    let pane = self.pane_layout.active_pane_mut();
                    pane.switch_buffer(buffer_id);
                    pane.cursor.line = line;
                    pane.cursor.col = col;
                    if let Some(buf) = self.buffers.iter().find(|b| b.id == buffer_id) {
                        let line_count = buf.line_count();
                        if pane.cursor.line >= line_count {
                            pane.cursor.line = line_count.saturating_sub(1);
                        }
                    }
                } else {
                    self.status_message = "Buffer no longer exists".into();
                }
            }
            JumpPosition::Browser { dir, selected, scroll_offset } => {
                // Open file browser at the saved directory with restored cursor
                let mut fb = crate::file_browser::FileBrowser::new(dir.clone());
                let entries = crate::file_browser::scan_directory(&dir);
                fb.set_entries(entries);
                // Restore cursor position, clamped to entry count.
                let count = fb.visible_count();
                fb.selected = if selected < count { selected } else { count.saturating_sub(1) };
                fb.scroll_offset = scroll_offset;
                self.file_browsers.insert(pane_id, fb);
            }
        }
    }

    fn cleanup(&mut self) {
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
