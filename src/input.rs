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
    palette_on: prefix_tree::Map<KeyCodeWrapper, action::Action>,
    mode_: Mode,
    cursor: cursor::Cursor,
}

#[derive(Clone, Debug, Deserialize, EnumDiscriminants, Serialize)]
#[strum_discriminants(derive(Serialize, Deserialize))]
pub enum Mode {
    Insert,
    Normal,
    Palette,
    Visual { start: cursor::Cursor },
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
        let palette_on = command_palette.clone();

        Ok(Self {
            command_palette,
            palette_on,
            mode_: Mode::Normal,
            cursor: cursor::Cursor { col: 0, line: 0 },
        })
    }

    pub fn consume_key(&mut self, key: &KeyCode) -> Result<()> {
        self.mode_ = Mode::Palette;
        match self.palette_on.enter(&KeyCodeWrapper(key.clone())) {
            None => self.mode_ = Mode::Normal,
            Some(prefix_tree::Node::Value(action)) => {
                self.mode_ = Mode::Normal;
                self.invoke_action(&action.clone())?;
                self.palette_on = self.command_palette.clone();
            }
            Some(prefix_tree::Node::SubTree(subtree)) => self.palette_on = subtree.clone(),
        }
        Ok(())
    }

    fn invoke_action(&mut self, action: &action::Action) -> Result<()> {
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
                self.set_mode(&mode_);
                Ok(())
            }
        }
    }

    fn set_mode(&mut self, mode_: &ModeDiscriminants) {
        self.mode_ = match mode_ {
            ModeDiscriminants::Insert => Mode::Insert,
            ModeDiscriminants::Normal => Mode::Normal,
            ModeDiscriminants::Palette => Mode::Palette,
            ModeDiscriminants::Visual => Mode::Visual {
                start: self.cursor.clone(),
            },
        };
    }

    fn move_cursor(&mut self, direction: &action::Direction) -> Result<()> {
        todo!()
    }
}
