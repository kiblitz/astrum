use ropey::Rope;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_BUFFER_ID: AtomicUsize = AtomicUsize::new(1);

fn next_id() -> usize {
    NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Self { line: 0, col: 0 }
    }
}

#[derive(Debug)]
pub struct Buffer {
    pub id: usize,
    pub rope: Rope,
    pub cursor: Cursor,
    pub path: Option<PathBuf>,
    pub modified: bool,
    pub scroll_offset: usize,
    pub name: String,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
}

#[derive(Debug, Clone)]
struct UndoEntry {
    rope_snapshot: Rope,
    cursor: Cursor,
}

impl Buffer {
    pub fn new_scratch() -> Self {
        Self {
            id: next_id(),
            rope: Rope::new(),
            cursor: Cursor::default(),
            path: None,
            modified: false,
            scroll_offset: 0,
            name: "[scratch]".to_string(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Create a buffer from already-loaded text (file reading is done externally).
    pub fn from_text(text: &str, path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        Self {
            id: next_id(),
            rope: Rope::from_str(text),
            cursor: Cursor::default(),
            path: Some(path),
            modified: false,
            scroll_offset: 0,
            name,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn new_for_path(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        Self {
            id: next_id(),
            rope: Rope::new(),
            cursor: Cursor::default(),
            path: Some(path),
            modified: false,
            scroll_offset: 0,
            name,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn text_snapshot(&self) -> String {
        self.rope.to_string()
    }

    fn save_undo(&mut self) {
        self.undo_stack.push(UndoEntry {
            rope_snapshot: self.rope.clone(),
            cursor: self.cursor,
        });
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(entry) = self.undo_stack.pop() {
            self.redo_stack.push(UndoEntry {
                rope_snapshot: self.rope.clone(),
                cursor: self.cursor,
            });
            self.rope = entry.rope_snapshot;
            self.cursor = entry.cursor;
            self.modified = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(entry) = self.redo_stack.pop() {
            self.undo_stack.push(UndoEntry {
                rope_snapshot: self.rope.clone(),
                cursor: self.cursor,
            });
            self.rope = entry.rope_snapshot;
            self.cursor = entry.cursor;
            self.modified = true;
        }
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines().max(1)
    }

    fn clamp_cursor(&mut self) {
        let line_count = self.line_count();
        if self.cursor.line >= line_count {
            self.cursor.line = line_count.saturating_sub(1);
        }
        let line_len = self.current_line_len();
        if self.cursor.col > line_len {
            self.cursor.col = line_len;
        }
    }

    pub fn current_line_len(&self) -> usize {
        if self.cursor.line >= self.rope.len_lines() {
            return 0;
        }
        let line = self.rope.line(self.cursor.line);
        let len = line.len_chars();
        if len > 0 {
            let last = line.char(len - 1);
            if last == '\n' || last == '\r' {
                if len >= 2 && line.char(len - 2) == '\r' {
                    len - 2
                } else {
                    len - 1
                }
            } else {
                len
            }
        } else {
            0
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.save_undo();
        let idx = self.cursor_char_idx();
        self.rope.insert_char(idx, ch);
        if ch == '\n' {
            self.cursor.line += 1;
            self.cursor.col = 0;
        } else {
            self.cursor.col += 1;
        }
        self.modified = true;
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn delete_char_backward(&mut self) {
        if self.cursor.col == 0 && self.cursor.line == 0 {
            return;
        }
        self.save_undo();
        let idx = self.cursor_char_idx();
        if idx == 0 {
            return;
        }
        self.rope.remove(idx - 1..idx);
        if self.cursor.col == 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.current_line_len();
        } else {
            self.cursor.col -= 1;
        }
        self.modified = true;
    }

    pub fn delete_char_forward(&mut self) {
        let idx = self.cursor_char_idx();
        if idx >= self.rope.len_chars() {
            return;
        }
        self.save_undo();
        self.rope.remove(idx..idx + 1);
        self.clamp_cursor();
        self.modified = true;
    }

    pub fn delete_line(&mut self) {
        if self.rope.len_lines() == 0 {
            return;
        }
        self.save_undo();
        let line_start = self.rope.line_to_char(self.cursor.line);
        let line_end = if self.cursor.line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(self.cursor.line + 1)
        } else {
            self.rope.len_chars()
        };
        if line_start < line_end {
            self.rope.remove(line_start..line_end);
        }
        self.clamp_cursor();
        self.modified = true;
    }

    pub fn move_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.clamp_cursor();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor.line + 1 < self.line_count() {
            self.cursor.line += 1;
            self.clamp_cursor();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.current_line_len();
        }
    }

    pub fn move_right(&mut self) {
        let line_len = self.current_line_len();
        if self.cursor.col < line_len {
            self.cursor.col += 1;
        } else if self.cursor.line + 1 < self.line_count() {
            self.cursor.line += 1;
            self.cursor.col = 0;
        }
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor.col = 0;
    }

    pub fn move_to_line_end(&mut self) {
        self.cursor.col = self.current_line_len();
    }

    pub fn move_to_first_line(&mut self) {
        self.cursor.line = 0;
        self.clamp_cursor();
    }

    pub fn move_to_last_line(&mut self) {
        self.cursor.line = self.line_count().saturating_sub(1);
        self.clamp_cursor();
    }

    pub fn move_word_forward(&mut self) {
        let line_count = self.line_count();
        let mut line = self.cursor.line;
        let mut col = self.cursor.col;

        loop {
            if line >= line_count {
                break;
            }
            let rope_line = self.rope.line(line);
            let line_len = line_len_no_newline(&rope_line);

            while col < line_len {
                if rope_line.char(col).is_alphanumeric() || rope_line.char(col) == '_' {
                    col += 1;
                } else {
                    break;
                }
            }
            while col < line_len {
                if !rope_line.char(col).is_alphanumeric() && rope_line.char(col) != '_' {
                    col += 1;
                } else {
                    break;
                }
            }

            if col < line_len {
                self.cursor.line = line;
                self.cursor.col = col;
                return;
            }

            line += 1;
            col = 0;
        }

        self.cursor.line = line_count.saturating_sub(1);
        self.cursor.col = self.current_line_len();
    }

    pub fn move_word_backward(&mut self) {
        let mut line = self.cursor.line;
        let mut col = self.cursor.col;

        if col == 0 {
            if line == 0 {
                return;
            }
            line -= 1;
            self.cursor.line = line;
            col = line_len_no_newline(&self.rope.line(line));
        }

        let rope_line = self.rope.line(line);

        while col > 0 {
            if !rope_line.char(col - 1).is_alphanumeric() && rope_line.char(col - 1) != '_' {
                col -= 1;
            } else {
                break;
            }
        }
        while col > 0 {
            if rope_line.char(col - 1).is_alphanumeric() || rope_line.char(col - 1) == '_' {
                col -= 1;
            } else {
                break;
            }
        }

        self.cursor.line = line;
        self.cursor.col = col;
    }

    pub fn insert_line_below(&mut self) {
        self.save_undo();
        let line_end = if self.cursor.line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(self.cursor.line + 1)
        } else {
            let len = self.rope.len_chars();
            if len > 0 && self.rope.char(len - 1) != '\n' {
                self.rope.insert_char(len, '\n');
            }
            self.rope.len_chars()
        };
        self.rope.insert_char(line_end, '\n');
        self.cursor.line += 1;
        self.cursor.col = 0;
        self.modified = true;
    }

    pub fn insert_line_above(&mut self) {
        self.save_undo();
        let line_start = self.rope.line_to_char(self.cursor.line);
        self.rope.insert_char(line_start, '\n');
        self.cursor.col = 0;
        self.modified = true;
    }

    pub fn page_up(&mut self, page_size: usize) {
        let offset = self.cursor.line.saturating_sub(self.scroll_offset);
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
        self.cursor.line = self.scroll_offset + offset;
        self.clamp_cursor();
    }

    pub fn page_down(&mut self, page_size: usize) {
        let max_line = self.line_count().saturating_sub(1);
        let offset = self.cursor.line.saturating_sub(self.scroll_offset);
        self.scroll_offset = (self.scroll_offset + page_size).min(max_line);
        self.cursor.line = (self.scroll_offset + offset).min(max_line);
        self.clamp_cursor();
    }

    pub fn half_page_up(&mut self, page_size: usize) {
        let half = page_size / 2;
        let offset = self.cursor.line.saturating_sub(self.scroll_offset);
        self.scroll_offset = self.scroll_offset.saturating_sub(half);
        self.cursor.line = self.scroll_offset + offset;
        self.clamp_cursor();
    }

    pub fn half_page_down(&mut self, page_size: usize) {
        let half = page_size / 2;
        let max_line = self.line_count().saturating_sub(1);
        let offset = self.cursor.line.saturating_sub(self.scroll_offset);
        self.scroll_offset = (self.scroll_offset + half).min(max_line);
        self.cursor.line = (self.scroll_offset + offset).min(max_line);
        self.clamp_cursor();
    }

    pub fn ensure_visible(&mut self, viewport_height: usize) {
        if self.cursor.line < self.scroll_offset {
            self.scroll_offset = self.cursor.line;
        }
        if self.cursor.line >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor.line - viewport_height + 1;
        }
    }

    /// Scroll viewport down by one line (Ctrl-e). Cursor stays put unless
    /// it would go above the viewport.
    pub fn scroll_down(&mut self, _viewport_height: usize) {
        let max_offset = self.line_count().saturating_sub(1);
        if self.scroll_offset < max_offset {
            self.scroll_offset += 1;
            // Push cursor down if it's above the viewport.
            if self.cursor.line < self.scroll_offset {
                self.cursor.line = self.scroll_offset;
                self.clamp_cursor();
            }
        }
    }

    /// Scroll viewport up by one line (Ctrl-y). Cursor stays put unless
    /// it would go below the viewport.
    pub fn scroll_up(&mut self, viewport_height: usize) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
            // Push cursor up if it's below the viewport.
            let bottom = self.scroll_offset + viewport_height.saturating_sub(1);
            if self.cursor.line > bottom {
                self.cursor.line = bottom;
                self.clamp_cursor();
            }
        }
    }

    /// Return the full text of the current line (including newline).
    pub fn current_line_text(&self) -> String {
        if self.cursor.line >= self.rope.len_lines() {
            return String::new();
        }
        self.rope.line(self.cursor.line).to_string()
    }

    /// Paste text after the current line.
    pub fn paste_after(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.save_undo();
        // If the text ends with a newline, treat it as a line-paste (insert below).
        if text.ends_with('\n') || text.ends_with("\r\n") {
            let insert_pos = if self.cursor.line + 1 < self.rope.len_lines() {
                self.rope.line_to_char(self.cursor.line + 1)
            } else {
                let len = self.rope.len_chars();
                // Ensure there's a trailing newline before inserting.
                if len > 0 && self.rope.char(len - 1) != '\n' {
                    self.rope.insert_char(len, '\n');
                }
                self.rope.len_chars()
            };
            self.rope.insert(insert_pos, text);
            self.cursor.line += 1;
            self.cursor.col = 0;
        } else {
            // Inline paste after cursor.
            let idx = self.cursor_char_idx();
            let insert_at = (idx + 1).min(self.rope.len_chars());
            self.rope.insert(insert_at, text);
            self.cursor.col += text.len();
        }
        self.modified = true;
    }

    /// Paste text before the current line.
    pub fn paste_before(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.save_undo();
        if text.ends_with('\n') || text.ends_with("\r\n") {
            let insert_pos = self.rope.line_to_char(self.cursor.line);
            self.rope.insert(insert_pos, text);
            self.cursor.col = 0;
        } else {
            let idx = self.cursor_char_idx();
            self.rope.insert(idx, text);
        }
        self.modified = true;
    }

    fn cursor_char_idx(&self) -> usize {
        if self.cursor.line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(self.cursor.line);
        let line_len = self.current_line_len();
        line_start + self.cursor.col.min(line_len)
    }

    pub fn goto_line(&mut self, line: usize) {
        self.cursor.line = line.min(self.line_count().saturating_sub(1));
        self.clamp_cursor();
    }
}

fn line_len_no_newline(line: &ropey::RopeSlice<'_>) -> usize {
    let len = line.len_chars();
    if len > 0 {
        let last = line.char(len - 1);
        if last == '\n' || last == '\r' {
            if len >= 2 && line.char(len - 2) == '\r' {
                len - 2
            } else {
                len - 1
            }
        } else {
            len
        }
    } else {
        0
    }
}
