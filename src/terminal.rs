use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use ratatui::style::{Color, Modifier, Style};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
fn next_id() -> usize {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// A single cell in the terminal grid.
#[derive(Clone, Debug)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}

/// A terminal session running a child process via PTY.
pub struct TerminalSession {
    pub id: usize,
    pub grid: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub rows: u16,
    pub cols: u16,
    pub title: String,
    pub exited: Option<String>,
    /// Current style applied to new characters.
    current_style: Style,
    /// Saved cursor position (for ESC 7 / ESC 8).
    saved_cursor: (usize, usize),
    /// VTE parser state machine.
    parser: vte::Parser,
    /// PTY master (for resize).
    master: Box<dyn MasterPty + Send>,
    /// Writer to send input to the child.
    writer: Box<dyn Write + Send>,
    /// Reader to get output from the child (wrapped for thread-safe async reads).
    reader: Arc<Mutex<Option<Box<dyn Read + Send>>>>,
    /// Child process handle.
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Scrollback buffer: lines that scrolled off the top.
    pub scrollback: Vec<Vec<Cell>>,
    /// Scroll offset from the bottom (0 = viewing live output).
    pub scroll_offset: usize,
}

impl TerminalSession {
    /// Spawn a new terminal with the given dimensions and shell.
    pub fn new(rows: u16, cols: u16, shell: Option<&str>) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let default_shell: Option<String> = if shell.is_some() {
            None
        } else if cfg!(windows) {
            Some("powershell".into())
        } else {
            std::env::var("SHELL").ok()
        };
        let shell_name = shell.or(default_shell.as_deref());
        let cmd = if let Some(sh) = shell_name {
            let mut cmd = CommandBuilder::new(sh);
            cmd.cwd(std::env::current_dir().unwrap_or_default());
            cmd
        } else {
            let mut cmd = CommandBuilder::new_default_prog();
            cmd.cwd(std::env::current_dir().unwrap_or_default());
            cmd
        };

        let child = pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;
        let reader = pair.master.try_clone_reader()?;

        let grid = vec![vec![Cell::default(); cols as usize]; rows as usize];

        Ok(Self {
            id: next_id(),
            grid,
            cursor_row: 0,
            cursor_col: 0,
            rows,
            cols,
            title: String::from("[terminal]"),
            exited: None,
            current_style: Style::default(),
            saved_cursor: (0, 0),
            parser: vte::Parser::new(),
            master: pair.master,
            writer,
            reader: Arc::new(Mutex::new(Some(reader))),
            child,
            scrollback: Vec::new(),
            scroll_offset: 0,
        })
    }

    /// Send raw bytes to the child process.
    pub fn write_bytes(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()
    }

    /// Process raw bytes received from the PTY through the VTE parser.
    /// Called with data read from the PTY reader (typically in a background thread).
    pub fn process_output(&mut self, data: &[u8]) {
        self.process_bytes(data);
    }

    fn process_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            // We use a helper struct because vte::Parser::advance needs
            // the Perform implementor as a separate mutable reference.
            let mut performer = TermPerformer {
                grid: &mut self.grid,
                cursor_row: &mut self.cursor_row,
                cursor_col: &mut self.cursor_col,
                rows: self.rows as usize,
                cols: self.cols as usize,
                current_style: &mut self.current_style,
                saved_cursor: &mut self.saved_cursor,
                title: &mut self.title,
                scrollback: &mut self.scrollback,
            };
            self.parser.advance(&mut performer, byte);
        }
    }

    /// Check if the child process has exited.
    pub fn check_exit(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.exited = Some(format!("Process exited ({})", status));
                true
            }
            _ => false,
        }
    }

    /// Resize the terminal grid and notify the PTY.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        if rows == self.rows && cols == self.cols {
            return;
        }
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });

        let new_rows = rows as usize;
        let new_cols = cols as usize;

        // Resize the grid: adjust columns, then rows.
        for row in &mut self.grid {
            row.resize(new_cols, Cell::default());
        }
        self.grid.resize(new_rows, vec![Cell::default(); new_cols]);

        self.rows = rows;
        self.cols = cols;

        // Clamp cursor.
        if self.cursor_row >= new_rows {
            self.cursor_row = new_rows.saturating_sub(1);
        }
        if self.cursor_col >= new_cols {
            self.cursor_col = new_cols.saturating_sub(1);
        }
    }

    /// Kill the child process.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        // Drop the reader so read threads don't block.
        if let Ok(mut guard) = self.reader.lock() {
            *guard = None;
        }
    }

    /// Take the reader for async reading in a background thread.
    pub fn take_reader(&self) -> Option<Box<dyn Read + Send>> {
        let mut guard = self.reader.lock().ok()?;
        guard.take()
    }

    /// Create a mock terminal for testing (no real PTY).
    /// Populates the grid with the given text lines.
    pub fn new_mock(rows: u16, cols: u16, lines: &[&str], title: &str) -> Self {
        let mut grid = vec![vec![Cell::default(); cols as usize]; rows as usize];
        for (y, line) in lines.iter().enumerate() {
            if y >= rows as usize {
                break;
            }
            for (x, ch) in line.chars().enumerate() {
                if x >= cols as usize {
                    break;
                }
                grid[y][x].ch = ch;
            }
        }

        // Use portable-pty to create a real but unused PTY pair for the type system.
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 1,
            cols: 1,
            pixel_width: 0,
            pixel_height: 0,
        }).expect("mock PTY allocation failed");
        let writer = pair.master.take_writer().expect("mock writer failed");
        let reader = pair.master.try_clone_reader().expect("mock reader failed");
        let child = pair.slave.spawn_command({
            let mut cmd = CommandBuilder::new(if cfg!(windows) { "cmd.exe" } else { "true" });
            cmd.args([if cfg!(windows) { "/c" } else { "" }, if cfg!(windows) { "exit" } else { "" }]);
            cmd
        }).expect("mock child spawn failed");

        Self {
            id: next_id(),
            grid,
            cursor_row: 0,
            cursor_col: 0,
            rows,
            cols,
            title: title.to_string(),
            exited: None,
            current_style: Style::default(),
            saved_cursor: (0, 0),
            parser: vte::Parser::new(),
            master: pair.master,
            writer,
            reader: Arc::new(Mutex::new(Some(reader))),
            child,
            scrollback: Vec::new(),
            scroll_offset: 0,
        }
    }
}

