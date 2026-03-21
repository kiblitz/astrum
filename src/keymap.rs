use crate::action::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

/// A normalized key press, stripped of terminal quirks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyInput {
    pub code: KeyCode,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyInput {
    /// Normalize a crossterm KeyEvent into a KeyInput.
    /// For printable chars without ctrl/alt, we drop the SHIFT modifier
    /// because the character itself already encodes shift (e.g. '$' vs '4').
    pub fn from_event(event: &KeyEvent) -> Self {
        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        let alt = event.modifiers.contains(KeyModifiers::ALT);
        match event.code {
            KeyCode::Char(c) if ctrl => KeyInput {
                code: KeyCode::Char(c.to_ascii_lowercase()),
                ctrl: true,
                alt,
            },
            code => KeyInput {
                code,
                ctrl: false,
                alt,
            },
        }
    }

    /// Parse a key string from config notation.
    ///
    /// Examples:
    ///   "a"       → Char('a')
    ///   "A"       → Char('A')
    ///   "C-a"     → Ctrl + Char('a')
    ///   "space"   → Char(' ')
    ///   "esc"     → Esc
    ///   "enter"   → Enter
    ///   "tab"     → Tab
    ///   "up"      → Up
    ///   "C-f"     → Ctrl + Char('f')
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Check for modifier prefixes
        let (ctrl, alt, rest) = parse_modifiers(s);

        // Lowercase only for keyword matching; preserve original case for chars.
        let code = match rest.to_lowercase().as_str() {
            "space" | "spc" => KeyCode::Char(' '),
            "esc" | "escape" => KeyCode::Esc,
            "enter" | "return" | "cr" => KeyCode::Enter,
            "tab" => KeyCode::Tab,
            "s-tab" | "backtab" => KeyCode::BackTab,
            "backspace" | "bs" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdn" => KeyCode::PageDown,
            _ => {
                // Use original `rest` to preserve case (G != g).
                let chars: Vec<char> = rest.chars().collect();
                if chars.len() == 1 {
                    if ctrl {
                        KeyCode::Char(chars[0].to_ascii_lowercase())
                    } else {
                        KeyCode::Char(chars[0])
                    }
                } else {
                    return None;
                }
            }
        };

        Some(KeyInput { code, ctrl, alt })
    }

    /// Format for display (e.g. in the status bar when showing pending keys).
    pub fn display(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("C-");
        }
        if self.alt {
            s.push_str("M-");
        }
        match self.code {
            KeyCode::Char(' ') => s.push_str("SPC"),
            KeyCode::Char(c) => s.push(c),
            KeyCode::Esc => s.push_str("ESC"),
            KeyCode::Enter => s.push_str("RET"),
            KeyCode::Tab => s.push_str("TAB"),
            KeyCode::BackTab => s.push_str("S-TAB"),
            KeyCode::Backspace => s.push_str("BS"),
            KeyCode::Delete => s.push_str("DEL"),
            KeyCode::Up => s.push_str("UP"),
            KeyCode::Down => s.push_str("DOWN"),
            KeyCode::Left => s.push_str("LEFT"),
            KeyCode::Right => s.push_str("RIGHT"),
            _ => s.push('?'),
        }
        s
    }
}

fn parse_modifiers(s: &str) -> (bool, bool, &str) {
    let mut ctrl = false;
    let mut alt = false;
    let mut rest = s;

    loop {
        if let Some(r) = rest.strip_prefix("C-") {
            ctrl = true;
            rest = r;
        } else if let Some(r) = rest.strip_prefix("M-") {
            alt = true;
            rest = r;
        } else if let Some(r) = rest.strip_prefix("A-") {
            alt = true;
            rest = r;
        } else {
            break;
        }
    }

    (ctrl, alt, rest)
}

/// Parse a key sequence string (space-separated keys) into a vec of KeyInputs.
/// Example: "g g" → [KeyInput(g), KeyInput(g)]
///          "space f s" → [KeyInput(space), KeyInput(f), KeyInput(s)]
pub fn parse_key_sequence(s: &str) -> Option<Vec<KeyInput>> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let mut keys = Vec::with_capacity(parts.len());
    for part in parts {
        keys.push(KeyInput::parse(part)?);
    }
    Some(keys)
}

// -- Trie-based keymap --

/// A node in the keymap trie.
#[derive(Debug, Clone)]
pub enum KeyTrieNode {
    /// This key sequence resolves to an action.
    Leaf(Action),
    /// This key is a prefix — more keys are needed.
    Branch(HashMap<KeyInput, KeyTrieNode>),
}

