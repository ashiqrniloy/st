use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliCommand {
    Client,
    Server,
    Quit,
}

impl CliCommand {
    pub fn parse() -> Result<Self, String> {
        let mut args = env::args().skip(1);

        let Some(command) = args.next() else {
            return Ok(Self::Client);
        };

        match command.as_str() {
            "client" | "--client" => Ok(Self::Client),
            "server" | "--server" => Ok(Self::Server),
            "quit" | "--quit" => Ok(Self::Quit),
            "-h" | "--help" | "help" => Err(Self::usage()),
            unknown => Err(format!("unknown command: {unknown}\n\n{}", Self::usage())),
        }
    }

    pub fn usage() -> String {
        "Usage:\n  st             Open a client, starting the server if needed\n  st client      Open a client\n  st server      Run the server in the foreground\n  st quit        Ask the server to shut down\n\nCargo examples:\n  cargo run\n  cargo run -- client\n  cargo run -- server\n  cargo run -- quit".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_mentions_expected_commands() {
        let usage = CliCommand::usage();
        assert!(usage.contains("st client"));
        assert!(usage.contains("st server"));
        assert!(usage.contains("st quit"));
    }
}
