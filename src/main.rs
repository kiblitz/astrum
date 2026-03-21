mod action;
mod buffer;
mod config;
mod editor;
mod file_browser;
mod input;
mod keymap;
mod pane;
mod renderer;
mod syntax;

use anyhow::Result;
use config::Config;
use editor::Editor;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    let mut editor = Editor::new(config)?;

    let args: Vec<String> = std::env::args().skip(1).collect();
    for path in &args {
        editor.open_file(path).await?;
    }

    // No args → welcome screen (no scratch buffer created)

    editor.run().await
}
