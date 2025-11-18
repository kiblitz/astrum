use crate::import::*;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to find path: {config:?}"))]
    BasePathError {
        config: ConfigKind,
    },
    PathStringifyError {
        config: ConfigKind,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Keybindings {
    keybindings: Vec<Keybinding>,
    normal: Vec<KeyCode>,
    insert: Vec<KeyCode>,
    visual: Vec<KeyCode>,
    down: Vec<KeyCode>,
    left: Vec<KeyCode>,
    up: Vec<KeyCode>,
    right: Vec<KeyCode>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Keybinding {
    pub key_sequence: Vec<KeyCode>,
    pub action: action::Action,
}

impl IntoIterator for Keybindings {
    type Item = Keybinding;
    type IntoIter = vec::IntoIter<Keybinding>;

    fn into_iter(self) -> vec::IntoIter<Keybinding> {
        let mut keybindings = self.keybindings;

        keybindings.push(Keybinding {
            key_sequence: self.normal,
            action: action::Action::SetMode(input::Mode::Normal),
        });
        keybindings.push(Keybinding {
            key_sequence: self.insert,
            action: action::Action::SetMode(input::Mode::Insert),
        });
        keybindings.push(Keybinding {
            key_sequence: self.visual,
            action: action::Action::SetMode(input::Mode::Visual),
        });

        keybindings.push(Keybinding {
            key_sequence: self.down,
            action: action::Action::Move(action::Direction::Down),
        });
        keybindings.push(Keybinding {
            key_sequence: self.left,
            action: action::Action::Move(action::Direction::Left),
        });
        keybindings.push(Keybinding {
            key_sequence: self.up,
            action: action::Action::Move(action::Direction::Up),
        });
        keybindings.push(Keybinding {
            key_sequence: self.right,
            action: action::Action::Move(action::Direction::Right),
        });

        keybindings.into_iter()
    }
}

#[derive(Debug, Clone)]
pub enum ConfigKind {
    Keybindings,
}

impl ConfigKind {
    pub fn load<T: DeserializeOwned>(self) -> Result<T> {
        let path = self
            .path()
            .ok_or(Error::BasePathError {
                config: self.clone(),
            })
            .context(ConfigSnafu)?;
        let path_str = path
            .as_path()
            .to_str()
            .ok_or(Error::PathStringifyError {
                config: self.clone(),
            })
            .context(ConfigSnafu)?;
        match self {
            ConfigKind::Keybindings => load_sexp(path_str),
        }
    }

    fn filename<'a>(&self) -> &'a str {
        match self {
            ConfigKind::Keybindings => "key_bindings.sexp",
        }
    }

    fn path(&self) -> Option<PathBuf> {
        let mut path = dirs::config_dir()?;
        path.push(self.filename());
        Some(path)
    }
}

fn load_sexp<T: DeserializeOwned>(filename: &str) -> Result<T> {
    let contents = fs::read_to_string(filename).context(IoSnafu)?;
    serde_lexpr::from_str(&contents).context(SexpSerdeSnafu)
}
