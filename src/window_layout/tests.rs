use super::{DwimSplitThresholds, PaneId, PaneRect, SplitAxis, SplitError, WindowLayout};

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
        rects.iter().map(|r| (r.width, r.height)).collect::<Vec<_>>(),
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
