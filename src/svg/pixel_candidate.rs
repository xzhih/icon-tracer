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

pub(crate) fn pixel_potrace_relaxed_point_set_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MAX_EXTRA_MASK_PIXELS: usize = 48;
    const MIN_SEGMENT_SAVINGS: usize = 3;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if candidate.1.len().saturating_add(MIN_SEGMENT_SAVINGS) > best.1.len() {
        return false;
    }

    let rendered_candidate = quantize_potrace_candidate_to_tenth(candidate);
    let rendered_best = quantize_potrace_candidate_to_tenth(best);
    let candidate_error =
        pixel_potrace_candidate_mask_error(path, &rendered_candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, &rendered_best, width, height);
    if candidate_error <= best_error
        || candidate_error > best_error.saturating_add(MAX_EXTRA_MASK_PIXELS)
    {
        return false;
    }

    let candidate_boundary_error =
        pixel_potrace_candidate_boundary_rms_error(path, &rendered_candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, &rendered_best);
    if !pixel_potrace_boundary_error_is_acceptable(candidate_boundary_error, best_boundary_error) {
        return false;
    }

    let candidate_bytes = compact_svg_path_data_from_segments_without_arcs(
        rendered_candidate.0,
        &rendered_candidate.1,
    )
    .len();
    let best_bytes =
        compact_svg_path_data_from_segments_without_arcs(rendered_best.0, &rendered_best.1).len();

    candidate_bytes < best_bytes
}

pub(crate) fn pixel_potrace_area_alpha_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_D_BYTES_SAVINGS: usize = 16;
    const MAX_EXTRA_MASK_PIXELS: usize = 16;

    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();
    if candidate_bytes.saturating_add(MIN_D_BYTES_SAVINGS) > best_bytes {
        return false;
    }

    let Some((width, height)) = canvas_size else {
        return true;
    };

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    candidate_error <= best_error.saturating_add(MAX_EXTRA_MASK_PIXELS)
}

pub(crate) fn pixel_potrace_area_alpha_final_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
    allow_material_growth: bool,
) -> bool {
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 4;
    const MATERIAL_MASK_IMPROVEMENT_PIXELS: usize = 32;
    const STRONG_MASK_IMPROVEMENT_PIXELS: usize = 64;
    const MAX_EXTRA_D_BYTES: usize = 8;
    const MAX_MATERIAL_EXTRA_D_BYTES: usize = 96;
    const MAX_MATERIAL_EXTRA_SEGMENTS: usize = 2;
    const MIN_MATERIAL_MIRROR_MISMATCH_RATIO: f64 = 0.3;
    const STRONG_MIN_D_BYTES_SAVINGS: usize = 64;
    const MAX_EXTRA_BOUNDARY_ERROR: f64 = 0.02;
    const MIN_BEST_SEGMENTS: usize = 16;
    const MAX_STRONG_MIRROR_MISMATCH_RATIO: f64 = 1.0;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if best.1.len() < MIN_BEST_SEGMENTS {
        return false;
    }

    let saves_segments = candidate.1.len() < best.1.len();
    let preserves_segment_budget = candidate.1.len() <= best.1.len().saturating_add(1);

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

    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();
    if saves_segments && candidate_bytes <= best_bytes.saturating_add(MAX_EXTRA_D_BYTES) {
        return true;
    }

    if allow_material_growth
        && candidate.1.len() <= best.1.len().saturating_add(MAX_MATERIAL_EXTRA_SEGMENTS)
        && candidate_error.saturating_add(MATERIAL_MASK_IMPROVEMENT_PIXELS) <= best_error
        && candidate_bytes <= best_bytes.saturating_add(MAX_MATERIAL_EXTRA_D_BYTES)
        && pixel_potrace_horizontal_mirror_mismatch_ratio(path, width, height)
            >= MIN_MATERIAL_MIRROR_MISMATCH_RATIO
    {
        return true;
    }

    preserves_segment_budget
        && candidate_error.saturating_add(STRONG_MASK_IMPROVEMENT_PIXELS) <= best_error
        && candidate_boundary_error <= best_boundary_error
        && candidate_bytes.saturating_add(STRONG_MIN_D_BYTES_SAVINGS) <= best_bytes
        && pixel_potrace_horizontal_mirror_mismatch_ratio(path, width, height)
            < MAX_STRONG_MIRROR_MISMATCH_RATIO
}

