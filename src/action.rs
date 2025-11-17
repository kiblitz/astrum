use crate::import::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Cli {
        name: String,
        command: String,
        args: Vec<String>,
    },
}

impl Action {
    pub fn invoke(&self) {
        match self {
            Action::Cli {
                name,
                command,
                args,
            } => {
                let output = process::Command::new(command)
                    .args(args)
                    .output()
                    .expect(&format!("cli action invocation failed: {}", name,));
                info!("{:?}", output)
            }
        }
    }
}
