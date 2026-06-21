use super::*;
use crate::TracePath;

pub(crate) fn pixel_potrace_quadratic_polygon_candidate_is_better(
    path: &TracePath,
    width: usize,
    height: usize,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 10;
    const MAX_EXTRA_BOUNDARY_ERROR: f64 = 0.03;
    const MAX_EXTRA_FOREGROUND_DELTA: usize = 24;
    const MAX_EXTRA_D_BYTES: usize = 360;
    const MAX_EXTRA_SEGMENTS: usize = 24;

    if !pixel_potrace_candidate_is_concave_line_polygon(best) {
        return false;
    }

    if candidate.1.len() > best.1.len().saturating_add(MAX_EXTRA_SEGMENTS) {
        return false;
    }

    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();
    if candidate_bytes > best_bytes.saturating_add(MAX_EXTRA_D_BYTES) {
        return false;
    }

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    if candidate_error.saturating_add(MIN_MASK_IMPROVEMENT_PIXELS) > best_error {
        return false;
    }

    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);
    if candidate_boundary_error > best_boundary_error + MAX_EXTRA_BOUNDARY_ERROR {
        return false;
    }

    let candidate_delta =
        pixel_potrace_candidate_foreground_delta(path, candidate, width, height).unsigned_abs();
    let best_delta =
        pixel_potrace_candidate_foreground_delta(path, best, width, height).unsigned_abs();
    candidate_delta <= best_delta.saturating_add(MAX_EXTRA_FOREGROUND_DELTA)
}

fn pixel_potrace_candidate_is_concave_line_polygon(
    candidate: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const EPSILON: f64 = 1.0e-6;

    if candidate.1.len() < 5
        || !candidate.1.iter().all(|segment| match segment {
            SvgPathSegment::Line { .. } => true,
            SvgPathSegment::Cubic(cubic) => {
                potrace_cubic_is_nearly_linear(*cubic, PIXEL_POTRACE_LINEAR_DEVIATION)
            }
        })
    {
        return false;
    }

    let mut vertices = Vec::with_capacity(candidate.1.len() + 1);
    vertices.push(candidate.0);
    vertices.extend(candidate.1.iter().map(|segment| segment.end()));
    if vertices.len() > 1
        && distance_squared_float(
            vertices[0],
            *vertices.last().expect("vertices should not be empty"),
        ) <= 1.0e-9
    {
        vertices.pop();
    }

    let mut has_positive_turn = false;
    let mut has_negative_turn = false;
    for index in 0..vertices.len() {
        let previous = vertices[(index + vertices.len() - 1) % vertices.len()];
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];
        let turn = signed_area_twice(previous, current, next);
        if turn > EPSILON {
            has_positive_turn = true;
        } else if turn < -EPSILON {
            has_negative_turn = true;
        }

        if has_positive_turn && has_negative_turn {
            return true;
        }
    }

    false
}
