use super::*;

#[test]
fn potrace_segment_cleanup_removes_tiny_spike_between_long_curves() {
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (10.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((10.0, 0.0), (20.0, 0.0))),
        SvgPathSegment::Cubic(CubicSegment {
            start: (20.0, 0.0),
            control1: (19.9, 0.0),
            control2: (18.6, -0.9),
            end: (18.4, -1.2),
        }),
        SvgPathSegment::Cubic(test_cubic((18.4, -1.2), (30.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((30.0, 0.0), (40.0, 0.0))),
    ];

    let pruned = prune_tiny_potrace_curve_segments(segments);

    assert_eq!(pruned.len(), 4);
}

#[test]
fn potrace_segment_cleanup_removes_tiny_spike_at_closed_start() {
    let segments = vec![
        SvgPathSegment::Cubic(CubicSegment {
            start: (0.0, 0.0),
            control1: (0.0, -0.4),
            control2: (0.0, -1.2),
            end: (0.0, -1.8),
        }),
        SvgPathSegment::Cubic(test_cubic((0.0, -1.8), (12.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((12.0, 0.0), (12.0, 12.0))),
        SvgPathSegment::Cubic(test_cubic((12.0, 12.0), (-12.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((-12.0, 0.0), (0.0, 0.0))),
    ];

    let pruned = prune_tiny_potrace_curve_segments(segments.clone());
    let cleaned = cleanup_potrace_segments(segments, PIXEL_POTRACE_LINEAR_DEVIATION);
    let start = cleanup_potrace_start((0.0, 0.0), &cleaned);

    assert_eq!(pruned.len(), 4);
    assert_eq!(pruned[0].start(), (0.0, -1.8));
    assert_eq!(start, cleaned[0].start());
}

#[test]
fn potrace_segment_cleanup_snaps_near_axis_lines_continuously() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (10.0, 0.4),
        },
        SvgPathSegment::Cubic(line_as_cubic((10.0, 0.4), (10.0, 10.0))),
        SvgPathSegment::Line {
            start: (10.0, 10.0),
            end: (0.0, 9.8),
        },
        SvgPathSegment::Cubic(line_as_cubic((0.0, 9.8), (0.0, 0.0))),
    ];

    let snapped = snap_near_axis_potrace_lines(segments);

    assert_eq!(snapped[0].start(), (0.0, 0.2));
    assert_eq!(snapped[0].end(), (10.0, 0.2));
    assert_eq!(snapped[1].start(), (10.0, 0.2));
    assert_eq!(snapped[1].end(), (10.0, 9.9));
    assert_eq!(snapped[2].start(), (10.0, 9.9));
    assert_eq!(snapped[2].end(), (0.0, 9.9));
    assert_eq!(snapped[3].start(), (0.0, 9.9));
}

#[test]
fn potrace_segment_cleanup_demotes_nearly_linear_cubics() {
    let segments = [
        SvgPathSegment::Cubic(CubicSegment {
            start: (0.0, 0.0),
            control1: (33.0, 0.8),
            control2: (66.0, -0.8),
            end: (100.0, 0.0),
        }),
        SvgPathSegment::Cubic(CubicSegment {
            start: (100.0, 0.0),
            control1: (100.0, 40.0),
            control2: (140.0, 40.0),
            end: (140.0, 0.0),
        }),
    ];

    let strict_cleaned =
        demote_nearly_linear_potrace_cubics(segments.to_vec(), STRICT_POTRACE_LINEAR_DEVIATION);
    let pixel_cleaned =
        demote_nearly_linear_potrace_cubics(segments.to_vec(), PIXEL_POTRACE_LINEAR_DEVIATION);

    assert!(matches!(strict_cleaned[0], SvgPathSegment::Cubic(_)));
    assert!(matches!(pixel_cleaned[0], SvgPathSegment::Line { .. }));
    assert!(matches!(pixel_cleaned[1], SvgPathSegment::Cubic(_)));
}

#[test]
fn potrace_segment_cleanup_merges_adjacent_collinear_lines() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (10.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (10.0, 0.0),
            end: (20.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (20.0, 0.0),
            end: (20.0, 10.0),
        },
    ];

    let merged = merge_collinear_potrace_lines(segments);

    assert_eq!(merged.len(), 2, "{merged:?}");
    assert!(matches!(
        merged[0],
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (20.0, 0.0)
        }
    ));
}

#[test]
fn potrace_segment_cleanup_keeps_reversing_collinear_lines() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (10.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (10.0, 0.0),
            end: (0.0, 0.0),
        },
    ];

    let merged = merge_collinear_potrace_lines(segments);

    assert_eq!(merged.len(), 2, "{merged:?}");
}

