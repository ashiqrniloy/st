use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderCommand {
    DrawRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: u32,
    },
}
