use crate::action::{Action, SearchDirection};
use crate::keymap::{lookup_in_map, KeyInput, KeyLookup, KeyTrieNode, Keymap};
use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashMap;

/// Vim-style modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Command,
    Search,
}

impl Mode {
    pub fn display_name(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Visual => "VISUAL",
            Mode::Command => "COMMAND",
            Mode::Search => "SEARCH",
        }
    }
}

pub struct InputHandler {
    pub mode: Mode,
    pub command_buffer: String,
    pub search_buffer: String,
    pub search_direction: SearchDirection,
    pub pending_display: String,
    keymap: Keymap,

    // Trie traversal state: when we're in the middle of a key chain,
    // this holds the current branch we're waiting to descend into.
    // Key chains never timeout — press Esc to cancel.
    pending_branch: Option<HashMap<KeyInput, KeyTrieNode>>,

    // Normal mode count prefix (e.g. "5j" to move 5 lines).
    pub count_prefix: Option<usize>,

    /// When true, next char keypress produces InsertChar (used by visual surround).
    pub awaiting_char: bool,
}

impl InputHandler {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            mode: Mode::Normal,
            command_buffer: String::new(),
            search_buffer: String::new(),
            search_direction: SearchDirection::Forward,
            pending_display: String::new(),
            keymap,
            pending_branch: None,
            count_prefix: None,
            awaiting_char: false,
        }
    }

    /// Access the keymap (for palette binding introspection).
    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    /// Clear any pending key chain state.
    pub fn clear_pending(&mut self) {
        self.pending_branch = None;
        self.pending_display.clear();
        self.count_prefix = None;
    }

    /// Return the available next keys and their actions when in a pending chain.
    /// Used by the renderer to show which-key hints.
    pub fn pending_hints(&self) -> Vec<(String, String)> {
        let branch = match &self.pending_branch {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut hints: Vec<(String, String)> = branch
            .iter()
            .map(|(key, node)| {
                let key_str = key.display();
                let desc = match node {
                    KeyTrieNode::Leaf(action) => format!("{:?}", action),
                    KeyTrieNode::Branch(_) => "+prefix".to_string(),
                };
                (key_str, desc)
            })
            .collect();
        hints.sort_by(|a, b| a.0.cmp(&b.0));
        hints
    }

    /// Handle Esc, pending key chains, and count prefix — shared by all keymap modes.
    /// Returns `Some(action)` if fully handled, `None` to continue to trie lookup.
    fn handle_common(&mut self, key: &KeyEvent) -> Option<Action> {
        let input = KeyInput::from_event(key);

        // Esc cancels any pending key chain (and any accumulated count prefix).
        if input.code == KeyCode::Esc && self.pending_branch.is_some() {
            self.clear_pending();
            self.count_prefix = None;
            return Some(Action::Noop);
        }

        // Continue pending chain.
        if let Some(branch) = self.pending_branch.take() {
            match lookup_in_map(&branch, &input) {
                KeyLookup::Found(action) => {
                    self.pending_display.clear();
                    return Some(action);
                }
                KeyLookup::Prefix(children) => {
                    self.pending_display.push(' ');
                    self.pending_display.push_str(&input.display());
                    self.pending_branch = Some(children.clone());
                    return Some(Action::Noop);
                }
                KeyLookup::Miss => {
                    self.clear_pending();
                    return Some(Action::Noop);
                }
            }
        }

        // Count prefix (digits before a command) — only in Normal, Visual, and Browser modes.
        let supports_count = matches!(self.mode, Mode::Normal | Mode::Visual);
        if supports_count {
            if let KeyCode::Char(c) = key.code {
                if c.is_ascii_digit() && !input.ctrl && !input.alt {
                    if self.count_prefix.is_some() || c != '0' {
                        let digit = c.to_digit(10).unwrap() as usize;
                        let current = self.count_prefix.unwrap_or(0);
                        self.count_prefix = Some(
                            current.saturating_mul(10).saturating_add(digit)
                        );
                        return Some(Action::Noop);
                    }
                }
            }
        }

        None
    }

    /// Look up a key in a trie and handle prefix/miss. Returns the resolved action.
    fn lookup_keymap(
        input: &KeyInput,
        keymap: &crate::keymap::ModeKeymap,
        pending_display: &mut String,
        pending_branch: &mut Option<HashMap<KeyInput, KeyTrieNode>>,
        count_prefix: &mut Option<usize>,
    ) -> Action {
        match keymap.lookup(input) {
            KeyLookup::Found(action) => action,
            KeyLookup::Prefix(children) => {
                *pending_display = input.display();
                *pending_branch = Some(children.clone());
                Action::Noop
            }
            KeyLookup::Miss => {
                *count_prefix = None;
                Action::Noop
            }
        }
    }

    /// Handle a key event and return an action (or Noop).
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match self.mode {
            Mode::Command => return self.handle_command_key(key),
            Mode::Search => return self.handle_search_key(key),
            _ => {}
        }

        if let Some(action) = self.handle_common(&key) {
            return action;
        }

        // Awaiting a single char (e.g., visual surround): next char bypasses keymap.
        if self.awaiting_char {
            self.awaiting_char = false;
            let input = KeyInput::from_event(&key);
            if let KeyCode::Char(c) = key.code {
                if !input.ctrl && !input.alt {
                    return Action::InsertChar(c);
                }
            }
            // Non-char key (Esc, etc.) cancels the await and resets all pending state.
            return Action::EnterNormalMode;
        }

        let input = KeyInput::from_event(&key);
        let mode_keymap = match self.mode {
            Mode::Normal => &self.keymap.normal,
            Mode::Insert => &self.keymap.insert,
            Mode::Visual => &self.keymap.visual,
            Mode::Command | Mode::Search => unreachable!(),
        };

        let action = Self::lookup_keymap(
            &input, mode_keymap,
            &mut self.pending_display, &mut self.pending_branch, &mut self.count_prefix,
        );
        if action == Action::Noop && self.mode == Mode::Insert {
            match key.code {
                KeyCode::Char(c) if !input.ctrl && !input.alt => Action::InsertChar(c),
                KeyCode::Tab => Action::InsertChar('\t'),
                _ => Action::Noop,
            }
        } else {
            action
        }
    }

    /// Handle a key event using the browser keymap (for file browser navigate mode).
    pub fn handle_key_for_browser(&mut self, key: KeyEvent) -> Action {
        if let Some(action) = self.handle_common(&key) {
            return action;
        }

        let input = KeyInput::from_event(&key);
        Self::lookup_keymap(
            &input, &self.keymap.browser,
            &mut self.pending_display, &mut self.pending_branch, &mut self.count_prefix,
        )
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_buffer.clear();
                Action::Noop
            }
            KeyCode::Enter => {
                let cmd = self.command_buffer.clone();
                self.command_buffer.clear();
                self.mode = Mode::Normal;
                Action::ExecuteCommand(cmd)
            }
            KeyCode::Backspace => {
                if self.command_buffer.is_empty() {
                    self.mode = Mode::Normal;
                } else {
                    self.command_buffer.pop();
                }
                Action::Noop
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
                Action::Noop
            }
            _ => Action::Noop,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.search_buffer.clear();
                Action::SearchCancel
            }
            KeyCode::Enter => {
                let pattern = self.search_buffer.clone();
                self.search_buffer.clear();
                self.mode = Mode::Normal;
                if pattern.is_empty() {
                    // Empty pattern: repeat last search
                    Action::SearchNext
                } else {
                    Action::SearchExecute(pattern, self.search_direction)
                }
            }
            KeyCode::Backspace => {
                if self.search_buffer.is_empty() {
                    self.mode = Mode::Normal;
                    Action::SearchCancel
                } else {
                    self.search_buffer.pop();
                    Action::Noop
                }
            }
            KeyCode::Char(c) => {
                self.search_buffer.push(c);
                Action::Noop
            }
            _ => Action::Noop,
        }
    }
}
