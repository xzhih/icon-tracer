use super::*;

#[test]
fn optimal_potrace_polygon_reduces_nearly_straight_stair_steps() {
    let mut points = (0..=12)
        .map(|x| (x as f64, if x % 2 == 0 { 0.0 } else { 0.2 }))
        .collect::<Vec<_>>();
    points.extend([(12.0, 6.0), (0.0, 6.0), (0.0, 0.0)]);

    let polygon = optimal_potrace_polygon_indices(&points);

    assert!(polygon.len() < points.len() / 2, "{polygon:?}");
}

#[test]
fn vertex_adjustment_moves_corner_toward_fitted_line_intersection() {
    let points = vec![
        (0.0, 0.0),
        (1.0, 0.0),
        (2.0, 0.2),
        (2.0, 1.0),
        (2.0, 2.0),
        (0.0, 2.0),
    ];
    let adjusted = adjust_potrace_vertices(&points, &[0, 2, 4, 5], 1.0);

    assert!(
        adjusted[1].1 < points[2].1,
        "corner did not move toward the fitted intersection: {adjusted:?}"
    );
}

#[test]
fn graph_opticurve_merges_compatible_adjacent_curves() {
    let run = vec![
        CubicSegment {
            start: (0.0, 0.0),
            control1: (0.33, 0.0),
            control2: (0.66, 0.0),
            end: (1.0, 0.0),
        },
        CubicSegment {
            start: (1.0, 0.0),
            control1: (1.33, 0.0),
            control2: (1.66, 0.0),
            end: (2.0, 0.0),
        },
    ];

    let optimized = optimize_potrace_curve_run_graph(&run, 0.2);

    assert_eq!(optimized.len(), 1, "{optimized:?}");
}
