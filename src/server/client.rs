use tokio::sync::mpsc;

use crate::{
    configuration::{DEFAULT_CLIENT_WINDOW_HEIGHT_PX, DEFAULT_CLIENT_WINDOW_WIDTH_PX},
    events::{PaneScene, Viewport},
    protocol::ServerToClient,
    window_layout::WindowLayout,
};

use super::key_chord::SimpleChord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClientConnectionState {
    Connected,
    Closing,
}

#[derive(Debug)]
pub(super) struct ClientConnection {
    pub(super) state: ClientConnectionState,
    pub(super) tx: mpsc::UnboundedSender<ServerToClient>,
    pub(super) viewport: Viewport,
    pub(super) window_width: u32,
    pub(super) window_height: u32,
    pub(super) layout: WindowLayout,
    pub(super) pending_chord: Vec<SimpleChord>,
}

impl ClientConnection {
    pub(super) fn new(tx: mpsc::UnboundedSender<ServerToClient>) -> Self {
        Self {
            state: ClientConnectionState::Connected,
            tx,
            viewport: Viewport::default(),
            window_width: DEFAULT_CLIENT_WINDOW_WIDTH_PX as u32,
            window_height: DEFAULT_CLIENT_WINDOW_HEIGHT_PX as u32,
            layout: WindowLayout::new(),
            pending_chord: Vec::new(),
        }
    }
}

pub(super) fn pane_scenes(client: &ClientConnection) -> Vec<PaneScene> {
    client
        .layout
        .pane_rects(client.window_width, client.window_height)
        .into_iter()
        .map(|rect| PaneScene {
            pane_id: rect.pane_id,
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            active: rect.active,
        })
        .collect()
}
