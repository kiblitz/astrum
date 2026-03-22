use crate::syntax::EditInfo;
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
    /// Lazy BLAKE3 hash of rope content. Set to None on any mutation,
    /// recomputed on demand (swap timer tick, before save).
    content_hash: Option<[u8; 32]>,
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
            content_hash: None,
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
            content_hash: None,
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
            content_hash: None,
        }
    }

    pub fn text_snapshot(&self) -> String {
        self.rope.to_string()
    }

    /// Compute or return the cached BLAKE3 content hash.
    pub fn content_hash(&mut self) -> [u8; 32] {
        if let Some(h) = self.content_hash {
            return h;
        }
        let mut hasher = blake3::Hasher::new();
        for chunk in self.rope.chunks() {
            hasher.update(chunk.as_bytes());
        }
        let h = *hasher.finalize().as_bytes();
        self.content_hash = Some(h);
        h
    }

    /// Invalidate the cached content hash. Called by mutation methods.
    pub fn invalidate_hash(&mut self) {
        self.content_hash = None;
    }

    pub fn save_undo(&mut self) {
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
        self.invalidate_hash();
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
        self.invalidate_hash();
        }
    }

    /// Line count excluding ropey's phantom empty last line.
    /// Ropey counts "hello\n" as 2 lines; vim counts it as 1.
    /// This makes all cursor/range logic match vim semantics.
    pub fn line_count(&self) -> usize {
        let n = self.rope.len_lines();
        if n > 1 && self.rope.line(n - 1).len_chars() == 0 {
            n - 1
        } else {
            n.max(1)
        }
    }

    /// Compute the char index range for a span of lines (inclusive).
    pub fn linewise_range(&self, first_line: usize, last_line: usize) -> (usize, usize) {
        let start = self.rope.line_to_char(first_line);
        let end = if last_line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(last_line + 1)
        } else {
            self.rope.len_chars()
        };
        (start, end)
    }

    pub fn clamp_cursor(&mut self) {
        let line_count = self.line_count();
        if self.cursor.line >= line_count {
            self.cursor.line = line_count.saturating_sub(1);
        }
        let max_col = self.current_line_len().saturating_sub(1);
        if self.cursor.col > max_col {
            self.cursor.col = max_col;
        }
    }

    pub fn current_line_len(&self) -> usize {
        if self.cursor.line >= self.rope.len_lines() {
            return 0;
        }
        line_len_no_newline(&self.rope.line(self.cursor.line))
    }

    pub fn insert_char(&mut self, ch: char) -> Option<EditInfo> {
        self.save_undo();
        let idx = self.cursor_char_idx();
        let start_byte = self.rope.char_to_byte(idx);
        let start_pos = self.position_at_char(idx);
        self.rope.insert_char(idx, ch);
        let new_end_byte = start_byte + ch.len_utf8();
        let new_end_pos = self.position_at_char(idx + 1);
        if ch == '\n' {
            self.cursor.line += 1;
            self.cursor.col = 0;
        } else {
            self.cursor.col += 1;
        }
        self.modified = true;
        self.invalidate_hash();
        Some(EditInfo {
            start_byte,
            old_end_byte: start_byte,
            new_end_byte,
            start_position: start_pos,
            old_end_position: start_pos,
            new_end_position: new_end_pos,
        })
    }

    pub fn insert_newline(&mut self) -> Option<EditInfo> {
        self.insert_char('\n')
    }

    pub fn delete_char_backward(&mut self) -> Option<EditInfo> {
        if self.cursor.col == 0 && self.cursor.line == 0 {
            return None;
        }
        self.save_undo();
        let idx = self.cursor_char_idx();
        if idx == 0 {
            return None;
        }
        let start_byte = self.rope.char_to_byte(idx - 1);
        let old_end_byte = self.rope.char_to_byte(idx);
        let start_pos = self.position_at_char(idx - 1);
        let old_end_pos = self.position_at_char(idx);
        self.rope.remove(idx - 1..idx);
        if self.cursor.col == 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.current_line_len();
        } else {
            self.cursor.col -= 1;
        }
        self.modified = true;
        self.invalidate_hash();
        Some(EditInfo {
            start_byte,
            old_end_byte,
            new_end_byte: start_byte,
            start_position: start_pos,
            old_end_position: old_end_pos,
            new_end_position: start_pos,
        })
    }

    pub fn delete_word_backward(&mut self) -> Option<EditInfo> {
        if self.cursor.col == 0 && self.cursor.line == 0 {
            return None;
        }
        let end_idx = self.cursor_char_idx();
        if end_idx == 0 {
            return None;
        }

        // Save undo before moving so that undo restores the original cursor.
        self.save_undo();

        self.move_word_backward();
        let start_idx = self.cursor_char_idx();

        if start_idx == end_idx {
            // Nothing to delete — pop the undo we just saved.
            self.undo_stack.pop();
            return None;
        }
        let start_byte = self.rope.char_to_byte(start_idx);
        let old_end_byte = self.rope.char_to_byte(end_idx);
        let start_pos = self.position_at_char(start_idx);
        let old_end_pos = self.position_at_char(end_idx);
        self.rope.remove(start_idx..end_idx);
        // Cursor is already at the right position from move_word_backward.
        self.modified = true;
        self.invalidate_hash();
        Some(EditInfo {
            start_byte,
            old_end_byte,
            new_end_byte: start_byte,
            start_position: start_pos,
            old_end_position: old_end_pos,
            new_end_position: start_pos,
        })
    }

    pub fn delete_char_forward(&mut self) -> Option<EditInfo> {
        let idx = self.cursor_char_idx();
        if idx >= self.rope.len_chars() {
            return None;
        }
        self.save_undo();
        let start_byte = self.rope.char_to_byte(idx);
        let old_end_byte = self.rope.char_to_byte(idx + 1);
        let start_pos = self.position_at_char(idx);
        let old_end_pos = self.position_at_char(idx + 1);
        self.rope.remove(idx..idx + 1);
        self.clamp_cursor();
        self.modified = true;
        self.invalidate_hash();
        Some(EditInfo {
            start_byte,
            old_end_byte,
            new_end_byte: start_byte,
            start_position: start_pos,
            old_end_position: old_end_pos,
            new_end_position: start_pos,
        })
    }

    pub fn delete_line(&mut self) -> Option<EditInfo> {
        if self.rope.len_lines() == 0 {
            return None;
        }
        self.save_undo();
        let (line_start, line_end) = self.linewise_range(self.cursor.line, self.cursor.line);
        if line_start >= line_end {
            return None;
        }
        let start_byte = self.rope.char_to_byte(line_start);
        let old_end_byte = self.rope.char_to_byte(line_end);
        let start_pos = self.position_at_char(line_start);
        let old_end_pos = self.position_at_char(line_end);
        self.rope.remove(line_start..line_end);
        self.clamp_cursor();
        self.modified = true;
        self.invalidate_hash();
        Some(EditInfo {
            start_byte,
            old_end_byte,
            new_end_byte: start_byte,
            start_position: start_pos,
            old_end_position: old_end_pos,
            new_end_position: start_pos,
        })
    }

    /// Delete a range of characters by char index. Cursor is placed at `start`.
    pub fn delete_char_range(&mut self, start: usize, end: usize) -> Option<EditInfo> {
        if start >= end || start >= self.rope.len_chars() {
            return None;
        }
        let end = end.min(self.rope.len_chars());
        self.save_undo();
        let start_byte = self.rope.char_to_byte(start);
        let old_end_byte = self.rope.char_to_byte(end);
        let start_pos = self.position_at_char(start);
        let old_end_pos = self.position_at_char(end);
        self.rope.remove(start..end);
        // Place cursor at the start of the deleted range.
        self.cursor.line = self.rope.char_to_line(start.min(self.rope.len_chars().saturating_sub(1).max(0)));
        let line_start = self.rope.line_to_char(self.cursor.line);
        self.cursor.col = start.saturating_sub(line_start);
        self.clamp_cursor();
        self.modified = true;
        self.invalidate_hash();
        Some(EditInfo {
            start_byte,
            old_end_byte,
            new_end_byte: start_byte,
            start_position: start_pos,
            old_end_position: old_end_pos,
            new_end_position: start_pos,
        })
    }

    /// Extract text in a char range (for yanking).
    pub fn text_in_char_range(&self, start: usize, end: usize) -> String {
        let end = end.min(self.rope.len_chars());
        if start >= end {
            return String::new();
        }
        self.rope.slice(start..end).to_string()
    }

    /// Get char index from a (line, col) cursor position.
    pub fn char_idx_at(&self, line: usize, col: usize) -> usize {
        if line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(line);
        let line_len = line_len_no_newline(&self.rope.line(line));
        line_start + col.min(line_len)
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
        let max_col = self.current_line_len().saturating_sub(1);
        if self.cursor.col < max_col {
            self.cursor.col += 1;
        }
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor.col = 0;
    }

    pub fn move_to_line_end(&mut self) {
        self.cursor.col = self.current_line_len().saturating_sub(1);
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
        let start_line = self.cursor.line;
        let mut line = start_line;
        let mut col = self.cursor.col;

        // Try current line first, then at most the next line.
        for _ in 0..2 {
            if line >= line_count {
                break;
            }
            let rope_line = self.rope.line(line);
            let line_len = line_len_no_newline(&rope_line);

            // Skip current char class (word or punctuation).
            if col < line_len {
                let cls = char_class(rope_line.char(col));
                if cls != CharClass::Whitespace {
                    while col < line_len && char_class(rope_line.char(col)) == cls {
                        col += 1;
                    }
                }
            }
            // Skip whitespace.
            while col < line_len && char_class(rope_line.char(col)) == CharClass::Whitespace {
                col += 1;
            }

            if col < line_len {
                self.cursor.line = line;
                self.cursor.col = col;
                return;
            }

            line += 1;
            col = 0;
        }

        // Landed at start of line (at most 1 line ahead) or end of file.
        let target_line = (start_line + 1).min(line_count.saturating_sub(1));
        self.cursor.line = target_line;
        self.cursor.col = 0;
    }

    pub fn move_word_end(&mut self) {
        let line_count = self.line_count();
        let start_line = self.cursor.line;
        let mut line = start_line;
        let mut col = self.cursor.col + 1; // Move past current position.

        // Try current line first, then at most the next line.
        for _ in 0..2 {
            if line >= line_count {
                break;
            }
            let rope_line = self.rope.line(line);
            let line_len = line_len_no_newline(&rope_line);

            // Skip whitespace.
            while col < line_len && char_class(rope_line.char(col)) == CharClass::Whitespace {
                col += 1;
            }
            // Advance through current char class (word or punctuation) to end.
            if col < line_len {
                let cls = char_class(rope_line.char(col));
                while col + 1 < line_len && char_class(rope_line.char(col + 1)) == cls {
                    col += 1;
                }
                self.cursor.line = line;
                self.cursor.col = col;
                return;
            }

            line += 1;
            col = 0;
        }

        // End of file.
        let target_line = (start_line + 1).min(line_count.saturating_sub(1));
        self.cursor.line = target_line;
        let rope_line = self.rope.line(self.cursor.line);
        self.cursor.col = line_len_no_newline(&rope_line).saturating_sub(1);
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

        // Skip whitespace backward.
        while col > 0 && char_class(rope_line.char(col - 1)) == CharClass::Whitespace {
            col -= 1;
        }
        // Skip current char class (word or punctuation) backward.
        if col > 0 {
            let cls = char_class(rope_line.char(col - 1));
            while col > 0 && char_class(rope_line.char(col - 1)) == cls {
                col -= 1;
            }
        }

        self.cursor.line = line;
        self.cursor.col = col;
    }

    /// Move to start of next WORD (whitespace-delimited). Jumps at most 1 line.
    pub fn move_big_word_forward(&mut self) {
        let line_count = self.line_count();
        let start_line = self.cursor.line;
        let mut line = start_line;
        let mut col = self.cursor.col;

        for _ in 0..2 {
            if line >= line_count { break; }
            let rope_line = self.rope.line(line);
            let line_len = line_len_no_newline(&rope_line);

            // Skip non-whitespace.
            while col < line_len && !rope_line.char(col).is_whitespace() { col += 1; }
            // Skip whitespace.
            while col < line_len && rope_line.char(col).is_whitespace() { col += 1; }

            if col < line_len {
                self.cursor.line = line;
                self.cursor.col = col;
                return;
            }
            line += 1;
            col = 0;
        }

        let target_line = (start_line + 1).min(line_count.saturating_sub(1));
        self.cursor.line = target_line;
        self.cursor.col = 0;
    }

    /// Move to end of current/next WORD (whitespace-delimited). Jumps at most 1 line.
    pub fn move_big_word_end(&mut self) {
        let line_count = self.line_count();
        let start_line = self.cursor.line;
        let mut line = start_line;
        let mut col = self.cursor.col + 1;

        for _ in 0..2 {
            if line >= line_count { break; }
            let rope_line = self.rope.line(line);
            let line_len = line_len_no_newline(&rope_line);

            // Skip whitespace.
            while col < line_len && rope_line.char(col).is_whitespace() { col += 1; }
            // Advance to end of WORD.
            if col < line_len {
                while col + 1 < line_len && !rope_line.char(col + 1).is_whitespace() { col += 1; }
                self.cursor.line = line;
                self.cursor.col = col;
                return;
            }
            line += 1;
            col = 0;
        }

        let target_line = (start_line + 1).min(line_count.saturating_sub(1));
        self.cursor.line = target_line;
        let rope_line = self.rope.line(self.cursor.line);
        self.cursor.col = line_len_no_newline(&rope_line).saturating_sub(1);
    }

    /// Move to start of previous WORD (whitespace-delimited). Jumps at most 1 line.
    pub fn move_big_word_backward(&mut self) {
        let mut line = self.cursor.line;
        let mut col = self.cursor.col;

        if col == 0 {
            if line == 0 { return; }
            line -= 1;
            self.cursor.line = line;
            col = line_len_no_newline(&self.rope.line(line));
        }

        let rope_line = self.rope.line(line);

        // Skip whitespace backward.
        while col > 0 && rope_line.char(col - 1).is_whitespace() { col -= 1; }
        // Skip non-whitespace backward.
        while col > 0 && !rope_line.char(col - 1).is_whitespace() { col -= 1; }

        self.cursor.line = line;
        self.cursor.col = col;
    }

    pub fn insert_line_below(&mut self) -> Option<EditInfo> {
        self.save_undo();
        let line = self.rope.line(self.cursor.line);
        let line_chars = line.len_chars();
        let insert_at = self.rope.line_to_char(self.cursor.line) + line_chars;
        // If the current line has no trailing newline (last line of file),
        // insert two \n: one to terminate this line, one for the new line.
        // Otherwise ropey creates only a phantom empty line that line_count() hides.
        let has_nl = line_chars > 0 && line.char(line_chars - 1) == '\n';
        let text = if has_nl { "\n" } else { "\n\n" };
        let start_byte = self.rope.char_to_byte(insert_at);
        let start_pos = self.position_at_char(insert_at);
        self.rope.insert(insert_at, text);
        let new_end_byte = start_byte + text.len();
        let new_end_pos = self.position_at_char(insert_at + text.len());
        self.cursor.line += 1;
        self.cursor.col = 0;
        self.modified = true;
        self.invalidate_hash();
        Some(EditInfo {
            start_byte,
            old_end_byte: start_byte,
            new_end_byte,
            start_position: start_pos,
            old_end_position: start_pos,
            new_end_position: new_end_pos,
        })
    }

    pub fn insert_line_above(&mut self) -> Option<EditInfo> {
        self.save_undo();
        let line_start = self.rope.line_to_char(self.cursor.line);
        let start_byte = self.rope.char_to_byte(line_start);
        let start_pos = self.position_at_char(line_start);
        self.rope.insert_char(line_start, '\n');
        let new_end_byte = start_byte + 1;
        let new_end_pos = self.position_at_char(line_start + 1);
        self.cursor.col = 0;
        self.modified = true;
        self.invalidate_hash();
        Some(EditInfo {
            start_byte,
            old_end_byte: start_byte,
            new_end_byte,
            start_position: start_pos,
            old_end_position: start_pos,
            new_end_position: new_end_pos,
        })
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

    /// Paste text after the current line. Returns None — caller should do full re-parse.
    pub fn paste_after(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.save_undo();
        if text.ends_with('\n') || text.ends_with("\r\n") {
            // Insert on the line after cursor. Find the end of the current line
            // (including its newline), or end of file if no trailing newline.
            let line_slice = self.rope.line(self.cursor.line);
            let insert_pos = self.rope.line_to_char(self.cursor.line) + line_slice.len_chars();
            // If current line has no trailing newline (last line of file),
            // prepend one so the paste starts on its own line.
            let needs_nl = insert_pos == self.rope.len_chars()
                && insert_pos > 0
                && self.rope.char(insert_pos - 1) != '\n';
            if needs_nl {
                let combined = format!("\n{}", text);
                self.rope.insert(insert_pos, &combined);
            } else {
                self.rope.insert(insert_pos, text);
            }
            self.cursor.line += 1;
            self.cursor.col = 0;
        } else {
            let line_start = self.rope.line_to_char(self.cursor.line);
            let line_len = self.current_line_len();
            // Insert after cursor char, clamped to line bounds.
            // On empty lines, insert at line start (before the newline).
            let clamped_col = self.cursor.col.min(line_len.saturating_sub(1));
            let insert_col = if line_len == 0 { 0 } else { clamped_col + 1 };
            let insert_at = (line_start + insert_col).min(self.rope.len_chars());
            self.rope.insert(insert_at, text);
            // Place cursor at end of inserted text.
            let newlines = text.chars().filter(|c| *c == '\n').count();
            if newlines > 0 {
                self.cursor.line += newlines;
                let last_nl = text.rfind('\n').unwrap();
                self.cursor.col = text.len() - last_nl - 2;
            } else {
                self.cursor.col = insert_col + text.len() - 1;
            }
            self.clamp_cursor();
        }
        self.modified = true;
        self.invalidate_hash();
    }

    /// Paste text before the current line. Returns None — caller should do full re-parse.
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
        self.invalidate_hash();
    }

    fn cursor_char_idx(&self) -> usize {
        if self.cursor.line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(self.cursor.line);
        let line_len = self.current_line_len();
        line_start + self.cursor.col.min(line_len)
    }

    /// (row, col_in_bytes) for a given char index — used for tree-sitter Point.
    fn position_at_char(&self, char_idx: usize) -> (usize, usize) {
        let line = self.rope.char_to_line(char_idx);
        let line_start_byte = self.rope.line_to_byte(line);
        let byte = self.rope.char_to_byte(char_idx);
        (line, byte - line_start_byte)
    }

    pub fn goto_line(&mut self, line: usize) {
        self.cursor.line = line.min(self.line_count().saturating_sub(1));
        self.clamp_cursor();
    }
}

/// Whether a character is a "word" character (alphanumeric or underscore).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Vim character class: Word, Punctuation (non-blank non-word), or Whitespace.
#[derive(PartialEq, Eq)]
enum CharClass {
    Word,
    Punctuation,
    Whitespace,
}

fn char_class(c: char) -> CharClass {
    if is_word_char(c) {
        CharClass::Word
    } else if c.is_whitespace() {
        CharClass::Whitespace
    } else {
        CharClass::Punctuation
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