pub(crate) fn pixel_potrace_area_alpha_smoothing_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_SEGMENT_SAVINGS: usize = 2;
    const MAX_EXTRA_MASK_PIXELS: usize = 64;
    const MAX_MASK_IMPROVEMENT_PIXELS: usize = 80;
    const MAX_EXTRA_D_BYTES: usize = 64;
    const MAX_EXTRA_BOUNDARY_ERROR: f64 = 0.20;
    const MAX_RELATIVE_BOUNDARY_ERROR: f64 = 1.10;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if candidate.1.len().saturating_add(MIN_SEGMENT_SAVINGS) > best.1.len() {
        return false;
    }

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    if candidate_error > best_error.saturating_add(MAX_EXTRA_MASK_PIXELS) {
        return false;
    }
    if best_error.saturating_sub(candidate_error) > MAX_MASK_IMPROVEMENT_PIXELS {
        return false;
    }

    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);
    if candidate_boundary_error
        > (best_boundary_error + MAX_EXTRA_BOUNDARY_ERROR)
            .max(best_boundary_error * MAX_RELATIVE_BOUNDARY_ERROR)
    {
        return false;
    }

    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();

    candidate_bytes <= best_bytes.saturating_add(MAX_EXTRA_D_BYTES)
}

pub(crate) fn pixel_potrace_fitted_candidate_is_materially_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_LONGER_FIT_MASK_IMPROVEMENT: usize = 8;
    const MAX_SHORT_FIT_EXTRA_D_BYTES: usize = 16;

    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();
    if candidate.1.len() <= best.1.len()
        && candidate_bytes <= best_bytes.saturating_add(MAX_SHORT_FIT_EXTRA_D_BYTES)
    {
        return pixel_potrace_candidate_is_better(path, canvas_size, candidate, best);
    }

    let Some((width, height)) = canvas_size else {
        return false;
    };

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    candidate_error.saturating_add(MIN_LONGER_FIT_MASK_IMPROVEMENT) <= best_error
        && pixel_potrace_candidate_is_better(path, canvas_size, candidate, best)
}

pub(crate) fn pixel_potrace_candidate_is_no_more_complex(
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MAX_EXTRA_D_BYTES: usize = 16;

    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();
    candidate.1.len() <= best.1.len()
        && candidate_bytes <= best_bytes.saturating_add(MAX_EXTRA_D_BYTES)
}

pub(crate) fn pixel_potrace_fine_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 8;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if !pixel_potrace_candidate_is_no_more_complex(candidate, best) {
        return false;
    }

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    candidate_error.saturating_add(MIN_MASK_IMPROVEMENT_PIXELS) <= best_error
        && pixel_potrace_candidate_is_better(path, canvas_size, candidate, best)
}

pub(crate) fn pixel_potrace_loose_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MAX_EXTRA_MASK_PIXELS: usize = 48;
    const MAX_EXTRA_D_BYTES: usize = 32;
    const MIN_BEST_D_BYTES: usize = 240;
    const MIN_SEGMENT_SAVINGS: usize = 4;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if candidate.1.len().saturating_add(MIN_SEGMENT_SAVINGS) > best.1.len() {
        return false;
    }

    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();
    if best_bytes < MIN_BEST_D_BYTES {
        return false;
    }

    if candidate_bytes > best_bytes.saturating_add(MAX_EXTRA_D_BYTES) {
        return false;
    }

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    if candidate_error > best_error.saturating_add(MAX_EXTRA_MASK_PIXELS) {
        return false;
    }

    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);
    pixel_potrace_boundary_error_is_acceptable(candidate_boundary_error, best_boundary_error)
}

pub(crate) fn pixel_potrace_best_area_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_FOREGROUND_DELTA_IMPROVEMENT: usize = 4;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if !pixel_potrace_loose_candidate_is_better(path, canvas_size, candidate, best) {
        return false;
    }

    let candidate_delta =
        pixel_potrace_candidate_foreground_delta(path, candidate, width, height).unsigned_abs();
    let best_delta =
        pixel_potrace_candidate_foreground_delta(path, best, width, height).unsigned_abs();
    candidate_delta.saturating_add(MIN_FOREGROUND_DELTA_IMPROVEMENT) <= best_delta
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

