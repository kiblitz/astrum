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

    pub fn mark_modified(&mut self) {
        self.modified = true;
        self.invalidate_hash();
    }

    /// Build an EditInfo for a deletion (new_end == start, content shrank).
    fn edit_info_delete(&self, start_char: usize, end_char: usize) -> EditInfo {
        EditInfo {
            start_byte: self.rope.char_to_byte(start_char),
            old_end_byte: self.rope.char_to_byte(end_char),
            new_end_byte: self.rope.char_to_byte(start_char),
            start_position: self.position_at_char(start_char),
            old_end_position: self.position_at_char(end_char),
            new_end_position: self.position_at_char(start_char),
        }
    }

    /// Build an EditInfo for an insertion (old_end == start, content grew).
    fn edit_info_insert(&self, start_char: usize, new_end_char: usize) -> EditInfo {
        EditInfo {
            start_byte: self.rope.char_to_byte(start_char),
            old_end_byte: self.rope.char_to_byte(start_char),
            new_end_byte: self.rope.char_to_byte(new_end_char),
            start_position: self.position_at_char(start_char),
            old_end_position: self.position_at_char(start_char),
            new_end_position: self.position_at_char(new_end_char),
        }
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
        self.rope.insert_char(idx, ch);
        if ch == '\n' {
            self.cursor.line += 1;
            self.cursor.col = 0;
        } else {
            self.cursor.col += 1;
        }
        self.mark_modified();
        Some(self.edit_info_insert(idx, idx + 1))
    }

    pub fn insert_newline(&mut self) -> Option<EditInfo> {
        self.insert_char('\n')
    }

    pub fn delete_char_backward(&mut self) -> Option<EditInfo> {
        let idx = self.cursor_char_idx();
        if idx == 0 {
            return None;
        }
        self.save_undo();
        let info = self.edit_info_delete(idx - 1, idx);
        self.rope.remove(idx - 1..idx);
        if self.cursor.col == 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.current_line_len();
        } else {
            self.cursor.col -= 1;
        }
        self.mark_modified();
        Some(info)
    }

    pub fn delete_word_backward(&mut self) -> Option<EditInfo> {
        let end_idx = self.cursor_char_idx();
        if end_idx == 0 {
            return None;
        }

        // Probe where word-backward would land without committing yet.
        let saved = self.cursor;
        self.move_word_backward();
        let start_idx = self.cursor_char_idx();

        if start_idx == end_idx {
            self.cursor = saved;
            return None;
        }

        // Now we know the edit will happen — save undo with the original cursor.
        self.cursor = saved;
        self.save_undo();
        let info = self.edit_info_delete(start_idx, end_idx);
        self.rope.remove(start_idx..end_idx);
        self.cursor_to_char_pos(start_idx);
        self.mark_modified();
        Some(info)
    }

    pub fn delete_char_forward(&mut self) -> Option<EditInfo> {
        let idx = self.cursor_char_idx();
        if idx >= self.rope.len_chars() {
            return None;
        }
        self.save_undo();
        let info = self.edit_info_delete(idx, idx + 1);
        self.rope.remove(idx..idx + 1);
        self.clamp_cursor();
        self.mark_modified();
        Some(info)
    }

    pub fn delete_line(&mut self) -> Option<EditInfo> {
        let (line_start, line_end) = self.linewise_range(self.cursor.line, self.cursor.line);
        if line_start >= line_end {
            return None;
        }
        self.save_undo();
        let info = self.edit_info_delete(line_start, line_end);
        self.rope.remove(line_start..line_end);
        self.clamp_cursor();
        self.mark_modified();
        Some(info)
    }

    /// Delete a range of characters by char index. Cursor is placed at `start`.
    pub fn delete_char_range(&mut self, start: usize, end: usize) -> Option<EditInfo> {
        if start >= end || start >= self.rope.len_chars() {
            return None;
        }
        let end = end.min(self.rope.len_chars());
        self.save_undo();
        let info = self.edit_info_delete(start, end);
        self.rope.remove(start..end);
        self.cursor_to_char_pos(start);
        self.clamp_cursor();
        self.mark_modified();
        Some(info)
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

    pub fn move_word_forward(&mut self) { self.move_word_forward_impl(false); }
    pub fn move_big_word_forward(&mut self) { self.move_word_forward_impl(true); }

    pub fn move_word_end(&mut self) { self.move_word_end_impl(false); }
    pub fn move_big_word_end(&mut self) { self.move_word_end_impl(true); }

    pub fn move_word_backward(&mut self) { self.move_word_backward_impl(false); }
    pub fn move_big_word_backward(&mut self) { self.move_word_backward_impl(true); }

    /// Move to start of next word. `big` = true treats all non-whitespace as one class.
    fn move_word_forward_impl(&mut self, big: bool) {
        let line_count = self.line_count();
        let mut line = self.cursor.line;
        let mut col = self.cursor.col;

        // On the current line: skip past current class, then whitespace.
        if line < line_count {
            let rope_line = self.rope.line(line);
            let line_len = line_len_no_newline(&rope_line);

            if col < line_len {
                let start = rope_line.char(col);
                if !start.is_whitespace() {
                    while col < line_len && same_word_class(rope_line.char(col), start, big) {
                        col += 1;
                    }
                }
            }
            while col < line_len && rope_line.char(col).is_whitespace() {
                col += 1;
            }

            if col < line_len {
                self.cursor.line = line;
                self.cursor.col = col;
                return;
            }
        }

        // Crossed end of current line — find first non-blank on subsequent lines.
        line += 1;
        while line < line_count {
            let rope_line = self.rope.line(line);
            let line_len = line_len_no_newline(&rope_line);

            let mut c = 0;
            while c < line_len && rope_line.char(c).is_whitespace() {
                c += 1;
            }
            if c < line_len {
                self.cursor.line = line;
                self.cursor.col = c;
                return;
            }
            line += 1;
        }

        // No next word — move to last character of file (vim behavior).
        let last_line = line_count.saturating_sub(1);
        let rope_line = self.rope.line(last_line);
        let last_col = line_len_no_newline(&rope_line).saturating_sub(1);
        self.cursor.line = last_line;
        self.cursor.col = last_col;
    }

    /// Move to end of current/next word. `big` = true treats all non-whitespace as one class.
    fn move_word_end_impl(&mut self, big: bool) {
        let line_count = self.line_count();
        let mut line = self.cursor.line;
        let mut col = self.cursor.col + 1; // Move past current position.

        // Scan forward: skip whitespace, then advance to end of class.
        // Tries current line first, then subsequent lines.
        loop {
            if line >= line_count {
                break;
            }
            let rope_line = self.rope.line(line);
            let line_len = line_len_no_newline(&rope_line);

            while col < line_len && rope_line.char(col).is_whitespace() {
                col += 1;
            }
            if col < line_len {
                let start = rope_line.char(col);
                while col + 1 < line_len
                    && same_word_class(rope_line.char(col + 1), start, big)
                {
                    col += 1;
                }
                self.cursor.line = line;
                self.cursor.col = col;
                return;
            }

            line += 1;
            col = 0;
        }
        // No next word found — stay put.
    }

    /// Move to start of previous word. `big` = true treats all non-whitespace as one class.
    fn move_word_backward_impl(&mut self, big: bool) {
        let mut line = self.cursor.line;
        let mut col = self.cursor.col;

        loop {
            if col > 0 {
                let rope_line = self.rope.line(line);
                // Skip whitespace backward.
                while col > 0 && rope_line.char(col - 1).is_whitespace() {
                    col -= 1;
                }
                // Skip current class backward.
                if col > 0 {
                    let start = rope_line.char(col - 1);
                    while col > 0 && same_word_class(rope_line.char(col - 1), start, big) {
                        col -= 1;
                    }
                    self.cursor.line = line;
                    self.cursor.col = col;
                    return;
                }
            }
            // At start of line — move to end of previous line.
            if line == 0 {
                self.cursor.col = 0;
                return;
            }
            line -= 1;
            col = line_len_no_newline(&self.rope.line(line));
        }
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
        self.rope.insert(insert_at, text);
        self.cursor.line += 1;
        self.cursor.col = 0;
        self.mark_modified();
        Some(self.edit_info_insert(insert_at, insert_at + text.chars().count()))
    }

    pub fn insert_line_above(&mut self) -> Option<EditInfo> {
        self.save_undo();
        let line_start = self.rope.line_to_char(self.cursor.line);
        self.rope.insert_char(line_start, '\n');
        self.cursor.col = 0;
        self.mark_modified();
        Some(self.edit_info_insert(line_start, line_start + 1))
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
            let char_count = text.chars().count();
            let newlines = text.chars().filter(|c| *c == '\n').count();
            if newlines > 0 {
                self.cursor.line += newlines;
                let after_last_nl = text.chars().rev().take_while(|c| *c != '\n').count();
                self.cursor.col = after_last_nl.saturating_sub(1);
            } else {
                self.cursor.col = insert_col + char_count - 1;
            }
            self.clamp_cursor();
        }
        self.mark_modified();
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
        self.mark_modified();
    }

    /// Set cursor to the (line, col) corresponding to a char index.
    fn cursor_to_char_pos(&mut self, char_idx: usize) {
        let clamped = char_idx.min(self.rope.len_chars().saturating_sub(1));
        self.cursor.line = self.rope.char_to_line(clamped);
        let line_start = self.rope.line_to_char(self.cursor.line);
        self.cursor.col = char_idx.saturating_sub(line_start);
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

    /// Toggle line comments on a range of lines. If all lines are commented,
    /// uncomment them. Otherwise, comment all lines.
    /// Returns None (caller should do full re-parse).
    pub fn toggle_line_comment(&mut self, first_line: usize, last_line: usize, prefix: &str) {
        let line_count = self.line_count();
        let first = first_line.min(line_count.saturating_sub(1));
        let last = last_line.min(line_count.saturating_sub(1));

        // Check if all non-empty lines in the range are already commented.
        let prefix_space = format!("{} ", prefix);
        let all_commented = (first..=last).all(|ln| {
            let line = self.rope.line(ln);
            let trimmed: String = line.chars().collect::<String>();
            let trimmed = trimmed.trim_start();
            trimmed.is_empty() || trimmed.starts_with(&prefix_space) || trimmed.starts_with(prefix)
        });

        self.save_undo();

        if all_commented {
            // Uncomment: remove the comment prefix (and one trailing space if present).
            for ln in (first..=last).rev() {
                let line = self.rope.line(ln);
                let text: String = line.chars().collect();
                let trimmed = text.trim_start();
                if trimmed.is_empty() {
                    continue;
                }
                let indent_chars = text.chars().take_while(|c| c.is_whitespace()).count();
                let line_start = self.rope.line_to_char(ln);
                let prefix_chars = prefix.chars().count();
                if trimmed.starts_with(&prefix_space) {
                    let remove_start = line_start + indent_chars;
                    let remove_end = remove_start + prefix_chars + 1; // prefix + space
                    self.rope.remove(remove_start..remove_end);
                } else if trimmed.starts_with(prefix) {
                    let remove_start = line_start + indent_chars;
                    let remove_end = remove_start + prefix_chars;
                    self.rope.remove(remove_start..remove_end);
                }
            }
        } else {
            // Comment: find minimum indentation of non-empty lines, insert prefix there.
            let min_indent = (first..=last)
                .filter_map(|ln| {
                    let line = self.rope.line(ln);
                    let text: String = line.chars().collect();
                    let trimmed = text.trim_start();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(text.chars().take_while(|c| c.is_whitespace()).count())
                    }
                })
                .min()
                .unwrap_or(0);

            let insert_text = format!("{} ", prefix);
            for ln in (first..=last).rev() {
                let line = self.rope.line(ln);
                let text: String = line.chars().collect();
                if text.trim_start().is_empty() {
                    continue; // Skip empty lines.
                }
                let line_start = self.rope.line_to_char(ln);
                self.rope.insert(line_start + min_indent, &insert_text);
            }
        }

        self.clamp_cursor();
        self.mark_modified();
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

/// Are two characters in the same word class?
/// `big` = true uses WORD semantics (all non-whitespace is one class).
fn same_word_class(a: char, b: char, big: bool) -> bool {
    if big {
        !a.is_whitespace() && !b.is_whitespace()
    } else {
        char_class(a) == char_class(b)
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
