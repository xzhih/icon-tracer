use icon_tracer::{ContourMode, IconOptimizationCandidate, IconOptimizationResult};

pub fn optimization_report_json(result: &IconOptimizationResult) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"best_candidate\": ");
    push_candidate_json(&mut json, &result.best_candidate, 2);
    json.push_str(",\n  \"candidates\": [\n");

    for (index, candidate) in result.candidates.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }

        json.push_str("    ");
        push_candidate_json(&mut json, candidate, 4);
    }

    json.push_str("\n  ]\n}\n");
    json
}

fn push_candidate_json(json: &mut String, candidate: &IconOptimizationCandidate, indent: usize) {
    let nested = " ".repeat(indent + 2);
    json.push_str("{\n");
    json.push_str(&format!(
        "{nested}\"contour_mode\": \"{}\",\n",
        contour_mode_name(candidate.trace_options.contour_mode)
    ));
    json.push_str(&format!(
        "{nested}\"opt_tolerance\": {:.6},\n",
        candidate.trace_options.opt_tolerance
    ));
    json.push_str(&format!(
        "{nested}\"turd_size\": {},\n",
        candidate.trace_options.turd_size
    ));
    json.push_str(&format!("{nested}\"score\": {:.9},\n", candidate.score));
    json.push_str(&format!(
        "{nested}\"path_count\": {},\n",
        candidate.path_count
    ));
    json.push_str(&format!(
        "{nested}\"point_count\": {},\n",
        candidate.point_count
    ));
    json.push_str(&format!(
        "{nested}\"svg_command_count\": {},\n",
        candidate.svg_command_count
    ));
    json.push_str(&format!("{nested}\"metrics\": {{\n"));

    let metric_indent = " ".repeat(indent + 4);
    let metrics = candidate.metrics;
    json.push_str(&format!(
        "{metric_indent}\"total_pixels\": {},\n",
        metrics.total_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"target_foreground_pixels\": {},\n",
        metrics.target_foreground_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"candidate_foreground_pixels\": {},\n",
        metrics.candidate_foreground_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"true_positive_pixels\": {},\n",
        metrics.true_positive_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"false_positive_pixels\": {},\n",
        metrics.false_positive_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"false_negative_pixels\": {},\n",
        metrics.false_negative_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"xor_pixels\": {},\n",
        metrics.xor_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"xor_ratio\": {:.9},\n",
        metrics.xor_ratio
    ));
    json.push_str(&format!(
        "{metric_indent}\"foreground_error_ratio\": {:.9},\n",
        metrics.foreground_error_ratio
    ));
    json.push_str(&format!(
        "{metric_indent}\"false_positive_ratio\": {:.9},\n",
        metrics.false_positive_ratio
    ));
    json.push_str(&format!(
        "{metric_indent}\"false_negative_ratio\": {:.9},\n",
        metrics.false_negative_ratio
    ));
    json.push_str(&format!(
        "{metric_indent}\"precision\": {:.9},\n",
        metrics.precision
    ));
    json.push_str(&format!(
        "{metric_indent}\"recall\": {:.9},\n",
        metrics.recall
    ));
    json.push_str(&format!("{metric_indent}\"iou\": {:.9}\n", metrics.iou));
    json.push_str(&format!("{nested}}}\n"));
    json.push_str(&format!("{}}}", " ".repeat(indent)));
}

fn contour_mode_name(mode: ContourMode) -> &'static str {
    match mode {
        ContourMode::Pixel => "pixel",
        ContourMode::Subpixel => "subpixel",
        ContourMode::Scalar => "scalar",
    }
}
