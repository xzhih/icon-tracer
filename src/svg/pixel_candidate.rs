use super::*;
use crate::trace::rasterize_path_evenodd;

pub(crate) fn pixel_potrace_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    if let Some((width, height)) = canvas_size {
        let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
        let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
        let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
        let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);

        return (candidate_error < best_error
            && pixel_potrace_boundary_error_is_acceptable(
                candidate_boundary_error,
                best_boundary_error,
            ))
            || (candidate_error == best_error
                && pixel_potrace_boundary_error_is_acceptable(
                    candidate_boundary_error,
                    best_boundary_error,
                )
                && compact_svg_path_data_from_segments(candidate.0, &candidate.1).len()
                    < compact_svg_path_data_from_segments(best.0, &best.1).len());
    }

    compact_svg_path_data_from_segments(candidate.0, &candidate.1).len()
        < compact_svg_path_data_from_segments(best.0, &best.1).len()
}

pub(crate) fn pixel_potrace_boundary_error_is_acceptable(candidate: f64, best: f64) -> bool {
    const MAX_ABSOLUTE_EXTRA_ERROR: f64 = 0.35;
    const MAX_RELATIVE_EXTRA_ERROR: f64 = 1.15;

    candidate <= (best + MAX_ABSOLUTE_EXTRA_ERROR).max(best * MAX_RELATIVE_EXTRA_ERROR)
}

pub(crate) fn pixel_potrace_fitted_candidate_is_close_enough(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MAX_EXTRA_MASK_PIXELS: usize = 5;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    candidate_error <= best_error.saturating_add(MAX_EXTRA_MASK_PIXELS)
        && candidate.1.len() >= best.1.len()
}

pub(crate) fn pixel_potrace_primitive_candidate_is_close_enough(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_EXTRA_MASK_PIXELS: usize = 8;
    const MAX_EXTRA_MASK_RATIO: f64 = 0.003;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    let budget = MIN_EXTRA_MASK_PIXELS
        .max((width.saturating_mul(height) as f64 * MAX_EXTRA_MASK_RATIO).round() as usize);

    candidate_error <= best_error.saturating_add(budget)
}

pub(crate) fn pixel_potrace_rounded_rect_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_SLACK_PIXELS: usize = 32;
    const MAX_MASK_SLACK_RATIO: f64 = 0.0005;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);
    let slack = MIN_MASK_SLACK_PIXELS
        .max((width.saturating_mul(height) as f64 * MAX_MASK_SLACK_RATIO).round() as usize);

    candidate_error <= best_error.saturating_add(slack)
        && candidate_boundary_error < best_boundary_error
}

pub(crate) fn pixel_potrace_template_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_SLACK_PIXELS: usize = 96;
    const MAX_MASK_SLACK_RATIO: f64 = 0.0015;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);
    let slack = MIN_MASK_SLACK_PIXELS
        .max((width.saturating_mul(height) as f64 * MAX_MASK_SLACK_RATIO).round() as usize);

    candidate_error <= best_error.saturating_add(slack)
        && candidate_boundary_error < best_boundary_error
}

pub(crate) fn pixel_potrace_candidate_mask_error(
    path: &TracePath,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    width: usize,
    height: usize,
) -> usize {
    let mut reference = vec![false; width.saturating_mul(height)];
    let mut candidate_pixels = vec![false; width.saturating_mul(height)];
    rasterize_path_evenodd(path, width, height, &mut reference);

    let candidate_path = TracePath {
        is_hole: path.is_hole,
        points: flattened_potrace_segments(candidate.0, &candidate.1),
    };
    rasterize_path_evenodd(&candidate_path, width, height, &mut candidate_pixels);

    reference
        .iter()
        .zip(candidate_pixels.iter())
        .filter(|(left, right)| left != right)
        .count()
}

pub(crate) fn pixel_potrace_candidate_boundary_rms_error(
    path: &TracePath,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
) -> f64 {
    let reference = closed_polyline_points(&path.points);
    let candidate_points =
        closed_polyline_points(&flattened_potrace_segments(candidate.0, &candidate.1));
    if reference.len() < 2 || candidate_points.len() < 2 {
        return f64::INFINITY;
    }

    let reference_to_candidate = mean_squared_distance_to_polyline(&reference, &candidate_points);
    let candidate_to_reference = mean_squared_distance_to_polyline(&candidate_points, &reference);
    (reference_to_candidate.max(candidate_to_reference)).sqrt()
}

pub(crate) fn mean_squared_distance_to_polyline(
    points: &[(f64, f64)],
    polyline: &[(f64, f64)],
) -> f64 {
    if points.is_empty() || polyline.len() < 2 {
        return f64::INFINITY;
    }

    points
        .iter()
        .map(|point| distance_squared_to_polyline(*point, polyline).0)
        .sum::<f64>()
        / points.len() as f64
}

pub(crate) fn closed_polyline_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut closed = points.to_vec();
    if let (Some(first), Some(last)) = (closed.first().copied(), closed.last().copied()) {
        if distance_squared_float(first, last) > 1.0e-12 {
            closed.push(first);
        }
    }
    closed
}

pub(crate) fn flattened_potrace_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> Vec<(f64, f64)> {
    const CUBIC_FLATTEN_STEPS: usize = 64;

    let mut points = Vec::new();
    points.push(start);

    for segment in segments {
        match segment {
            SvgPathSegment::Line { end, .. } => points.push(*end),
            SvgPathSegment::Cubic(cubic) => {
                for step in 1..=CUBIC_FLATTEN_STEPS {
                    points.push(cubic_point(
                        *cubic,
                        step as f64 / CUBIC_FLATTEN_STEPS as f64,
                    ));
                }
            }
        }
    }

    dedup_nearby_points(points)
}
