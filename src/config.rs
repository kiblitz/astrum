use crate::action::Action;
use crate::keymap::{parse_key_sequence, KeyInput, Keymap};
use anyhow::{bail, Result};
use std::path::PathBuf;

/// General (non-keymap) settings.
pub struct GeneralConfig {
    pub theme: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: "base16-eighties.dark".to_string(),
        }
    }
}

/// Resolved configuration ready for use.
pub struct Config {
    pub general: GeneralConfig,
    pub keymap: Keymap,
}

impl Config {
    /// Load config from the default path, or fall back to defaults.
    pub fn load() -> Result<Self> {
        let mut general = GeneralConfig::default();
        let mut keymap = default_keymap();

        if let Some(path) = config_path() {
            if path.exists() {
                let text = std::fs::read_to_string(&path)?;
                let doc: kdl::KdlDocument = text.parse()
                    .map_err(|e| anyhow::anyhow!("config parse error: {}", e))?;

                // Parse general settings.
                if let Some(node) = doc.get("general") {
                    if let Some(children) = node.children() {
                        if let Some(theme_node) = children.get("theme") {
                            if let Some(val) = theme_node.get(0) {
                                if let Some(s) = val.as_string() {
                                    general.theme = s.to_string();
                                }
                            }
                        }
                    }
                }

                // Parse keymap modes.
                if let Some(keymap_node) = doc.get("keymap") {
                    if let Some(children) = keymap_node.children() {
                        for mode_name in &["normal", "insert", "visual", "browser"] {
                            if let Some(mode_node) = children.get(mode_name) {
                                let mode_keymap = match *mode_name {
                                    "normal" => &mut keymap.normal,
                                    "insert" => &mut keymap.insert,
                                    "visual" => &mut keymap.visual,
                                    "browser" => &mut keymap.browser,
                                    _ => unreachable!(),
                                };
                                if let Some(bindings) = mode_node.children() {
                                    parse_keymap_nodes(
                                        bindings,
                                        &mut Vec::new(),
                                        mode_keymap,
                                        mode_name,
                                    )?;
                                }
                            }
                        }
                    }
                }
            } else {
                // Write default config for the user.
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, DEFAULT_CONFIG)?;
            }
        }

        Ok(Config { general, keymap })
    }
}

/// Recursively walk KDL nodes to build keybindings.
///
/// Each node name is a key (e.g. `j`, `spc`, `C-f`).
/// - Leaf nodes (have a string argument): bind the key chain to the action.
/// - Branch nodes (have children): recurse, extending the key prefix.
/// - A node can be both (has an argument AND children).
fn parse_keymap_nodes(
    doc: &kdl::KdlDocument,
    prefix: &mut Vec<KeyInput>,
    mode_keymap: &mut crate::keymap::ModeKeymap,
    mode_name: &str,
) -> Result<()> {
    for node in doc.nodes() {
        let key_name = node.name().value().to_string();

        let key = match KeyInput::parse(&key_name) {
            Some(k) => k,
            None => bail!("[keymap.{}] unknown key: {}", mode_name, key_name),
        };
        prefix.push(key);

        // If node has children, recurse into them.
        if let Some(children) = node.children() {
            parse_keymap_nodes(children, prefix, mode_keymap, mode_name)?;
        }

        // If node has a positional argument, it's a leaf binding.
        if let Some(entry) = node.get(0) {
            if let Some(action_name) = entry.as_string() {
                let action = Action::from_name(action_name).ok_or_else(|| {
                    anyhow::anyhow!("[keymap.{}] unknown action: {}", mode_name, action_name)
                })?;
                mode_keymap.bind(prefix, action).map_err(|e| {
                    anyhow::anyhow!("[keymap.{}] {}", mode_name, e)
                })?;
            }
        }

        prefix.pop();
    }
    Ok(())
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("astrum").join("config.kdl"))
}

fn bind_all(mode: &mut crate::keymap::ModeKeymap, bindings: &[(&str, Action)]) {
    for (seq, action) in bindings {
        if let Some(keys) = parse_key_sequence(seq) {
            mode.bind(&keys, action.clone())
                .unwrap_or_else(|e| panic!("default keymap bug: {}", e));
        }
    }
}

