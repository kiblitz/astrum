use crate::import::*;

pub fn command() -> Result<()> {
    color_eyre::install().context(ReportSnafu)?;
    let mut terminal = ratatui::init();
    let result = app::App::default().run(&mut terminal);
    ratatui::restore();
    result
}
