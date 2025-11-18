use crate::import::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Cli(Cli),
    Move(Direction),
    SetMode(input::Mode),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cli {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Direction {
    Down,
    Left,
    Up,
    Right,
}