/// Build the default keymap (vim-style with spacemacs leader).
fn default_keymap() -> Keymap {
    let mut keymap = Keymap::new();
    let normal_bindings: &[(&str, Action)] = &[
        ("h", Action::MoveLeft),
        ("j", Action::MoveDown),
        ("k", Action::MoveUp),
        ("l", Action::MoveRight),
        ("left", Action::MoveLeft),
        ("down", Action::MoveDown),
        ("up", Action::MoveUp),
        ("right", Action::MoveRight),
        ("w", Action::MoveWordForward),
        ("e", Action::MoveWordEnd),
        ("b", Action::MoveWordBackward),
        ("W", Action::MoveBigWordForward),
        ("E", Action::MoveBigWordEnd),
        ("B", Action::MoveBigWordBackward),
        ("0", Action::MoveToLineStart),
        ("$", Action::MoveToLineEnd),
        ("G", Action::MoveToLastLine),
        ("g g", Action::MoveToFirstLine),
        ("C-f", Action::PageDown),
        ("C-b", Action::PageUp),
        ("C-d", Action::HalfPageDown),
        ("C-u", Action::HalfPageUp),
        ("C-e", Action::ScrollDown),
        ("C-y", Action::ScrollUp),
        ("i", Action::EnterInsertMode),
        ("a", Action::EnterInsertModeAppend),
        ("A", Action::EnterInsertModeLineEnd),
        ("I", Action::EnterInsertModeLineStart),
        ("v", Action::EnterVisualMode),
        (":", Action::EnterCommandMode),
        ("x", Action::DeleteCharForward),
        ("o", Action::InsertLineBelow),
        ("O", Action::InsertLineAbove),
        ("u", Action::Undo),
        ("C-r", Action::Redo),
        ("d", Action::OperatorDelete),
        ("D", Action::DeleteToLineEnd),
        ("c", Action::OperatorChange),
        ("C", Action::ChangeToLineEnd),
        ("y", Action::OperatorYank),
        ("Y", Action::YankLine),
        ("p", Action::Paste),
        ("P", Action::PasteBefore),
        ("space f f", Action::OpenFileBrowser),
        ("space f s", Action::SaveBuffer),
        ("space b n", Action::NextBuffer),
        ("space b p", Action::PrevBuffer),
        ("space b d", Action::CloseBuffer),
        ("space q q", Action::QuitAll),
        ("space q Q", Action::ForceQuitAll),
        ("space w /", Action::SplitVertical),
        ("space w -", Action::SplitHorizontal),
        ("space w w", Action::FocusPaneNext),
        ("space w h", Action::FocusPaneLeft),
        ("space w j", Action::FocusPaneDown),
        ("space w k", Action::FocusPaneUp),
        ("space w l", Action::FocusPaneRight),
        ("space w . h", Action::MovePaneLeft),
        ("space w . j", Action::MovePaneDown),
        ("space w . k", Action::MovePaneUp),
        ("space w . l", Action::MovePaneRight),
        ("space w . H", Action::MovePaneLeft),
        ("space w . J", Action::MovePaneDown),
        ("space w . K", Action::MovePaneUp),
        ("space w . L", Action::MovePaneRight),
        ("space w d", Action::ClosePane),
        ("space tab", Action::JumpBack),
        ("space S-tab", Action::JumpForward),
        ("space space", Action::CommandPalette),
        ("/", Action::EnterSearchForward),
        ("?", Action::EnterSearchBackward),
        ("n", Action::SearchNext),
        ("N", Action::SearchPrev),
        ("q", Action::RecordMacro),
        ("@", Action::PlayMacro),
        ("esc", Action::EnterNormalMode),
    ];
    bind_all(&mut keymap.normal, normal_bindings);

    // --- Insert mode ---
    let insert_bindings: &[(&str, Action)] = &[
        ("esc", Action::EnterNormalMode),
        ("C-s", Action::SaveBuffer),
        ("enter", Action::InsertNewline),
        ("backspace", Action::DeleteCharBackward),
        ("C-backspace", Action::DeleteWordBackward),
        ("C-w", Action::DeleteWordBackward),
        ("delete", Action::DeleteCharForward),
        ("left", Action::MoveLeft),
        ("right", Action::MoveRight),
        ("up", Action::MoveUp),
        ("down", Action::MoveDown),
    ];
    bind_all(&mut keymap.insert, insert_bindings);

    // --- Visual mode ---
    let visual_bindings: &[(&str, Action)] = &[
        ("esc", Action::EnterNormalMode),
        ("v", Action::EnterNormalMode),
        ("h", Action::MoveLeft),
        ("j", Action::MoveDown),
        ("k", Action::MoveUp),
        ("l", Action::MoveRight),
        ("left", Action::MoveLeft),
        ("down", Action::MoveDown),
        ("up", Action::MoveUp),
        ("right", Action::MoveRight),
        ("w", Action::MoveWordForward),
        ("e", Action::MoveWordEnd),
        ("b", Action::MoveWordBackward),
        ("W", Action::MoveBigWordForward),
        ("E", Action::MoveBigWordEnd),
        ("B", Action::MoveBigWordBackward),
        ("0", Action::MoveToLineStart),
        ("$", Action::MoveToLineEnd),
        ("g g", Action::MoveToFirstLine),
        ("G", Action::MoveToLastLine),
        ("C-u", Action::HalfPageUp),
        ("C-d", Action::HalfPageDown),
        ("d", Action::VisualDelete),
        ("x", Action::VisualDelete),
        ("y", Action::VisualYank),
        ("c", Action::VisualChange),
        ("s", Action::VisualSurround),
    ];
    bind_all(&mut keymap.visual, visual_bindings);

    // --- Browser navigate mode ---
    let browser_bindings: &[(&str, Action)] = &[
        ("h", Action::MoveLeft),
        ("j", Action::MoveDown),
        ("k", Action::MoveUp),
        ("l", Action::MoveRight),
        ("left", Action::MoveLeft),
        ("down", Action::MoveDown),
        ("up", Action::MoveUp),
        ("right", Action::MoveRight),
        ("G", Action::MoveToLastLine),
        ("g", Action::MoveToFirstLine),
        ("C-d", Action::HalfPageDown),
        ("C-u", Action::HalfPageUp),
        ("q", Action::BrowserClose),
        ("enter", Action::BrowserOpen),
        ("backspace", Action::BrowserParentDir),
        ("-", Action::BrowserParentDir),
        ("^", Action::BrowserParentDir),
        ("/", Action::BrowserFilter),
        ("n", Action::BrowserNewFile),
        ("~", Action::BrowserHome),
        ("space f f", Action::OpenFileBrowser),
        ("space f s", Action::SaveBuffer),
        ("space b n", Action::NextBuffer),
        ("space b p", Action::PrevBuffer),
        ("space b d", Action::CloseBuffer),
        ("space q q", Action::QuitAll),
        ("space q Q", Action::ForceQuitAll),
        ("space w /", Action::SplitVertical),
        ("space w -", Action::SplitHorizontal),
        ("space w w", Action::FocusPaneNext),
        ("space w h", Action::FocusPaneLeft),
        ("space w j", Action::FocusPaneDown),
        ("space w k", Action::FocusPaneUp),
        ("space w l", Action::FocusPaneRight),
        ("space w . h", Action::MovePaneLeft),
        ("space w . j", Action::MovePaneDown),
        ("space w . k", Action::MovePaneUp),
        ("space w . l", Action::MovePaneRight),
        ("space w . H", Action::MovePaneLeft),
        ("space w . J", Action::MovePaneDown),
        ("space w . K", Action::MovePaneUp),
        ("space w . L", Action::MovePaneRight),
        ("space w d", Action::ClosePane),
        ("space tab", Action::JumpBack),
        ("space S-tab", Action::JumpForward),
        ("space space", Action::CommandPalette),
        (":", Action::EnterCommandMode),
    ];
    bind_all(&mut keymap.browser, browser_bindings);

    keymap
}