/// Result of looking up a key in the trie.
pub enum KeyLookup<'a> {
    /// Found an action — execute it.
    Found(Action),
    /// This key is a prefix — more keys expected.
    Prefix(&'a HashMap<KeyInput, KeyTrieNode>),
    /// No match for this key.
    Miss,
}

/// The keymap for a single mode.
#[derive(Debug, Clone)]
pub struct ModeKeymap {
    root: HashMap<KeyInput, KeyTrieNode>,
}

impl ModeKeymap {
    pub fn new() -> Self {
        Self {
            root: HashMap::new(),
        }
    }

    /// Insert a key sequence → action binding.
    /// Returns an error if the binding conflicts with an existing one
    /// (a prefix of this sequence is already a leaf, or this sequence
    /// is a prefix of an existing longer binding).
    pub fn bind(&mut self, keys: &[KeyInput], action: Action) -> Result<(), KeymapConflict> {
        if keys.is_empty() {
            return Ok(());
        }
        insert_into_trie(&mut self.root, keys, action)
    }

    /// Look up a key in the root of this mode's trie.
    pub fn lookup(&self, key: &KeyInput) -> KeyLookup<'_> {
        lookup_in_map(&self.root, key)
    }

    /// Collect all bindings as (key_sequence_string, Action) pairs.
    pub fn all_bindings(&self) -> Vec<(String, Action)> {
        let mut result = Vec::new();
        collect_bindings(&self.root, &mut Vec::new(), &mut result);
        result
    }
}

/// Error returned when a key binding conflicts with an existing one.
#[derive(Debug)]
pub struct KeymapConflict {
    pub new_seq: String,
    pub conflict: String,
}

impl std::fmt::Display for KeymapConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "key binding \"{}\" conflicts with existing binding \"{}\"",
            self.new_seq, self.conflict
        )
    }
}

impl std::error::Error for KeymapConflict {}

/// Look up a key in a trie node map.
pub fn lookup_in_map<'a>(
    map: &'a HashMap<KeyInput, KeyTrieNode>,
    key: &KeyInput,
) -> KeyLookup<'a> {
    match map.get(key) {
        Some(KeyTrieNode::Leaf(action)) => KeyLookup::Found(action.clone()),
        Some(KeyTrieNode::Branch(children)) => KeyLookup::Prefix(children),
        None => KeyLookup::Miss,
    }
}

fn insert_into_trie(
    map: &mut HashMap<KeyInput, KeyTrieNode>,
    keys: &[KeyInput],
    action: Action,
) -> Result<(), KeymapConflict> {
    let seq_display = format_key_sequence(keys);

    if keys.len() == 1 {
        if let Some(KeyTrieNode::Branch(_)) = map.get(&keys[0]) {
            return Err(KeymapConflict {
                conflict: format!("{} is already a prefix for longer bindings", seq_display),
                new_seq: seq_display,
            });
        }
        map.insert(keys[0].clone(), KeyTrieNode::Leaf(action));
        return Ok(());
    }

    let node = map
        .entry(keys[0].clone())
        .or_insert_with(|| KeyTrieNode::Branch(HashMap::new()));

    match node {
        KeyTrieNode::Branch(children) => {
            insert_into_trie(children, &keys[1..], action)
        }
        KeyTrieNode::Leaf(existing) => {
            Err(KeymapConflict {
                new_seq: seq_display,
                conflict: format!(
                    "\"{}\" is already bound to {:?}",
                    keys[0].display(),
                    existing
                ),
            })
        }
    }
}

fn format_key_sequence(keys: &[KeyInput]) -> String {
    keys.iter()
        .map(|k| k.display())
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_bindings(
    map: &HashMap<KeyInput, KeyTrieNode>,
    prefix: &mut Vec<KeyInput>,
    result: &mut Vec<(String, Action)>,
) {
    for (key, node) in map {
        prefix.push(key.clone());
        match node {
            KeyTrieNode::Leaf(action) => {
                result.push((format_key_sequence(prefix), action.clone()));
            }
            KeyTrieNode::Branch(children) => {
                collect_bindings(children, prefix, result);
            }
        }
        prefix.pop();
    }
}

/// Full keymap containing bindings for all modes.
#[derive(Debug, Clone)]
pub struct Keymap {
    pub normal: ModeKeymap,
    pub insert: ModeKeymap,
    pub visual: ModeKeymap,
    pub browser: ModeKeymap,
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            normal: ModeKeymap::new(),
            insert: ModeKeymap::new(),
            visual: ModeKeymap::new(),
            browser: ModeKeymap::new(),
        }
    }
}
