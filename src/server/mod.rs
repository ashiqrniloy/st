mod client;
mod dispatch;
mod docs;
mod key_chord;
mod key_input;
mod keymap;
mod r#loop;
mod scene;
mod socket_task;
mod state;

pub use r#loop::{request_shutdown, run_foreground};

#[cfg(test)]
mod tests;
