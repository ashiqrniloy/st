use serde::{Deserialize, Serialize};

use crate::configuration::{
    DEFAULT_DWIM_HALF_HEIGHT_ASPECT_RATIO, DEFAULT_DWIM_HALF_HEIGHT_MAX_PX,
    DEFAULT_DWIM_WIDE_ASPECT_RATIO,
};

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
enum PaneNode {
    Leaf(PaneId),
    Split {
        axis: SplitAxis,
        children: Vec<PaneNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowLayout {
    root: PaneNode,
    active_pane: PaneId,
    next_pane_id: u64,
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

impl Default for WindowLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowLayout {
    pub fn new() -> Self {
        let first = PaneId(1);
        Self {
            root: PaneNode::Leaf(first),
            active_pane: first,
            next_pane_id: 2,
        }
    }

    pub fn active_pane(&self) -> PaneId {
        self.active_pane
    }

    pub fn pane_count(&self) -> usize {
        self.root.pane_count()
    }

    pub fn pane_order(&self) -> Vec<PaneId> {
        let mut panes = Vec::new();
        self.root.collect_panes(&mut panes);
        panes
    }

    pub fn activate_pane(&mut self, pane: PaneId) -> Result<(), String> {
        if self.pane_order().contains(&pane) {
            self.active_pane = pane;
            Ok(())
        } else {
            Err(format!("unknown pane id: {}", pane.0))
        }
    }

    pub fn split(
        &mut self,
        axis: SplitAxis,
        window_width: u32,
        window_height: u32,
    ) -> Result<SplitOutcome, SplitError> {
        if !self.can_split_geometry(axis, window_width, window_height) {
            return Err(SplitError::PaneTooSmall { axis });
        }

        let new_pane = PaneId(self.next_pane_id);
        let active = self.active_pane;
        let split =
            split_node(&mut self.root, axis, active, new_pane).ok_or(SplitError::TerminalLayout)?;
        self.next_pane_id += 1;
        self.active_pane = split;
        Ok(SplitOutcome {
            new_pane: split,
            active_pane: self.active_pane,
            pane_count: self.pane_count(),
        })
    }

    pub fn split_dwim(
        &mut self,
        window_width: u32,
        window_height: u32,
        thresholds: DwimSplitThresholds,
    ) -> Result<SplitOutcome, SplitError> {
        let axis = self.dwim_axis(window_width, window_height, thresholds);
        self.split(axis, window_width, window_height)
    }

    pub fn dwim_axis(&self, width: u32, height: u32, thresholds: DwimSplitThresholds) -> SplitAxis {
        if let Some(root_axis) = self.root_axis() {
            if self.is_mixed_candidate() {
                return root_axis.opposite();
            }
            return root_axis;
        }

        let aspect = width as f32 / height.max(1) as f32;
        if height <= thresholds.half_height_max_px
            && aspect >= thresholds.half_height_vertical_aspect_ratio
        {
            SplitAxis::Vertical
        } else if aspect >= thresholds.wide_full_window_aspect_ratio {
            SplitAxis::Horizontal
        } else {
            SplitAxis::Horizontal
        }
    }

    pub fn pane_rects(&self, width: u32, height: u32) -> Vec<PaneRect> {
        let mut rects = Vec::new();
        self.root
            .rects(0, 0, width, height, self.active_pane, &mut rects);
        rects
    }

    fn root_axis(&self) -> Option<SplitAxis> {
        match &self.root {
            PaneNode::Split { axis, .. } => Some(*axis),
            PaneNode::Leaf(_) => None,
        }
    }

    fn is_mixed_candidate(&self) -> bool {
        matches!(&self.root, PaneNode::Split { children, .. } if children.len() == 2)
            && self.pane_count() < 4
    }

    fn can_split_geometry(&self, axis: SplitAxis, width: u32, height: u32) -> bool {
        match axis {
            SplitAxis::Horizontal => height >= 2,
            SplitAxis::Vertical => width >= 2,
        }
    }
}

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

fn split_node(
    root: &mut PaneNode,
    axis: SplitAxis,
    active: PaneId,
    new_pane: PaneId,
) -> Option<PaneId> {
    match root {
        PaneNode::Leaf(existing) => {
            let old = *existing;
            *root = PaneNode::Split {
                axis,
                children: vec![PaneNode::Leaf(old), PaneNode::Leaf(new_pane)],
            };
            Some(new_pane)
        }
        PaneNode::Split {
            axis: root_axis,
            children,
        } if *root_axis == axis => {
            if children
                .iter()
                .all(|child| matches!(child, PaneNode::Leaf(_)))
                && children.len() == 2
            {
                children.push(PaneNode::Leaf(new_pane));
                Some(new_pane)
            } else {
                None
            }
        }
        PaneNode::Split {
            axis: root_axis,
            children,
        } => {
            if children.len() != 2 || axis != root_axis.opposite() {
                return None;
            }

            let target = children
                .iter()
                .position(|child| matches!(child, PaneNode::Leaf(id) if *id == active))
                .or_else(|| {
                    children
                        .iter()
                        .position(|child| matches!(child, PaneNode::Leaf(_)))
                })?;

            if let PaneNode::Leaf(old) = children[target] {
                children[target] = PaneNode::Split {
                    axis,
                    children: vec![PaneNode::Leaf(old), PaneNode::Leaf(new_pane)],
                };
                Some(new_pane)
            } else {
                None
            }
        }
    }
}

impl PaneNode {
    fn pane_count(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Split { children, .. } => children.iter().map(Self::pane_count).sum(),
        }
    }

    fn collect_panes(&self, panes: &mut Vec<PaneId>) {
        match self {
            Self::Leaf(id) => panes.push(*id),
            Self::Split { children, .. } => {
                for child in children {
                    child.collect_panes(panes);
                }
            }
        }
    }

    fn rects(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        active: PaneId,
        out: &mut Vec<PaneRect>,
    ) {
        match self {
            Self::Leaf(pane_id) => out.push(PaneRect {
                pane_id: *pane_id,
                x,
                y,
                width,
                height,
                active: *pane_id == active,
            }),
            Self::Split { axis, children } => {
                let count = children.len() as u32;
                for (idx, child) in children.iter().enumerate() {
                    let idx = idx as u32;
                    match axis {
                        SplitAxis::Horizontal => {
                            let y0 = y + height * idx / count;
                            let y1 = y + height * (idx + 1) / count;
                            child.rects(x, y0, width, y1.saturating_sub(y0), active, out);
                        }
                        SplitAxis::Vertical => {
                            let x0 = x + width * idx / count;
                            let x1 = x + width * (idx + 1) / count;
                            child.rects(x0, y, x1.saturating_sub(x0), height, active, out);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(rects: &[PaneRect]) -> Vec<u64> {
        rects.iter().map(|r| r.pane_id.0).collect()
    }

    #[test]
    fn full_window_splits_to_horizontal_and_vertical_pairs() {
        let mut layout = WindowLayout::new();
        layout.split(SplitAxis::Horizontal, 80, 24).unwrap();
        assert_eq!(layout.pane_count(), 2);
        assert_eq!(layout.pane_rects(80, 24)[0].height, 12);

        let mut layout = WindowLayout::new();
        layout.split(SplitAxis::Vertical, 80, 24).unwrap();
        assert_eq!(layout.pane_count(), 2);
        assert_eq!(layout.pane_rects(80, 24)[0].width, 40);
    }

    #[test]
    fn same_axis_reaches_three_then_rejects() {
        let mut layout = WindowLayout::new();
        layout.split(SplitAxis::Horizontal, 90, 25).unwrap();
        layout.split(SplitAxis::Horizontal, 90, 25).unwrap();
        assert_eq!(layout.pane_count(), 3);
        assert!(matches!(
            layout.split(SplitAxis::Horizontal, 90, 25),
            Err(SplitError::TerminalLayout)
        ));
        assert!(matches!(
            layout.split(SplitAxis::Vertical, 90, 25),
            Err(SplitError::TerminalLayout)
        ));
        assert_eq!(ids(&layout.pane_rects(90, 25)), vec![1, 2, 3]);
        assert_eq!(layout.pane_rects(90, 25)[0].height, 8);
        assert_eq!(layout.pane_rects(90, 25)[2].height, 9);
    }

    #[test]
    fn mixed_splits_prefer_active_then_next_unsplit_pane() {
        let mut layout = WindowLayout::new();
        layout.split(SplitAxis::Horizontal, 100, 40).unwrap();
        layout.activate_pane(PaneId(1)).unwrap();
        layout.split(SplitAxis::Vertical, 100, 40).unwrap();
        assert_eq!(layout.pane_count(), 3);
        assert_eq!(layout.active_pane(), PaneId(3));
        layout.split(SplitAxis::Vertical, 100, 40).unwrap();
        assert_eq!(layout.pane_count(), 4);
        assert_eq!(layout.active_pane(), PaneId(4));
        assert!(layout.split(SplitAxis::Vertical, 100, 40).is_err());
    }

    #[test]
    fn vertical_root_can_be_filled_with_horizontal_splits() {
        let mut layout = WindowLayout::new();
        layout.split(SplitAxis::Vertical, 100, 40).unwrap();
        layout.activate_pane(PaneId(1)).unwrap();
        layout.split(SplitAxis::Horizontal, 100, 40).unwrap();
        layout.split(SplitAxis::Horizontal, 100, 40).unwrap();
        assert_eq!(layout.pane_count(), 4);
        let rects = layout.pane_rects(101, 41);
        assert_eq!(
            rects
                .iter()
                .map(|r| (r.width, r.height))
                .collect::<Vec<_>>(),
            vec![(50, 20), (50, 21), (51, 20), (51, 21)]
        );
    }

    #[test]
    fn dwim_matches_required_dimension_profiles() {
        let thresholds = DwimSplitThresholds::default();
        let mut wide = WindowLayout::new();
        assert_eq!(wide.dwim_axis(1600, 900, thresholds), SplitAxis::Horizontal);
        wide.split_dwim(1600, 900, thresholds).unwrap();
        assert_eq!(wide.dwim_axis(1600, 900, thresholds), SplitAxis::Vertical);
        wide.split_dwim(1600, 900, thresholds).unwrap();
        wide.split_dwim(1600, 900, thresholds).unwrap();
        assert_eq!(wide.pane_count(), 4);
        assert!(wide.split_dwim(1600, 900, thresholds).is_err());

        let mut half_height = WindowLayout::new();
        assert_eq!(
            half_height.dwim_axis(900, 500, thresholds),
            SplitAxis::Vertical
        );
        half_height.split_dwim(900, 500, thresholds).unwrap();
        half_height.split_dwim(900, 500, thresholds).unwrap();
        assert_eq!(half_height.pane_count(), 3);

        let tall = WindowLayout::new();
        assert_eq!(tall.dwim_axis(500, 900, thresholds), SplitAxis::Horizontal);
    }

    #[test]
    fn rejects_too_small_windows_without_mutating_layout() {
        let mut layout = WindowLayout::new();
        let err = layout.split(SplitAxis::Vertical, 1, 10).unwrap_err();
        assert_eq!(
            err,
            SplitError::PaneTooSmall {
                axis: SplitAxis::Vertical
            }
        );
        assert_eq!(layout.pane_count(), 1);
    }
}
