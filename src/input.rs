use crate::import::*;

// We need to do this ugly conversion because [crossterm::event::KeyCode] doesn't derive [Ord].
#[derive(Clone, Debug, Hash, Eq, PartialEq, PartialOrd)]
pub struct KeyCodeWrapper(pub KeyCode);

impl Ord for KeyCodeWrapper {
    fn cmp(self: &Self, self2: &Self) -> Ordering {
        self.partial_cmp(self2).unwrap()
    }
}

#[derive(Debug)]
pub struct Input {
    command_palette: prefix_tree::Map<KeyCodeWrapper, action::Action>,
}

impl Input {
    pub fn load() -> Result<Self> {
        let keybindings: config::Keybindings = config::ConfigKind::Keybindings.load()?;

        let command_palette = keybindings.iter().fold(
            Ok(prefix_tree::Map::new()),
            |command_palette,
             config::Keybinding {
                 key_sequence,
                 action,
             }| {
                let key_sequence: Vec<KeyCodeWrapper> = key_sequence
                    .into_iter()
                    .map(|key_code| KeyCodeWrapper(key_code.clone()))
                    .collect();
                command_palette?.insert(&key_sequence, action.clone())
            },
        )?;

        Ok(Self { command_palette })
    }
}
