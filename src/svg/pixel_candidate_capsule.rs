use super::*;
use crate::TracePath;

pub(crate) fn pixel_potrace_diagonal_capsule_fine_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 2;
    const MAX_EXTRA_D_BYTES: usize = 8;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if fit_closed_diagonal_capsule_potrace_segments(&path.points).is_none() {
        return false;
    }

    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();
    if candidate.1.len() != best.1.len() {
        return false;
    }

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
    candidate_boundary_error < best_boundary_error
}

pub(crate) fn pixel_potrace_diagonal_capsule_template_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MAX_EXTRA_MASK_PIXELS: usize = 280;
    const MAX_BOUNDARY_ERROR: f64 = 0.75;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if !diagonal_capsule_prefers_thick_low_angle_template(&path.points) {
        return false;
    }

    if candidate.1.len() != 8
        || !candidate
            .1
            .iter()
            .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        return false;
    }

    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();
    if candidate_bytes > best_bytes {
        return false;
    }

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    if candidate_error > best_error.saturating_add(MAX_EXTRA_MASK_PIXELS) {
        return false;
    }

    pixel_potrace_candidate_boundary_rms_error(path, candidate) <= MAX_BOUNDARY_ERROR
}

pub(crate) fn pixel_potrace_diagonal_capsule_best_area_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 4;
    const MIN_SEGMENT_SAVINGS: usize = 2;
    const MAX_EXTRA_BOUNDARY_ERROR: f64 = 0.02;
    const MAX_EXTRA_FOREGROUND_DELTA: usize = 40;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if fit_closed_diagonal_capsule_potrace_segments(&path.points).is_none() {
        return false;
    }

    if candidate.1.len().saturating_add(MIN_SEGMENT_SAVINGS) != best.1.len() {
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

pub(crate) fn pixel_potrace_quadratic_vertex_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_SEGMENT_GROWTH: usize = 4;
    const MAX_SEGMENT_GROWTH: usize = 8;
    const MAX_EXTRA_D_BYTES: usize = 96;
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 12;
    const MAX_LOW_ANGLE_PRIMITIVE_MASK_PIXELS: usize = 360;
    const MAX_EXTRA_BOUNDARY_FOR_MASK_RESCUE: f64 = 0.02;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if pixel_potrace_points_are_detailed_annular_sector(&path.points, width, height) {
        return false;
    }

    if !pixel_potrace_points_match_quadratic_vertex_capsule_rescue(&path.points) {
        if pixel_potrace_points_match_quadratic_vertex_medium_capsule_rescue(&path.points) {
            return pixel_potrace_quadratic_medium_capsule_candidate_is_better(
                path, width, height, candidate, best,
            );
        }

        return pixel_potrace_quadratic_polygon_candidate_is_better(
            path, width, height, candidate, best,
        );
    }

    let segment_growth = candidate.1.len().saturating_sub(best.1.len());
    if !(MIN_SEGMENT_GROWTH..=MAX_SEGMENT_GROWTH).contains(&segment_growth) {
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
    if best.1.len() == 6
        && best
            .1
            .iter()
            .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
        && candidate
            .1
            .iter()
            .any(|segment| matches!(segment, SvgPathSegment::Line { .. }))
        && best_error <= MAX_LOW_ANGLE_PRIMITIVE_MASK_PIXELS
    {
        return false;
    }

    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);

    candidate_error.saturating_add(MIN_MASK_IMPROVEMENT_PIXELS) <= best_error
        && candidate_boundary_error <= best_boundary_error + MAX_EXTRA_BOUNDARY_FOR_MASK_RESCUE
}

