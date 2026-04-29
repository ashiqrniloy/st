use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerOptions {
    pub idle_timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliCommand {
    Client,
    Server(ServerOptions),
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
            "server" | "--server" => {
                let mut idle_timeout_secs = None;

                while let Some(arg) = args.next() {
                    match arg.as_str() {
                        "--idle-timeout-secs" => {
                            let Some(value) = args.next() else {
                                return Err(format!(
                                    "missing value for --idle-timeout-secs\n\n{}",
                                    Self::usage()
                                ));
                            };

                            let parsed = value.parse::<u64>().map_err(|_| {
                                format!(
                                    "invalid value for --idle-timeout-secs: {value}\n\n{}",
                                    Self::usage()
                                )
                            })?;

                            idle_timeout_secs = Some(parsed);
                        }
                        "--no-idle-timeout" => {
                            idle_timeout_secs = None;
                        }
                        unknown => {
                            return Err(format!(
                                "unknown server argument: {unknown}\n\n{}",
                                Self::usage()
                            ));
                        }
                    }
                }

                Ok(Self::Server(ServerOptions { idle_timeout_secs }))
            }
            "quit" | "--quit" => Ok(Self::Quit),
            "-h" | "--help" | "help" => Err(Self::usage()),
            unknown => Err(format!("unknown command: {unknown}\n\n{}", Self::usage())),
        }
    }

    pub fn usage() -> String {
        "Usage:\n  st             Open a client, starting the server if needed\n  st client      Open a client\n  st server      Run the server in the foreground (no idle timeout)\n  st server --idle-timeout-secs <N>\n                 Run server with optional idle shutdown\n  st quit        Ask the server to shut down\n\nCargo examples:\n  cargo run\n  cargo run -- client\n  cargo run -- server\n  cargo run -- server --idle-timeout-secs 300\n  cargo run -- quit".into()
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
        assert!(usage.contains("--idle-timeout-secs"));
        assert!(usage.contains("st quit"));
    }

    #[test]
    fn server_options_can_be_constructed_with_idle_timeout() {
        let options = ServerOptions {
            idle_timeout_secs: Some(300),
        };
        assert_eq!(options.idle_timeout_secs, Some(300));
    }
}
