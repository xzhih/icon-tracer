use super::*;

#[test]
fn icon_candidate_selection_uses_global_fit_band() {
    let candidates = vec![
        test_icon_candidate(0.0, 10.0, 100, 100),
        test_icon_candidate(0.0015, 8.0, 80, 80),
        test_icon_candidate(0.003, 1.0, 10, 10),
    ];

    let best_index = best_icon_candidate_index(&candidates).expect("candidates should exist");

    assert_eq!(best_index, 1);
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
