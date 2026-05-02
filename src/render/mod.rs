mod actions;
mod app;
mod element;
mod input;
mod key;
mod layout;
mod panes;
mod root_view;
mod selection;

use tokio::sync::mpsc as tokio_mpsc;

use crate::events::{EditorEvent, SceneUpdate};

pub struct UiChannels {
    pub ui_tx: tokio_mpsc::UnboundedSender<EditorEvent>,
    pub scene_rx: tokio_mpsc::UnboundedReceiver<SceneUpdate>,
}

pub fn run_ui_with_gpui(channels: UiChannels) -> Result<(), String> {
    app::run_ui_with_gpui(channels)
}

#[cfg(test)]
mod tests;
