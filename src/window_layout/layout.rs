use super::{
    dwim::DwimSplitThresholds,
    tree::{split_node, PaneNode},
    types::{PaneId, PaneRect, SplitAxis, SplitError, SplitOutcome},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowLayout {
    root: PaneNode,
    active_pane: PaneId,
    next_pane_id: u64,
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
