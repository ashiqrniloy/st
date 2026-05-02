mod buffer;
mod cursor;
mod selection;
mod state;
mod transaction;

pub use buffer::{BufferId, BufferSnapshot, BufferVersion, TextBuffer};
pub use state::EditorState;

#[cfg(test)]
mod tests;
