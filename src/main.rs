mod cli;
mod client;
mod commands;
mod configuration;
mod documentation;
mod editor;
mod events;
mod ipc;
mod js_runtime;
mod protocol;
mod render;
mod server;
mod window_layout;
mod workers;

use cli::CliCommand;

fn main() {
    let result = match CliCommand::parse() {
        Ok(CliCommand::Client) => client::run(),
        Ok(CliCommand::Server(options)) => server::run_foreground(options),
        Ok(CliCommand::Quit) => server::request_shutdown(),
        Err(message) => Err(message),
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