/// Convert a crossterm KeyEvent to bytes suitable for sending to a PTY.
pub fn key_to_bytes(key: &crossterm::event::KeyEvent) -> Option<Vec<u8>> {
    use crossterm::event::{KeyCode, KeyModifiers};

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        KeyCode::Char(c) if ctrl => {
            // Ctrl+A..Z maps to 0x01..0x1A.
            let byte = (c.to_ascii_lowercase() as u8).wrapping_sub(b'a').wrapping_add(1);
            if byte <= 26 {
                Some(vec![byte])
            } else {
                Some(c.to_string().into_bytes())
            }
        }
        KeyCode::Char(c) if alt => {
            // Alt sends ESC prefix.
            let mut bytes = vec![0x1b];
            bytes.extend(c.to_string().as_bytes());
            Some(bytes)
        }
        KeyCode::Char(c) => Some(c.to_string().into_bytes()),
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        KeyCode::Esc => Some(vec![0x1b]),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        KeyCode::Insert => Some(b"\x1b[2~".to_vec()),
        KeyCode::F(n) => {
            let code = match n {
                1 => "\x1bOP",
                2 => "\x1bOQ",
                3 => "\x1bOR",
                4 => "\x1bOS",
                5 => "\x1b[15~",
                6 => "\x1b[17~",
                7 => "\x1b[18~",
                8 => "\x1b[19~",
                9 => "\x1b[20~",
                10 => "\x1b[21~",
                11 => "\x1b[23~",
                12 => "\x1b[24~",
                _ => return None,
            };
            Some(code.as_bytes().to_vec())
        }
        _ => None,
    }
}

// --- VTE Perform implementation ---

