use crate::import::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cursor {
    pub col: usize,
    pub line: usize,
}