fn quantize_potrace_candidate_to_tenth(
    candidate: &((f64, f64), Vec<SvgPathSegment>),
) -> ((f64, f64), Vec<SvgPathSegment>) {
    (
        quantize_point_to_tenth(candidate.0),
        candidate
            .1
            .iter()
            .copied()
            .map(quantize_segment_to_tenth)
            .collect(),
    )
}

fn quantize_segment_to_tenth(segment: SvgPathSegment) -> SvgPathSegment {
    match segment {
        SvgPathSegment::Line { start, end } => SvgPathSegment::Line {
            start: quantize_point_to_tenth(start),
            end: quantize_point_to_tenth(end),
        },
        SvgPathSegment::Cubic(cubic) => SvgPathSegment::Cubic(CubicSegment {
            start: quantize_point_to_tenth(cubic.start),
            control1: quantize_point_to_tenth(cubic.control1),
            control2: quantize_point_to_tenth(cubic.control2),
            end: quantize_point_to_tenth(cubic.end),
        }),
    }
}

fn quantize_point_to_tenth(point: (f64, f64)) -> (f64, f64) {
    (quantize_to_tenth(point.0), quantize_to_tenth(point.1))
}

fn quantize_to_tenth(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
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

pub(crate) fn pixel_potrace_compact_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
    allow_exact_best_replacement: bool,
) -> bool {
    const MIN_MASK_SLACK_PIXELS: usize = 256;
    const MAX_MASK_SLACK_RATIO: f64 = 0.004;
    const MAX_BOUNDARY_ERROR: f64 = 0.9;
    const MAX_EXTRA_BOUNDARY_ERROR: f64 = 0.65;
    const MAX_RELATIVE_BOUNDARY_ERROR: f64 = 3.25;
    const MIN_BEST_MASK_ERROR: usize = 32;
    const MIN_RELATIVE_MASK_ERROR: usize = 4;
    const MAX_RELATIVE_MASK_ERROR: usize = 6;
    const EXTRA_RELATIVE_MASK_PIXELS: usize = 8;
    const MAX_COMPACT_FOREGROUND_DELTA: isize = -120;
    const MIN_HORIZONTAL_MIRROR_MISMATCH_RATIO: f64 = 0.3;
    const MAX_RELATIVE_PATH_BYTES: usize = 90;
    const MIN_SEGMENT_SAVINGS: usize = 3;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    if best_error < MIN_BEST_MASK_ERROR {
        if !allow_exact_best_replacement {
            return false;
        }
    } else if candidate_error < best_error.saturating_mul(MIN_RELATIVE_MASK_ERROR) {
        return false;
    }

    let slack = MIN_MASK_SLACK_PIXELS
        .max((width.saturating_mul(height) as f64 * MAX_MASK_SLACK_RATIO).round() as usize);
    if candidate_error > best_error.saturating_add(slack)
        || (best_error >= MIN_BEST_MASK_ERROR
            && candidate_error
                > best_error
                    .saturating_mul(MAX_RELATIVE_MASK_ERROR)
                    .saturating_add(EXTRA_RELATIVE_MASK_PIXELS))
    {
        return false;
    }

    if pixel_potrace_candidate_foreground_delta(path, candidate, width, height)
        > MAX_COMPACT_FOREGROUND_DELTA
    {
        return false;
    }
    if pixel_potrace_horizontal_mirror_mismatch_ratio(path, width, height)
        < MIN_HORIZONTAL_MIRROR_MISMATCH_RATIO
    {
        return false;
    }

    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);
    if candidate_boundary_error > MAX_BOUNDARY_ERROR
        || candidate_boundary_error
            > (best_boundary_error + MAX_EXTRA_BOUNDARY_ERROR)
                .max(best_boundary_error * MAX_RELATIVE_BOUNDARY_ERROR)
    {
        return false;
    }

    let candidate_bytes = compact_svg_path_data_from_segments(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments(best.0, &best.1).len();
    let saves_segments = candidate.1.len().saturating_add(MIN_SEGMENT_SAVINGS) <= best.1.len();
    let saves_bytes =
        candidate_bytes.saturating_mul(100) <= best_bytes.saturating_mul(MAX_RELATIVE_PATH_BYTES);

    saves_segments && saves_bytes
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
    rasterize_path_evenodd_coverage_threshold(
        &candidate_path,
        width,
        height,
        CANDIDATE_MASK_SUPERSAMPLE,
        &mut candidate_pixels,
    );

    reference
        .iter()
        .zip(candidate_pixels.iter())
        .filter(|(left, right)| left != right)
        .count()
}