/// Separate struct to implement vte::Perform, borrowing terminal state.
struct TermPerformer<'a> {
    grid: &'a mut Vec<Vec<Cell>>,
    cursor_row: &'a mut usize,
    cursor_col: &'a mut usize,
    rows: usize,
    cols: usize,
    current_style: &'a mut Style,
    saved_cursor: &'a mut (usize, usize),
    title: &'a mut String,
    scrollback: &'a mut Vec<Vec<Cell>>,
}

impl<'a> TermPerformer<'a> {
    fn scroll_up(&mut self) {
        if !self.grid.is_empty() {
            let top_row = self.grid.remove(0);
            self.scrollback.push(top_row);
            // Cap scrollback at 10000 lines.
            if self.scrollback.len() > 10000 {
                self.scrollback.remove(0);
            }
            self.grid.push(vec![Cell::default(); self.cols]);
        }
    }

    fn ensure_row(&mut self) {
        while *self.cursor_row >= self.grid.len() {
            self.grid.push(vec![Cell::default(); self.cols]);
        }
    }
}

impl<'a> vte::Perform for TermPerformer<'a> {
    fn print(&mut self, c: char) {
        self.ensure_row();
        if *self.cursor_col >= self.cols {
            // Line wrap.
            *self.cursor_col = 0;
            *self.cursor_row += 1;
            if *self.cursor_row >= self.rows {
                self.scroll_up();
                *self.cursor_row = self.rows - 1;
            }
            self.ensure_row();
        }
        if let Some(cell) = self.grid[*self.cursor_row].get_mut(*self.cursor_col) {
            cell.ch = c;
            cell.style = *self.current_style;
        }
        *self.cursor_col += 1;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // Backspace.
            0x08 => {
                *self.cursor_col = self.cursor_col.saturating_sub(1);
            }
            // Tab.
            0x09 => {
                let next_tab = (*self.cursor_col / 8 + 1) * 8;
                *self.cursor_col = next_tab.min(self.cols.saturating_sub(1));
            }
            // Newline / Line Feed.
            0x0A | 0x0B | 0x0C => {
                *self.cursor_row += 1;
                if *self.cursor_row >= self.rows {
                    self.scroll_up();
                    *self.cursor_row = self.rows - 1;
                }
            }
            // Carriage Return.
            0x0D => {
                *self.cursor_col = 0;
            }
            // Bell — ignore.
            0x07 => {}
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params_vec: Vec<Vec<u16>> = params.iter().map(|s| s.to_vec()).collect();
        let p = |idx: usize, default: u16| -> u16 {
            params_vec
                .get(idx)
                .and_then(|s| s.first().copied())
                .filter(|&v| v != 0)
                .unwrap_or(default)
        };

        match action {
            // Cursor Up.
            'A' => {
                let n = p(0, 1) as usize;
                *self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            // Cursor Down.
            'B' => {
                let n = p(0, 1) as usize;
                *self.cursor_row = (*self.cursor_row + n).min(self.rows.saturating_sub(1));
            }
            // Cursor Forward.
            'C' => {
                let n = p(0, 1) as usize;
                *self.cursor_col = (*self.cursor_col + n).min(self.cols.saturating_sub(1));
            }
            // Cursor Back.
            'D' => {
                let n = p(0, 1) as usize;
                *self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            // Cursor Next Line.
            'E' => {
                let n = p(0, 1) as usize;
                *self.cursor_row = (*self.cursor_row + n).min(self.rows.saturating_sub(1));
                *self.cursor_col = 0;
            }
            // Cursor Previous Line.
            'F' => {
                let n = p(0, 1) as usize;
                *self.cursor_row = self.cursor_row.saturating_sub(n);
                *self.cursor_col = 0;
            }
            // Cursor Horizontal Absolute.
            'G' => {
                let col = p(0, 1) as usize;
                *self.cursor_col = col.saturating_sub(1).min(self.cols.saturating_sub(1));
            }
            // Cursor Position (CUP).
            'H' | 'f' => {
                let row = p(0, 1) as usize;
                let col = p(1, 1) as usize;
                *self.cursor_row = row.saturating_sub(1).min(self.rows.saturating_sub(1));
                *self.cursor_col = col.saturating_sub(1).min(self.cols.saturating_sub(1));
            }
            // Erase in Display.
            'J' => {
                let mode = p(0, 0);
                match mode {
                    0 => {
                        // Clear from cursor to end of screen.
                        self.ensure_row();
                        for col in *self.cursor_col..self.cols {
                            if let Some(cell) = self.grid[*self.cursor_row].get_mut(col) {
                                *cell = Cell::default();
                            }
                        }
                        for row in (*self.cursor_row + 1)..self.rows {
                            if let Some(r) = self.grid.get_mut(row) {
                                for cell in r.iter_mut() {
                                    *cell = Cell::default();
                                }
                            }
                        }
                    }
                    1 => {
                        // Clear from start to cursor.
                        for row in 0..*self.cursor_row {
                            if let Some(r) = self.grid.get_mut(row) {
                                for cell in r.iter_mut() {
                                    *cell = Cell::default();
                                }
                            }
                        }
                        self.ensure_row();
                        for col in 0..=*self.cursor_col {
                            if let Some(cell) = self.grid[*self.cursor_row].get_mut(col) {
                                *cell = Cell::default();
                            }
                        }
                    }
                    2 | 3 => {
                        // Clear entire screen.
                        for row in self.grid.iter_mut() {
                            for cell in row.iter_mut() {
                                *cell = Cell::default();
                            }
                        }
                    }
                    _ => {}
                }
            }
            // Erase in Line.
            'K' => {
                let mode = p(0, 0);
                self.ensure_row();
                match mode {
                    0 => {
                        for col in *self.cursor_col..self.cols {
                            if let Some(cell) = self.grid[*self.cursor_row].get_mut(col) {
                                *cell = Cell::default();
                            }
                        }
                    }
                    1 => {
                        for col in 0..=*self.cursor_col {
                            if let Some(cell) = self.grid[*self.cursor_row].get_mut(col) {
                                *cell = Cell::default();
                            }
                        }
                    }
                    2 => {
                        for cell in self.grid[*self.cursor_row].iter_mut() {
                            *cell = Cell::default();
                        }
                    }
                    _ => {}
                }
            }
            // Insert Lines.
            'L' => {
                let n = p(0, 1) as usize;
                for _ in 0..n {
                    if *self.cursor_row < self.rows {
                        self.grid
                            .insert(*self.cursor_row, vec![Cell::default(); self.cols]);
                        if self.grid.len() > self.rows {
                            self.grid.pop();
                        }
                    }
                }
            }
            // Delete Lines.
            'M' => {
                let n = p(0, 1) as usize;
                for _ in 0..n {
                    if *self.cursor_row < self.grid.len() {
                        self.grid.remove(*self.cursor_row);
                        self.grid.push(vec![Cell::default(); self.cols]);
                    }
                }
            }
            // Delete Characters.
            'P' => {
                let n = p(0, 1) as usize;
                self.ensure_row();
                let row = &mut self.grid[*self.cursor_row];
                for _ in 0..n {
                    if *self.cursor_col < row.len() {
                        row.remove(*self.cursor_col);
                        row.push(Cell::default());
                    }
                }
            }
            // Scroll Up.
            'S' => {
                let n = p(0, 1) as usize;
                for _ in 0..n {
                    self.scroll_up();
                }
            }
            // Scroll Down.
            'T' => {
                let n = p(0, 1) as usize;
                for _ in 0..n {
                    self.grid.insert(0, vec![Cell::default(); self.cols]);
                    if self.grid.len() > self.rows {
                        self.grid.pop();
                    }
                }
            }
            // Erase Characters.
            'X' => {
                let n = p(0, 1) as usize;
                self.ensure_row();
                for i in 0..n {
                    let col = *self.cursor_col + i;
                    if let Some(cell) = self.grid[*self.cursor_row].get_mut(col) {
                        *cell = Cell::default();
                    }
                }
            }
            // SGR — Select Graphic Rendition.
            'm' => {
                self.apply_sgr(&params_vec);
            }
            // Save cursor (DECSC via CSI).
            's' if intermediates.is_empty() => {
                *self.saved_cursor = (*self.cursor_row, *self.cursor_col);
            }
            // Restore cursor (DECRC via CSI).
            'u' if intermediates.is_empty() => {
                *self.cursor_row = self.saved_cursor.0.min(self.rows.saturating_sub(1));
                *self.cursor_col = self.saved_cursor.1.min(self.cols.saturating_sub(1));
            }
            // Insert Characters.
            '@' => {
                let n = p(0, 1) as usize;
                self.ensure_row();
                let row = &mut self.grid[*self.cursor_row];
                for _ in 0..n {
                    row.insert(*self.cursor_col, Cell::default());
                    if row.len() > self.cols {
                        row.pop();
                    }
                }
            }
            // Cursor visibility, mode sets — ignore for now.
            'h' | 'l' | 'r' | 'c' | 'n' | 'd' | 't' => {}
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            // Save cursor (DECSC).
            b'7' => {
                *self.saved_cursor = (*self.cursor_row, *self.cursor_col);
            }
            // Restore cursor (DECRC).
            b'8' => {
                *self.cursor_row = self.saved_cursor.0.min(self.rows.saturating_sub(1));
                *self.cursor_col = self.saved_cursor.1.min(self.cols.saturating_sub(1));
            }
            // Reverse Index — move cursor up, scroll if at top.
            b'M' => {
                if *self.cursor_row == 0 {
                    self.grid.insert(0, vec![Cell::default(); self.cols]);
                    if self.grid.len() > self.rows {
                        self.grid.pop();
                    }
                } else {
                    *self.cursor_row -= 1;
                }
            }
            // Reset.
            b'c' if intermediates.is_empty() => {
                for row in self.grid.iter_mut() {
                    for cell in row.iter_mut() {
                        *cell = Cell::default();
                    }
                }
                *self.cursor_row = 0;
                *self.cursor_col = 0;
                *self.current_style = Style::default();
            }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // OSC 0 or 2: set window title.
        if let Some(&first) = params.first() {
            if first == b"0" || first == b"2" {
                if let Some(title_bytes) = params.get(1) {
                    if let Ok(title) = std::str::from_utf8(title_bytes) {
                        *self.title = title.to_string();
                    }
                }
            }
        }
    }
}

impl<'a> TermPerformer<'a> {
    fn apply_sgr(&mut self, params: &[Vec<u16>]) {
        if params.is_empty() || (params.len() == 1 && params[0].first() == Some(&0)) {
            *self.current_style = Style::default();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            let code = params[i].first().copied().unwrap_or(0);
            match code {
                0 => *self.current_style = Style::default(),
                1 => *self.current_style = self.current_style.add_modifier(Modifier::BOLD),
                2 => *self.current_style = self.current_style.add_modifier(Modifier::DIM),
                3 => *self.current_style = self.current_style.add_modifier(Modifier::ITALIC),
                4 => *self.current_style = self.current_style.add_modifier(Modifier::UNDERLINED),
                7 => *self.current_style = self.current_style.add_modifier(Modifier::REVERSED),
                9 => *self.current_style = self.current_style.add_modifier(Modifier::CROSSED_OUT),
                22 => {
                    *self.current_style = self
                        .current_style
                        .remove_modifier(Modifier::BOLD)
                        .remove_modifier(Modifier::DIM)
                }
                23 => {
                    *self.current_style =
                        self.current_style.remove_modifier(Modifier::ITALIC)
                }
                24 => {
                    *self.current_style =
                        self.current_style.remove_modifier(Modifier::UNDERLINED)
                }
                27 => {
                    *self.current_style =
                        self.current_style.remove_modifier(Modifier::REVERSED)
                }
                // Foreground colors.
                30 => *self.current_style = self.current_style.fg(Color::Black),
                31 => *self.current_style = self.current_style.fg(Color::Red),
                32 => *self.current_style = self.current_style.fg(Color::Green),
                33 => *self.current_style = self.current_style.fg(Color::Yellow),
                34 => *self.current_style = self.current_style.fg(Color::Blue),
                35 => *self.current_style = self.current_style.fg(Color::Magenta),
                36 => *self.current_style = self.current_style.fg(Color::Cyan),
                37 => *self.current_style = self.current_style.fg(Color::Gray),
                38 => {
                    if let Some(color) = self.parse_extended_color(params, &mut i) {
                        *self.current_style = self.current_style.fg(color);
                    }
                }
                39 => *self.current_style = self.current_style.fg(Color::Reset),
                // Background colors.
                40 => *self.current_style = self.current_style.bg(Color::Black),
                41 => *self.current_style = self.current_style.bg(Color::Red),
                42 => *self.current_style = self.current_style.bg(Color::Green),
                43 => *self.current_style = self.current_style.bg(Color::Yellow),
                44 => *self.current_style = self.current_style.bg(Color::Blue),
                45 => *self.current_style = self.current_style.bg(Color::Magenta),
                46 => *self.current_style = self.current_style.bg(Color::Cyan),
                47 => *self.current_style = self.current_style.bg(Color::Gray),
                48 => {
                    if let Some(color) = self.parse_extended_color(params, &mut i) {
                        *self.current_style = self.current_style.bg(color);
                    }
                }
                49 => *self.current_style = self.current_style.bg(Color::Reset),
                // Bright foreground.
                90 => *self.current_style = self.current_style.fg(Color::DarkGray),
                91 => *self.current_style = self.current_style.fg(Color::LightRed),
                92 => *self.current_style = self.current_style.fg(Color::LightGreen),
                93 => *self.current_style = self.current_style.fg(Color::LightYellow),
                94 => *self.current_style = self.current_style.fg(Color::LightBlue),
                95 => *self.current_style = self.current_style.fg(Color::LightMagenta),
                96 => *self.current_style = self.current_style.fg(Color::LightCyan),
                97 => *self.current_style = self.current_style.fg(Color::White),
                // Bright background.
                100 => *self.current_style = self.current_style.bg(Color::DarkGray),
                101 => *self.current_style = self.current_style.bg(Color::LightRed),
                102 => *self.current_style = self.current_style.bg(Color::LightGreen),
                103 => *self.current_style = self.current_style.bg(Color::LightYellow),
                104 => *self.current_style = self.current_style.bg(Color::LightBlue),
                105 => *self.current_style = self.current_style.bg(Color::LightMagenta),
                106 => *self.current_style = self.current_style.bg(Color::LightCyan),
                107 => *self.current_style = self.current_style.bg(Color::White),
                _ => {}
            }
            i += 1;
        }
    }

    /// Parse 256-color (38;5;N) or true-color (38;2;R;G;B) sequences.
    fn parse_extended_color(&self, params: &[Vec<u16>], i: &mut usize) -> Option<Color> {
        // Extended colors can come as sub-params (38:5:N) or separate params (38;5;N).
        let first = &params[*i];
        if first.len() >= 3 && first[1] == 5 {
            // Sub-param form: 38:5:N
            return Some(Color::Indexed(first[2] as u8));
        }
        if first.len() >= 5 && first[1] == 2 {
            // Sub-param form: 38:2:R:G:B
            return Some(Color::Rgb(first[2] as u8, first[3] as u8, first[4] as u8));
        }
        // Separate param form.
        if *i + 1 < params.len() {
            let mode = params[*i + 1].first().copied().unwrap_or(0);
            if mode == 5 && *i + 2 < params.len() {
                let idx = params[*i + 2].first().copied().unwrap_or(0);
                *i += 2;
                return Some(Color::Indexed(idx as u8));
            }
            if mode == 2 && *i + 4 < params.len() {
                let r = params[*i + 2].first().copied().unwrap_or(0) as u8;
                let g = params[*i + 3].first().copied().unwrap_or(0) as u8;
                let b = params[*i + 4].first().copied().unwrap_or(0) as u8;
                *i += 4;
                return Some(Color::Rgb(r, g, b));
            }
        }
        None
    }
}