fn pixel_potrace_quadratic_medium_capsule_candidate_is_better(
    path: &TracePath,
    width: usize,
    height: usize,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 12;
    const MAX_EXTRA_D_BYTES: usize = 96;
    const MAX_EXTRA_FOREGROUND_DELTA: usize = 32;

    if candidate.1.len() != best.1.len() {
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
    if candidate_boundary_error >= best_boundary_error {
        return false;
    }

    let candidate_delta =
        pixel_potrace_candidate_foreground_delta(path, candidate, width, height).unsigned_abs();
    let best_delta =
        pixel_potrace_candidate_foreground_delta(path, best, width, height).unsigned_abs();
    candidate_delta <= best_delta.saturating_add(MAX_EXTRA_FOREGROUND_DELTA)
}

fn pixel_potrace_points_match_quadratic_vertex_capsule_rescue(points: &[(f64, f64)]) -> bool {
    const MIN_AXIS_ANGLE_DEGREES: f64 = 25.0;
    const MAX_AXIS_ANGLE_DEGREES: f64 = 36.0;

    if fit_closed_diagonal_capsule_potrace_segments(points).is_none() {
        return false;
    }

    let origin = arc_centroid(points);
    let Some(axis) = principal_axis_for_points(points, origin) else {
        return false;
    };
    let angle = axis.1.abs().atan2(axis.0.abs()).to_degrees();

    (MIN_AXIS_ANGLE_DEGREES..=MAX_AXIS_ANGLE_DEGREES).contains(&angle)
}

fn pixel_potrace_points_match_quadratic_vertex_medium_capsule_rescue(
    points: &[(f64, f64)],
) -> bool {
    const MIN_AXIS_ANGLE_DEGREES: f64 = 42.0;
    const MAX_AXIS_ANGLE_DEGREES: f64 = 52.0;
    const MIN_RADIUS: f64 = 18.0;

    if fit_closed_diagonal_capsule_potrace_segments(points).is_none() {
        return false;
    }

    let origin = arc_centroid(points);
    let Some(axis) = principal_axis_for_points(points, origin) else {
        return false;
    };
    let angle = axis.1.abs().atan2(axis.0.abs()).to_degrees();
    if !(MIN_AXIS_ANGLE_DEGREES..=MAX_AXIS_ANGLE_DEGREES).contains(&angle) {
        return false;
    }

    let Some(bounds) = local_bounds(points, origin, axis) else {
        return false;
    };
    let radius = (bounds.max_y - bounds.min_y) / 2.0 + 0.125;
    radius >= MIN_RADIUS
}

pub(crate) fn pixel_potrace_diagonal_capsule_compact_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    primitive: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 6;
    const MAX_EXTRA_SEGMENTS: usize = 24;
    const MAX_RELATIVE_PATH_BYTES_PERCENT: usize = 205;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if candidate.1.len() > primitive.1.len().saturating_add(MAX_EXTRA_SEGMENTS) {
        return false;
    }

    let rendered_candidate = quantize_potrace_candidate_to_tenth(candidate);
    let rendered_primitive = quantize_potrace_candidate_to_tenth(primitive);

    let candidate_bytes =
        compact_svg_path_data_from_segments(rendered_candidate.0, &rendered_candidate.1).len();
    let primitive_bytes =
        compact_svg_path_data_from_segments(rendered_primitive.0, &rendered_primitive.1).len();
    if candidate_bytes.saturating_mul(100)
        > primitive_bytes.saturating_mul(MAX_RELATIVE_PATH_BYTES_PERCENT)
    {
        return false;
    }

    let candidate_error =
        pixel_potrace_candidate_mask_error(path, &rendered_candidate, width, height);
    let primitive_error =
        pixel_potrace_candidate_mask_error(path, &rendered_primitive, width, height);
    if candidate_error.saturating_add(MIN_MASK_IMPROVEMENT_PIXELS) > primitive_error {
        return false;
    }

    let candidate_boundary_error =
        pixel_potrace_candidate_boundary_rms_error(path, &rendered_candidate);
    let primitive_boundary_error =
        pixel_potrace_candidate_boundary_rms_error(path, &rendered_primitive);
    pixel_potrace_boundary_error_is_acceptable(candidate_boundary_error, primitive_boundary_error)
}