#[test]
fn potrace_segment_cleanup_reruns_curve_optimization_after_linear_demotion() {
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (1.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((1.0, 0.0), (2.0, 0.0))),
        SvgPathSegment::Cubic(line_as_cubic((2.0, 0.0), (30.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((30.0, 0.0), (31.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((31.0, 0.0), (32.0, 0.0))),
        SvgPathSegment::Cubic(line_as_cubic((32.0, 0.0), (0.0, 0.0))),
    ];

    let (_, optimized) =
        finish_potrace_segments((0.0, 0.0), segments, 0.2, STRICT_POTRACE_LINEAR_DEVIATION);
    let cubic_count = optimized
        .iter()
        .filter(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
        .count();
    let line_count = optimized
        .iter()
        .filter(|segment| matches!(segment, SvgPathSegment::Line { .. }))
        .count();

    assert_eq!(cubic_count, 2, "{optimized:?}");
    assert_eq!(line_count, 2, "{optimized:?}");
}

#[test]
fn bezier_tangent_parameter_handles_linear_degenerate_case() {
    let cubic = CubicSegment {
        start: (0.0, 0.0),
        control1: (1.0, 1.0),
        control2: (2.0, 1.0),
        end: (3.0, 0.0),
    };

    let parameter = bezier_tangent_parameter(cubic, (0.0, 0.0), (1.0, 0.0))
        .expect("linear tangent equation should have an in-range solution");

    assert!((parameter - 0.5).abs() <= 1.0e-9);
}

#[test]
fn regularize_potrace_orthogonal_corner_uses_tangent_controls() {
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (100.0, 0.0))),
        SvgPathSegment::Cubic(CubicSegment {
            start: (100.0, 0.0),
            control1: (104.0, 0.2),
            control2: (109.8, 5.5),
            end: (110.0, 10.0),
        }),
        SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (40.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((40.0, 90.0), (0.0, 0.0))),
    ];

    let regularized = regularize_potrace_orthogonal_corners(segments);
    let SvgPathSegment::Cubic(corner) = regularized[1] else {
        panic!("corner should remain cubic: {regularized:?}");
    };

    assert_eq!(regularized.len(), 5);
    assert!(
        (corner.control1.1 - corner.start.1).abs() <= 1.0e-6,
        "{corner:?}"
    );
    assert!(
        (corner.control2.0 - corner.end.0).abs() <= 1.0e-6,
        "{corner:?}"
    );
}

#[test]
fn regularize_potrace_orthogonal_corner_merges_straight_lead_in() {
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (100.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((100.0, 0.0), (120.0, 0.0))),
        SvgPathSegment::Cubic(CubicSegment {
            start: (120.0, 0.0),
            control1: (124.0, 0.5),
            control2: (130.0, 6.0),
            end: (130.0, 12.0),
        }),
        SvgPathSegment::Cubic(test_cubic((130.0, 12.0), (130.0, 92.0))),
        SvgPathSegment::Cubic(test_cubic((130.0, 92.0), (0.0, 92.0))),
    ];

    let regularized = regularize_potrace_orthogonal_corners(segments);
    let SvgPathSegment::Cubic(corner) = regularized[1] else {
        panic!("merged corner should be cubic: {regularized:?}");
    };

    assert_eq!(regularized.len(), 4);
    assert_eq!(corner.start, (100.0, 0.0));
    assert_eq!(corner.end, (130.0, 12.0));
}

#[test]
fn regularize_potrace_orthogonal_corner_handles_closed_start_boundary() {
    let segments = vec![
        SvgPathSegment::Cubic(CubicSegment {
            start: (100.0, 0.0),
            control1: (104.0, 0.2),
            control2: (109.8, 5.5),
            end: (110.0, 10.0),
        }),
        SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (20.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((20.0, 90.0), (20.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((20.0, 0.0), (100.0, 0.0))),
    ];

    let regularized = regularize_potrace_orthogonal_corners(segments);
    let corner = regularized
        .iter()
        .find_map(|segment| match *segment {
            SvgPathSegment::Cubic(cubic)
                if cubic.start == (100.0, 0.0) && cubic.end == (110.0, 10.0) =>
            {
                Some(cubic)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("wrapped corner should remain cubic: {regularized:?}"));

    assert_eq!(regularized.len(), 5);
    assert!(
        (corner.control1.1 - corner.start.1).abs() <= 1.0e-6,
        "{corner:?}"
    );
    assert!(
        (corner.control2.0 - corner.end.0).abs() <= 1.0e-6,
        "{corner:?}"
    );
}

#[test]
fn regularize_potrace_orthogonal_corner_rejects_beveled_turn() {
    let bevel = test_cubic((100.0, 0.0), (110.0, 10.0));
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (100.0, 0.0))),
        SvgPathSegment::Cubic(bevel),
        SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (40.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((40.0, 90.0), (0.0, 0.0))),
    ];

    let regularized = regularize_potrace_orthogonal_corners(segments);
    let SvgPathSegment::Cubic(unchanged) = regularized[1] else {
        panic!("bevel should remain cubic: {regularized:?}");
    };

    assert_eq!(regularized.len(), 5);
    assert_eq!(unchanged.control1, bevel.control1);
    assert_eq!(unchanged.control2, bevel.control2);
}

#[test]
fn regularize_potrace_orthogonal_corner_ignores_mixed_line_boundaries() {
    let corner = CubicSegment {
        start: (100.0, 0.0),
        control1: (104.0, 0.2),
        control2: (109.8, 5.5),
        end: (110.0, 10.0),
    };
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (100.0, 0.0),
        },
        SvgPathSegment::Cubic(corner),
        SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (40.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((40.0, 90.0), (0.0, 0.0))),
    ];

    let regularized = regularize_potrace_orthogonal_corners(segments);
    let SvgPathSegment::Cubic(unchanged) = regularized[1] else {
        panic!("mixed line boundary should keep the corner cubic: {regularized:?}");
    };

    assert_eq!(regularized.len(), 5);
    assert_eq!(unchanged.control1, corner.control1);
    assert_eq!(unchanged.control2, corner.control2);
}

fn test_cubic(start: (f64, f64), end: (f64, f64)) -> CubicSegment {
    CubicSegment {
        start,
        control1: (
            start.0 + (end.0 - start.0) / 3.0,
            start.1 + (end.1 - start.1) / 3.0,
        ),
        control2: (
            start.0 + (end.0 - start.0) * 2.0 / 3.0,
            start.1 + (end.1 - start.1) * 2.0 / 3.0,
        ),
        end,
    }
}
