use crate::import::*;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to find path: {cli:?}"))]
    CliInvocationError {
        #[snafu(source(from(io::Error, Rc::new)))]
        source: Rc<io::Error>,
        cli: action::Cli,
    },
}

#[derive(Debug)]
pub struct Input {
    command_palette: prefix_tree::Map<KeyCodeWrapper, action::Action>,
    mode_: Mode,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Mode {
    Insert,
    Normal,
    Visual,
}

// We need to do this ugly conversion because [crossterm::event::KeyCode] doesn't derive [Ord].
#[derive(Clone, Debug, Hash, Eq, PartialEq, PartialOrd)]
pub struct KeyCodeWrapper(pub KeyCode);

impl Ord for KeyCodeWrapper {
    fn cmp(self: &Self, self2: &Self) -> Ordering {
        self.partial_cmp(self2).unwrap()
    }
}

impl Input {
    pub fn load() -> Result<Self> {
        let keybindings: config::Keybindings = config::ConfigKind::Keybindings.load()?;

        let command_palette = keybindings.into_iter().fold(
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

        Ok(Self {
            command_palette,
            mode_: Mode::Normal,
        })
    }

    fn invoke_action(&mut self, action: action::Action) -> Result<()> {
        match action {
            action::Action::Cli(cli) => {
                let output = process::Command::new(&cli.command)
                    .args(&cli.args)
                    .output()
                    .context(CliInvocationSnafu { cli: cli.clone() })
                    .context(InputSnafu)?;
                info!("{:?}", output);
                Ok(())
            }
            action::Action::Move(direction) => self.move_cursor(&direction),
            action::Action::SetMode(mode_) => {
                self.mode_ = mode_;
                Ok(())
            }
        }
    }

    fn move_cursor(&mut self, direction: &action::Direction) -> Result<()> {
        todo!()
    }
}
