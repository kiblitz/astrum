use crate::action::Action;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

/// Result of handling a key in the file browser.
pub enum FileBrowserResult {
    /// User selected a file to open.
    OpenFile(PathBuf),
    /// User wants to create a new file at this path.
    CreateFile(PathBuf),
    /// Close the browser without doing anything.
    Close,
    /// Browser needs a directory scan (e.g. navigated to new dir).
    NeedsScan,
    /// Close browser and pass this key to the normal input handler.
    PassThrough,
    /// No action needed, just re-render.
    Noop,
}

#[derive(Clone)]
pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<DirEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub filter: String,
    pub filtered_indices: Vec<usize>,
    pub input_mode: BrowserInputMode,
    pub new_file_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserInputMode {
    Navigate,
    Filter,
    NewFile,
}

impl FileBrowser {
    pub fn new(start_dir: PathBuf) -> Self {
        Self {
            current_dir: start_dir,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            filter: String::new(),
            filtered_indices: Vec::new(),
            input_mode: BrowserInputMode::Navigate,
            new_file_name: String::new(),
        }
    }

    /// Populate entries from a directory listing (call after scan completes).
    pub fn set_entries(&mut self, mut entries: Vec<DirEntry>) {
        // Sort: directories first, then alphabetical
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.entries = entries;
        self.selected = 0;
        self.scroll_offset = 0;
        self.refilter();
    }

    fn refilter(&mut self) {
        if self.filter.is_empty() {
            self.filtered_indices = (0..self.entries.len()).collect();
        } else {
            let lower = self.filter.to_lowercase();
            self.filtered_indices = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.name.to_lowercase().contains(&lower))
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

    pub fn visible_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Get the currently selected entry (if any).
    pub fn selected_entry(&self) -> Option<&DirEntry> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.entries.get(idx))
    }

    pub fn ensure_visible(&mut self, viewport_height: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
        if self.selected >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.selected - viewport_height + 1;
        }
    }

    /// Handle a raw key event — dispatches to the appropriate input mode handler.
    /// Only used for Filter and NewFile modes (text input). Navigate mode uses handle_action.
    pub fn handle_key(&mut self, key: KeyEvent) -> FileBrowserResult {
        match self.input_mode {
            BrowserInputMode::Navigate => FileBrowserResult::Noop,
            BrowserInputMode::Filter => self.handle_filter(key),
            BrowserInputMode::NewFile => self.handle_new_file(key),
        }
    }

    /// Handle a resolved Action in navigate mode.
    pub fn handle_action(&mut self, action: &Action, count: usize) -> FileBrowserResult {
        match action {
            Action::BrowserClose => FileBrowserResult::Close,

            Action::MoveDown => {
                let target = (self.selected + count).min(self.visible_count().saturating_sub(1));
                self.selected = target;
                FileBrowserResult::Noop
            }
            Action::MoveUp => {
                self.selected = self.selected.saturating_sub(count);
                FileBrowserResult::Noop
            }
            Action::MoveToLastLine => {
                self.selected = self.visible_count().saturating_sub(1);
                FileBrowserResult::Noop
            }
            Action::MoveToFirstLine => {
                self.selected = 0;
                FileBrowserResult::Noop
            }
            Action::HalfPageDown => {
                let jump = 15;
                self.selected =
                    (self.selected + jump).min(self.visible_count().saturating_sub(1));
                FileBrowserResult::Noop
            }
            Action::HalfPageUp => {
                self.selected = self.selected.saturating_sub(15);
                FileBrowserResult::Noop
            }

            Action::BrowserOpen => {
                if let Some(entry) = self.selected_entry().cloned() {
                    if entry.is_dir {
                        self.current_dir = entry.path;
                        self.filter.clear();
                        FileBrowserResult::NeedsScan
                    } else {
                        FileBrowserResult::OpenFile(entry.path)
                    }
                } else {
                    FileBrowserResult::Noop
                }
            }

            Action::BrowserParentDir => {
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.filter.clear();
                    FileBrowserResult::NeedsScan
                } else {
                    FileBrowserResult::Noop
                }
            }

            Action::BrowserFilter => {
                self.input_mode = BrowserInputMode::Filter;
                self.filter.clear();
                self.refilter();
                FileBrowserResult::Noop
            }

            Action::BrowserNewFile => {
                self.input_mode = BrowserInputMode::NewFile;
                self.new_file_name.clear();
                FileBrowserResult::Noop
            }

            Action::BrowserHome => {
                if let Some(home) = dirs::home_dir() {
                    self.current_dir = home;
                    self.filter.clear();
                    FileBrowserResult::NeedsScan
                } else {
                    FileBrowserResult::Noop
                }
            }

            // Actions the browser doesn't handle — pass through to execute_action.
            _ => FileBrowserResult::PassThrough,
        }
    }

    fn handle_filter(&mut self, key: KeyEvent) -> FileBrowserResult {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = BrowserInputMode::Navigate;
                self.filter.clear();
                self.refilter();
                FileBrowserResult::Noop
            }
            KeyCode::Enter => {
                self.input_mode = BrowserInputMode::Navigate;
                FileBrowserResult::Noop
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.refilter();
                FileBrowserResult::Noop
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.refilter();
                FileBrowserResult::Noop
            }
            _ => FileBrowserResult::Noop,
        }
    }

    fn handle_new_file(&mut self, key: KeyEvent) -> FileBrowserResult {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = BrowserInputMode::Navigate;
                self.new_file_name.clear();
                FileBrowserResult::Noop
            }
            KeyCode::Enter => {
                self.input_mode = BrowserInputMode::Navigate;
                if self.new_file_name.is_empty() {
                    return FileBrowserResult::Noop;
                }
                let path = self.current_dir.join(&self.new_file_name);
                self.new_file_name.clear();
                FileBrowserResult::CreateFile(path)
            }
            KeyCode::Backspace => {
                self.new_file_name.pop();
                FileBrowserResult::Noop
            }
            KeyCode::Char(c) => {
                self.new_file_name.push(c);
                FileBrowserResult::Noop
            }
            _ => FileBrowserResult::Noop,
        }
    }
}

/// Scan a directory and return its entries. Meant to run on spawn_blocking.
pub fn scan_directory(dir: &Path) -> Vec<DirEntry> {
    let mut entries = Vec::new();
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return entries,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push(DirEntry {
            name,
            path,
            is_dir,
            size,
        });
    }
    entries
}

/// Format a file size for display.
pub fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{}B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1}K", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1}M", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
