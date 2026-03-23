use anyhow::Result;
use astrum::config::Config;
use astrum::editor::Editor;

#[tokio::main]
async fn main() -> Result<()> {
    let (config, config_error) = Config::load();
    let mut editor = Editor::new(config)?;
    if let Some(err) = config_error {
        editor.set_config_error(err);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    for path in &args {
        editor.open_file(path).await?;
    }

    // No args → welcome screen (no scratch buffer created)

    editor.run().await
}
