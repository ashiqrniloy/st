use tokio::sync::mpsc as tokio_mpsc;

use crate::editor_core::{RenderCommand, UiEvent};

pub mod gpui_backend;

pub struct UiChannels {
    pub ui_tx: tokio_mpsc::UnboundedSender<UiEvent>,
    pub render_rx: tokio_mpsc::UnboundedReceiver<RenderCommand>,
}

pub trait UiRenderer {
    fn run(self, channels: UiChannels) -> Result<(), String>;
}

pub fn run_ui_with_gpui(channels: UiChannels) -> Result<(), String> {
    gpui_backend::GpuiRenderer.run(channels)
}