pub(crate) fn pixel_potrace_candidate_foreground_delta(
    path: &TracePath,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    width: usize,
    height: usize,
) -> isize {
    let mut reference = vec![false; width.saturating_mul(height)];
    let mut candidate_pixels = vec![false; width.saturating_mul(height)];
    rasterize_path_evenodd(path, width, height, &mut reference);

    let candidate_path = TracePath {
        is_hole: path.is_hole,
        points: flattened_potrace_segments(candidate.0, &candidate.1),
    };
    rasterize_path_evenodd_coverage_threshold(
        &candidate_path,
        width,
        height,
        CANDIDATE_MASK_SUPERSAMPLE,
        &mut candidate_pixels,
    );

    let reference_foreground = reference.iter().filter(|pixel| **pixel).count();
    let candidate_foreground = candidate_pixels.iter().filter(|pixel| **pixel).count();
    candidate_foreground as isize - reference_foreground as isize
}

pub(crate) fn pixel_potrace_horizontal_mirror_mismatch_ratio(
    path: &TracePath,
    width: usize,
    height: usize,
) -> f64 {
    let mut reference = vec![false; width.saturating_mul(height)];
    rasterize_path_evenodd(path, width, height, &mut reference);
    let foreground = reference.iter().filter(|pixel| **pixel).count();
    if foreground == 0 {
        return f64::INFINITY;
    }

    let mut mismatches = 0usize;
    for y in 0..height {
        for x in 0..width {
            if reference[y * width + x] != reference[y * width + (width - 1 - x)] {
                mismatches += 1;
            }
        }
    }

    mismatches as f64 / foreground as f64
}

pub(crate) const CANDIDATE_MASK_SUPERSAMPLE: usize = 4;

pub(crate) fn rasterize_path_evenodd_coverage_threshold(
    path: &TracePath,
    width: usize,
    height: usize,
    scale: usize,
    pixels: &mut [bool],
) {
    if path.points.len() < 3 || scale == 0 {
        return;
    }

    let expected = width.saturating_mul(height);
    if pixels.len() < expected {
        return;
    }

    let sample_width = width.saturating_mul(scale);
    let sample_height = height.saturating_mul(scale);
    let threshold = scale.saturating_mul(scale);
    let mut coverage = vec![0u16; expected];
    let mut intersections = Vec::new();

    for sample_y in 0..sample_height {
        let scan_y = (sample_y as f64 + 0.5) / scale as f64;
        intersections.clear();

        for (start, end) in path.points.iter().zip(path.points.iter().cycle().skip(1)) {
            if (start.1 <= scan_y && scan_y < end.1) || (end.1 <= scan_y && scan_y < start.1) {
                let amount = (scan_y - start.1) / (end.1 - start.1);
                intersections.push(start.0 + (end.0 - start.0) * amount);
            }
        }

        intersections.sort_by(|a, b| a.total_cmp(b));
        let pixel_y = sample_y / scale;

        for pair in intersections.chunks_exact(2) {
            let left = pair[0].min(pair[1]);
            let right = pair[0].max(pair[1]);
            let start_x = clamp_sample_x((left * scale as f64 - 0.5).ceil(), sample_width);
            let end_x = clamp_sample_x((right * scale as f64 - 0.5).ceil(), sample_width);

            for sample_x in start_x..end_x {
                let pixel_x = sample_x / scale;
                coverage[pixel_y * width + pixel_x] += 1;
            }
        }
    }

    for (pixel, count) in pixels.iter_mut().zip(coverage) {
        *pixel = usize::from(count).saturating_mul(2) >= threshold;
    }
}

pub(crate) fn clamp_sample_x(value: f64, sample_width: usize) -> usize {
    if value <= 0.0 {
        0
    } else if value >= sample_width as f64 {
        sample_width
    } else {
        value as usize
    }
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
