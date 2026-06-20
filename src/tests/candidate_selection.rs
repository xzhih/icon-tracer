use super::*;
use crate::trace::rasterize_path_evenodd;

#[test]
fn pixel_potrace_candidate_selection_rejects_shorter_mask_regression() {
    let path = TracePath {
        is_hole: false,
        points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
    };
    let best = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (10.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 10.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 0.0),
            },
        ],
    );
    let shorter_wrong = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 0.0),
            },
        ],
    );

    assert!(pixel_potrace_candidate_is_better(
        &path,
        None,
        &shorter_wrong,
        &best
    ));
    assert!(!pixel_potrace_candidate_is_better(
        &path,
        Some((12, 12)),
        &shorter_wrong,
        &best
    ));
}

#[test]
fn fitted_candidate_selection_allows_tiny_mask_slack_only() {
    let path = TracePath {
        is_hole: false,
        points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
    };
    let best = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (10.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 10.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 0.0),
            },
        ],
    );
    let close = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Cubic(line_as_cubic((0.0, 0.0), (10.0, 0.0))),
            SvgPathSegment::Cubic(line_as_cubic((10.0, 0.0), (10.0, 10.0))),
            SvgPathSegment::Cubic(line_as_cubic((10.0, 10.0), (0.0, 10.0))),
            SvgPathSegment::Cubic(line_as_cubic((0.0, 10.0), (0.0, 0.0))),
        ],
    );
    let far = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Cubic(line_as_cubic((0.0, 0.0), (10.0, 0.0))),
            SvgPathSegment::Cubic(line_as_cubic((10.0, 0.0), (0.0, 10.0))),
            SvgPathSegment::Cubic(line_as_cubic((0.0, 10.0), (0.0, 0.0))),
        ],
    );

    assert!(pixel_potrace_fitted_candidate_is_close_enough(
        &path,
        Some((12, 12)),
        &close,
        &best
    ));
    assert!(!pixel_potrace_fitted_candidate_is_close_enough(
        &path,
        Some((12, 12)),
        &far,
        &best
    ));
}

#[test]
fn coverage_threshold_rasterizer_matches_integer_square_pixels() {
    let path = TracePath {
        is_hole: false,
        points: vec![(2.0, 2.0), (8.0, 2.0), (8.0, 8.0), (2.0, 8.0)],
    };
    let mut center_sampled = vec![false; 10 * 10];
    let mut coverage_sampled = vec![false; 10 * 10];

    rasterize_path_evenodd(&path, 10, 10, &mut center_sampled);
    rasterize_path_evenodd_coverage_threshold(
        &path,
        10,
        10,
        CANDIDATE_MASK_SUPERSAMPLE,
        &mut coverage_sampled,
    );

    assert_eq!(coverage_sampled, center_sampled);
}

#[test]
fn pixel_trace_can_preserve_collinear_boundary_points() {
    let bitmap =
        Bitmap::from_rows(3, 1, &[true, true, true]).expect("bitmap dimensions should match");
    let simplified = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 0,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: false,
        },
    );
    let preserved = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 0,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );

    assert_eq!(simplified.paths[0].points.len(), 4);
    assert_eq!(preserved.paths[0].points.len(), 8);
}
