use crate::configuration::{
    DEFAULT_DWIM_HALF_HEIGHT_ASPECT_RATIO, DEFAULT_DWIM_HALF_HEIGHT_MAX_PX,
    DEFAULT_DWIM_WIDE_ASPECT_RATIO,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DwimSplitThresholds {
    pub wide_full_window_aspect_ratio: f32,
    pub half_height_vertical_aspect_ratio: f32,
    pub half_height_max_px: u32,
}

impl Default for DwimSplitThresholds {
    fn default() -> Self {
        Self {
            wide_full_window_aspect_ratio: DEFAULT_DWIM_WIDE_ASPECT_RATIO,
            half_height_vertical_aspect_ratio: DEFAULT_DWIM_HALF_HEIGHT_ASPECT_RATIO,
            half_height_max_px: DEFAULT_DWIM_HALF_HEIGHT_MAX_PX,
        }
    }
}
