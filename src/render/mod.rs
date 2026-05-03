mod actions;
mod app;
mod element;
mod input;
mod key;
mod layout;
mod panes;
mod root_view;
mod selection;
pub(crate) mod text_cache;

use tokio::sync::mpsc as tokio_mpsc;

use crate::events::{EditorEvent, ScenePatch, SceneUpdate};

#[derive(Debug, Clone)]
pub enum UiSceneEvent {
    Snapshot(SceneUpdate),
    Patch(ScenePatch),
}

pub struct UiChannels {
    pub ui_tx: tokio_mpsc::UnboundedSender<EditorEvent>,
    pub scene_rx: tokio_mpsc::Receiver<UiSceneEvent>,
}

pub fn run_ui_with_gpui(channels: UiChannels) -> Result<(), String> {
    app::run_ui_with_gpui(channels)
}

#[cfg(test)]
mod tests;
