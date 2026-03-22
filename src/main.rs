use anyhow::Result;
use astrum::config::Config;
use astrum::editor::Editor;

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
