use super::types::{PaneId, PaneRect, SplitAxis};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PaneNode {
    Leaf(PaneId),
    Split {
        axis: SplitAxis,
        children: Vec<PaneNode>,
    },
}

pub(super) fn split_node(
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
    pub(super) fn pane_count(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Split { children, .. } => children.iter().map(Self::pane_count).sum(),
        }
    }

    pub(super) fn collect_panes(&self, panes: &mut Vec<PaneId>) {
        match self {
            Self::Leaf(id) => panes.push(*id),
            Self::Split { children, .. } => {
                for child in children {
                    child.collect_panes(panes);
                }
            }
        }
    }

    pub(super) fn rects(
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
