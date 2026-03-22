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

    /// Handle a key event and return an action (or Noop).
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match self.mode {
            Mode::Command => return self.handle_command_key(key),
            Mode::Search => return self.handle_search_key(key),
            _ => {}
        }

        let input = KeyInput::from_event(&key);

        // Esc cancels any pending key chain.
        if input.code == KeyCode::Esc && self.pending_branch.is_some() {
            self.pending_branch = None;
            self.pending_display.clear();
            self.count_prefix = None;
            return Action::Noop;
        }

        // If we're in the middle of a key chain, continue the trie walk.
        if let Some(branch) = self.pending_branch.take() {
            match lookup_in_map(&branch, &input) {
                KeyLookup::Found(action) => {
                    self.pending_display.clear();
                    self.count_prefix = None;
                    return action;
                }
                KeyLookup::Prefix(children) => {
                    self.pending_display.push(' ');
                    self.pending_display.push_str(&input.display());
                    self.pending_branch = Some(children.clone());
                    return Action::Noop;
                }
                KeyLookup::Miss => {
                    // Chain broken — reset.
                    self.pending_display.clear();
                    self.count_prefix = None;
                    return Action::Noop;
                }
            }
        }

        // Normal/Visual: handle count prefix (digits before a command).
        if self.mode == Mode::Normal {
            if let KeyCode::Char(c) = key.code {
                if c.is_ascii_digit() && !input.ctrl && !input.alt {
                    if self.count_prefix.is_some() || c != '0' {
                        let digit = c.to_digit(10).unwrap() as usize;
                        self.count_prefix =
                            Some(self.count_prefix.unwrap_or(0) * 10 + digit);
                        return Action::Noop;
                    }
                }
            }
        }

        // Look up in the mode's trie.
        let mode_keymap = match self.mode {
            Mode::Normal => &self.keymap.normal,
            Mode::Insert => &self.keymap.insert,
            Mode::Visual => &self.keymap.visual,
            Mode::Command | Mode::Search => unreachable!(),
        };

        match mode_keymap.lookup(&input) {
            KeyLookup::Found(action) => {
                self.count_prefix = None;
                action
            }
            KeyLookup::Prefix(children) => {
                self.pending_display = input.display();
                self.pending_branch = Some(children.clone());
                Action::Noop
            }
            KeyLookup::Miss => {
                self.count_prefix = None;
                // Insert mode fallback: type the character.
                if self.mode == Mode::Insert {
                    match key.code {
                        KeyCode::Char(c) if !input.ctrl && !input.alt => {
                            Action::InsertChar(c)
                        }
                        KeyCode::Tab => Action::InsertChar('\t'),
                        _ => Action::Noop,
                    }
                } else {
                    Action::Noop
                }
            }
        }
    }

    /// Handle a key event using the browser keymap (for file browser navigate mode).
    /// Shares pending chain state with normal handle_key.
    pub fn handle_key_for_browser(&mut self, key: KeyEvent) -> Action {
        let input = KeyInput::from_event(&key);

        // Esc cancels any pending key chain.
        if input.code == KeyCode::Esc && self.pending_branch.is_some() {
            self.pending_branch = None;
            self.pending_display.clear();
            self.count_prefix = None;
            return Action::Noop;
        }

        // Continue pending chain.
        if let Some(branch) = self.pending_branch.take() {
            match lookup_in_map(&branch, &input) {
                KeyLookup::Found(action) => {
                    self.pending_display.clear();
                    self.count_prefix = None;
                    return action;
                }
                KeyLookup::Prefix(children) => {
                    self.pending_display.push(' ');
                    self.pending_display.push_str(&input.display());
                    self.pending_branch = Some(children.clone());
                    return Action::Noop;
                }
                KeyLookup::Miss => {
                    self.pending_display.clear();
                    self.count_prefix = None;
                    return Action::Noop;
                }
            }
        }

        // Count prefix.
        if let KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() && !input.ctrl && !input.alt {
                if self.count_prefix.is_some() || c != '0' {
                    let digit = c.to_digit(10).unwrap() as usize;
                    self.count_prefix =
                        Some(self.count_prefix.unwrap_or(0) * 10 + digit);
                    return Action::Noop;
                }
            }
        }

        // Look up in browser keymap.
        match self.keymap.browser.lookup(&input) {
            KeyLookup::Found(action) => {
                self.count_prefix = None;
                action
            }
            KeyLookup::Prefix(children) => {
                self.pending_display = input.display();
                self.pending_branch = Some(children.clone());
                Action::Noop
            }
            KeyLookup::Miss => {
                self.count_prefix = None;
                Action::Noop
            }
        }
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
