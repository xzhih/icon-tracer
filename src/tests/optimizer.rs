use super::*;

#[test]
fn icon_candidate_selection_uses_global_fit_band() {
    let candidates = vec![
        test_icon_candidate(0.0, 10.0, 100, 100),
        test_icon_candidate(0.0015, 8.0, 80, 80),
        test_icon_candidate(0.003, 1.0, 10, 10),
    ];

    let best_index =
        best_icon_candidate_index(&candidates, false).expect("candidates should exist");

    assert_eq!(best_index, 1);
}

#[test]
fn icon_candidate_selection_can_prefer_simpler_scalar_candidate_for_isolated_icons() {
    let mut exact_subpixel = test_icon_candidate(0.0, 0.0049, 1344, 62);
    exact_subpixel.path_count = 2;
    exact_subpixel.trace_options = TraceOptions {
        contour_mode: ContourMode::Subpixel,
        opt_tolerance: 0.25,
        ..TraceOptions::default()
    };

    let mut simple_scalar = test_icon_candidate(0.0062, 0.0116, 138, 45);
    simple_scalar.metrics.false_negative_ratio = 0.0061;
    simple_scalar.metrics.iou = 0.9938;
    simple_scalar.path_count = 2;
    simple_scalar.trace_options = TraceOptions {
        contour_mode: ContourMode::Scalar,
        opt_tolerance: 0.75,
        ..TraceOptions::default()
    };

    let mut overfit_scalar = test_icon_candidate(0.02, 0.002, 20, 20);
    overfit_scalar.metrics.false_negative_ratio = 0.02;
    overfit_scalar.metrics.iou = 0.98;
    overfit_scalar.trace_options = TraceOptions {
        contour_mode: ContourMode::Scalar,
        opt_tolerance: 6.0,
        ..TraceOptions::default()
    };

    let candidates = vec![exact_subpixel, simple_scalar, overfit_scalar];

    let strict_index =
        best_icon_candidate_index(&candidates, false).expect("candidates should exist");
    let isolated_icon_index =
        best_icon_candidate_index(&candidates, true).expect("candidates should exist");

    assert_eq!(strict_index, 0);
    assert_eq!(isolated_icon_index, 1);
}

fn test_icon_candidate(
    foreground_error_ratio: f64,
    score: f64,
    point_count: usize,
    svg_command_count: usize,
) -> IconOptimizationCandidate {
    IconOptimizationCandidate {
        trace_options: TraceOptions::default(),
        metrics: IconDiffMetrics {
            total_pixels: 1000,
            target_foreground_pixels: 1000,
            candidate_foreground_pixels: 1000,
            true_positive_pixels: 1000,
            false_positive_pixels: 0,
            false_negative_pixels: 0,
            xor_pixels: 0,
            xor_ratio: foreground_error_ratio,
            foreground_error_ratio,
            false_positive_ratio: 0.0,
            false_negative_ratio: 0.0,
            precision: 1.0,
            recall: 1.0,
            iou: 1.0,
        },
        score,
        path_count: 1,
        point_count,
        svg_command_count,
    }
}
