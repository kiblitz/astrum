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

pub type Keybindings = Vec<Keybinding>;

#[derive(Debug, Deserialize, Serialize)]
pub struct Keybinding {
    pub key_sequence: Vec<KeyCode>,
    pub action: action::Action,
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
