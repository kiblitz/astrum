use crate::import::*;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to find path: {cli:?}"))]
    CliInvocationError {
        #[snafu(source(from(io::Error, Rc::new)))]
        source: Rc<io::Error>,
        cli: Cli,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Cli(Cli),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cli {
    name: String,
    command: String,
    args: Vec<String>,
}

impl Action {
    pub fn invoke(&self) -> Result<()> {
        match self {
            Action::Cli(cli) => {
                let output = process::Command::new(&cli.command)
                    .args(&cli.args)
                    .output()
                    .context(CliInvocationSnafu { cli: cli.clone() })
                    .context(ActionSnafu)?;
                info!("{:?}", output);
                Ok(())
            }
        }
    }
}