pub const DEFAULT_CONFIG: &str = r#"// Astrum editor configuration

general {
    theme "base16-eighties.dark"
}

// Keybindings are organized by mode. Each node name is a key, and nesting
// represents key chains. The first argument is the action name.
//
// Key notation:
//   j          single key
//   G          shifted key (uppercase)
//   C-a        ctrl + a
//   M-a        alt + a
//   spc        spacebar
//   esc        escape
//   enter      enter/return
//   tab        tab
//   backspace  backspace
//   delete     delete
//   up         arrow up (also: down, left, right)
//
// Key chains are expressed as nesting:
//   g { g "MoveToFirstLine" }   // press g, then g
//   spc { f { f "OpenFileBrowser" } }

keymap {
    normal {
        // Movement
        h "MoveLeft"
        j "MoveDown"
        k "MoveUp"
        l "MoveRight"
        left "MoveLeft"
        down "MoveDown"
        up "MoveUp"
        right "MoveRight"
        w "MoveWordForward"
        e "MoveWordEnd"
        b "MoveWordBackward"
        W "MoveBigWordForward"
        E "MoveBigWordEnd"
        B "MoveBigWordBackward"
        "0" "MoveToLineStart"
        "$" "MoveToLineEnd"
        G "MoveToLastLine"
        g { g "MoveToFirstLine" }

        // Page movement
        C-f "PageDown"
        C-b "PageUp"
        C-d "HalfPageDown"
        C-u "HalfPageUp"

        // Scroll (viewport moves, cursor stays)
        C-e "ScrollDown"
        C-y "ScrollUp"

        // Mode changes
        i "EnterInsertMode"
        a "EnterInsertModeAppend"
        A "EnterInsertModeLineEnd"
        I "EnterInsertModeLineStart"
        v "EnterVisualMode"
        ":" "EnterCommandMode"

        // Editing
        x "DeleteCharForward"
        o "InsertLineBelow"
        O "InsertLineAbove"
        u "Undo"
        C-r "Redo"
        d "OperatorDelete"
        D "DeleteToLineEnd"
        c "OperatorChange"
        C "ChangeToLineEnd"
        y "OperatorYank"
        Y "YankLine"

        // Clipboard
        p "Paste"
        P "PasteBefore"

        // Search
        "/" "EnterSearchForward"
        "?" "EnterSearchBackward"
        n "SearchNext"
        N "SearchPrev"

        // Leader key (spacemacs-style)
        spc {
            spc "CommandPalette"
            f {
                f "OpenFileBrowser"
                s "SaveBuffer"
            }
            b {
                n "NextBuffer"
                p "PrevBuffer"
                d "CloseBuffer"
            }
            w {
                "/" "SplitVertical"
                "-" "SplitHorizontal"
                w "FocusPaneNext"
                h "FocusPaneLeft"
                j "FocusPaneDown"
                k "FocusPaneUp"
                l "FocusPaneRight"
                "." {
                    h "MovePaneLeft"
                    j "MovePaneDown"
                    k "MovePaneUp"
                    l "MovePaneRight"
                    H "MovePaneLeft"
                    J "MovePaneDown"
                    K "MovePaneUp"
                    L "MovePaneRight"
                }
                d "ClosePane"
            }
            q {
                q "QuitAll"
                Q "ForceQuitAll"
            }
            tab "JumpBack"
            S-tab "JumpForward"
        }
    }

    insert {
        esc "EnterNormalMode"
        C-s "SaveBuffer"
        enter "InsertNewline"
        backspace "DeleteCharBackward"
        delete "DeleteCharForward"
        left "MoveLeft"
        right "MoveRight"
        up "MoveUp"
        down "MoveDown"
    }

    visual {
        esc "EnterNormalMode"
        v "EnterNormalMode"
        h "MoveLeft"
        j "MoveDown"
        k "MoveUp"
        l "MoveRight"
        left "MoveLeft"
        down "MoveDown"
        up "MoveUp"
        right "MoveRight"
        w "MoveWordForward"
        e "MoveWordEnd"
        b "MoveWordBackward"
        W "MoveBigWordForward"
        E "MoveBigWordEnd"
        B "MoveBigWordBackward"
        "0" "MoveToLineStart"
        "$" "MoveToLineEnd"
        g { g "MoveToFirstLine" }
        G "MoveToLastLine"
        C-u "HalfPageUp"
        C-d "HalfPageDown"
        d "VisualDelete"
        x "VisualDelete"
        y "VisualYank"
        c "VisualChange"
        s "VisualSurround"
    }

    browser {
        // Movement (shared with normal mode)
        h "MoveLeft"
        j "MoveDown"
        k "MoveUp"
        l "MoveRight"
        left "MoveLeft"
        down "MoveDown"
        up "MoveUp"
        right "MoveRight"
        G "MoveToLastLine"
        g "MoveToFirstLine"
        C-d "HalfPageDown"
        C-u "HalfPageUp"

        // Browser-specific
        q "BrowserClose"
        enter "BrowserOpen"
        backspace "BrowserParentDir"
        "-" "BrowserParentDir"
        "^" "BrowserParentDir"
        "/" "BrowserFilter"
        n "BrowserNewFile"
        "~" "BrowserHome"
        ":" "EnterCommandMode"

        // Leader keys
        spc {
            spc "CommandPalette"
            f {
                f "OpenFileBrowser"
                s "SaveBuffer"
            }
            b {
                n "NextBuffer"
                p "PrevBuffer"
                d "CloseBuffer"
            }
            w {
                "/" "SplitVertical"
                "-" "SplitHorizontal"
                w "FocusPaneNext"
                h "FocusPaneLeft"
                j "FocusPaneDown"
                k "FocusPaneUp"
                l "FocusPaneRight"
                "." {
                    h "MovePaneLeft"
                    j "MovePaneDown"
                    k "MovePaneUp"
                    l "MovePaneRight"
                    H "MovePaneLeft"
                    J "MovePaneDown"
                    K "MovePaneUp"
                    L "MovePaneRight"
                }
                d "ClosePane"
            }
            q {
                q "QuitAll"
                Q "ForceQuitAll"
            }
            tab "JumpBack"
            S-tab "JumpForward"
        }
    }
}
"#;
