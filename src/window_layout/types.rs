use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

impl SplitAxis {
    pub fn opposite(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneRect {
    pub pane_id: PaneId,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitOutcome {
    pub new_pane: PaneId,
    pub active_pane: PaneId,
    pub pane_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitError {
    TerminalLayout,
    PaneTooSmall { axis: SplitAxis },
}

impl std::fmt::Display for SplitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TerminalLayout => write!(
                f,
                "window layout is terminal: split would exceed the four-pane or same-axis limit"
            ),
            Self::PaneTooSmall { axis } => {
                write!(f, "active split region is too small for a {axis:?} split")
            }
        }
    }
}
