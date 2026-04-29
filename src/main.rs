mod cli;
mod client;
mod editor;
mod events;
mod ipc;
mod js_runtime;
mod protocol;
mod render;
mod server;

use cli::CliCommand;

fn main() {
    let result = match CliCommand::parse() {
        Ok(CliCommand::Client) => client::run(),
        Ok(CliCommand::Server) => server::run_foreground(),
        Ok(CliCommand::Quit) => server::request_shutdown(),
        Err(message) => Err(message),
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
