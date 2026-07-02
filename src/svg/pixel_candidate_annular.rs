use super::*;
use crate::TracePath;

pub(crate) fn pixel_potrace_ring_sector_detailed_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 12;
    const MIN_SAME_SEGMENT_MASK_IMPROVEMENT_PIXELS: usize = 8;
    const MIN_SEGMENT_GROWTH: usize = 3;
    const MAX_SEGMENT_GROWTH: usize = 6;
    const MAX_EXTRA_D_BYTES: usize = 180;
    const MIN_THIN_SEGMENT_GROWTH: usize = 7;
    const MAX_THIN_SEGMENT_GROWTH: usize = 8;
    const MAX_THIN_EXTRA_MASK_PIXELS: usize = 12;
    const MAX_THIN_EXTRA_D_BYTES: usize = 220;
    const MAX_THIN_CANDIDATE_FOREGROUND_DELTA: usize = 24;
    const MAX_THIN_BEST_FOREGROUND_DELTA: usize = 8;
    const MIN_COMPACT_ANNULAR_SEGMENT_GROWTH: usize = 8;
    const MAX_COMPACT_ANNULAR_SEGMENT_GROWTH: usize = 10;
    const MAX_COMPACT_ANNULAR_BEST_ERROR: usize = 24;
    const MAX_COMPACT_ANNULAR_EXTRA_MASK_PIXELS: usize = 128;
    const MAX_COMPACT_ANNULAR_EXTRA_D_BYTES: usize = 300;
    const MAX_COMPACT_ANNULAR_EXTRA_BOUNDARY_ERROR: f64 = 0.12;
    const MAX_COMPACT_ANNULAR_EXTRA_FOREGROUND_DELTA: usize = 4;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    if !pixel_potrace_points_are_detailed_annular_sector(&path.points, width, height) {
        return false;
    }

    let segment_growth = candidate.1.len().saturating_sub(best.1.len());
    let candidate_bytes =
        compact_svg_path_data_from_segments_without_arcs(candidate.0, &candidate.1).len();
    let best_bytes = compact_svg_path_data_from_segments_without_arcs(best.0, &best.1).len();

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);

    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);

    let candidate_delta =
        pixel_potrace_candidate_foreground_delta(path, candidate, width, height).unsigned_abs();
    let best_delta =
        pixel_potrace_candidate_foreground_delta(path, best, width, height).unsigned_abs();
    if segment_growth == 0
        && candidate_bytes <= best_bytes
        && candidate_error.saturating_add(MIN_SAME_SEGMENT_MASK_IMPROVEMENT_PIXELS) <= best_error
        && candidate_boundary_error <= best_boundary_error
        && candidate_delta <= best_delta
    {
        return true;
    }

    if (MIN_SEGMENT_GROWTH..=MAX_SEGMENT_GROWTH).contains(&segment_growth)
        && candidate_bytes <= best_bytes.saturating_add(MAX_EXTRA_D_BYTES)
        && candidate_error.saturating_add(MIN_MASK_IMPROVEMENT_PIXELS) <= best_error
        && candidate_boundary_error <= best_boundary_error
        && candidate_delta <= best_delta
    {
        return true;
    }

    if (MIN_THIN_SEGMENT_GROWTH..=MAX_THIN_SEGMENT_GROWTH).contains(&segment_growth)
        && candidate_bytes <= best_bytes.saturating_add(MAX_THIN_EXTRA_D_BYTES)
        && candidate_error <= best_error.saturating_add(MAX_THIN_EXTRA_MASK_PIXELS)
        && candidate_boundary_error < best_boundary_error
        && candidate_delta <= MAX_THIN_CANDIDATE_FOREGROUND_DELTA
        && best_delta <= MAX_THIN_BEST_FOREGROUND_DELTA
    {
        return true;
    }

    let compact_annular_best =
        fit_closed_annular_sector_potrace_segments(&path.points, canvas_size).is_some_and(
            |segments| {
                segments.first().is_some_and(|first| {
                    compact_svg_path_data_from_segments_without_arcs(first.start(), &segments)
                        == compact_svg_path_data_from_segments_without_arcs(best.0, &best.1)
                })
            },
        );
    compact_annular_best
        && (MIN_COMPACT_ANNULAR_SEGMENT_GROWTH..=MAX_COMPACT_ANNULAR_SEGMENT_GROWTH)
            .contains(&segment_growth)
        && best_error <= MAX_COMPACT_ANNULAR_BEST_ERROR
        && candidate_error <= best_error.saturating_add(MAX_COMPACT_ANNULAR_EXTRA_MASK_PIXELS)
        && candidate_bytes <= best_bytes.saturating_add(MAX_COMPACT_ANNULAR_EXTRA_D_BYTES)
        && candidate_boundary_error
            <= best_boundary_error + MAX_COMPACT_ANNULAR_EXTRA_BOUNDARY_ERROR
        && candidate_delta <= best_delta.saturating_add(MAX_COMPACT_ANNULAR_EXTRA_FOREGROUND_DELTA)
}

pub(crate) fn pixel_potrace_ring_sector_loose_vertex_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MIN_MASK_IMPROVEMENT_PIXELS: usize = 8;
    const MIN_DETAILED_MASK_IMPROVEMENT_PIXELS: usize = 2;
    const MAX_EXTRA_D_BYTES: usize = 24;
    const MAX_EXTRA_BOUNDARY_ERROR: f64 = 0.03;
    const MAX_EXTRA_FOREGROUND_DELTA: usize = 8;

    let Some((width, height)) = canvas_size else {
        return false;
    };

    let thin = pixel_potrace_points_are_thin_detailed_annular_sector(&path.points, width, height);
    if !thin && !pixel_potrace_points_match_moderate_gap_annular_sector(&path.points, width, height)
    {
        return false;
    }

    if candidate.1.len() > best.1.len() {
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
    let min_mask_improvement = if thin {
        MIN_MASK_IMPROVEMENT_PIXELS
    } else {
        MIN_DETAILED_MASK_IMPROVEMENT_PIXELS
    };
    if candidate_error.saturating_add(min_mask_improvement) > best_error {
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

fn pixel_potrace_points_match_moderate_gap_annular_sector(
    points: &[(f64, f64)],
    width: usize,
    height: usize,
) -> bool {
    const MIN_GAP_DEGREES: f64 = 120.0;
    const MAX_GAP_DEGREES: f64 = 150.0;

    if !pixel_potrace_points_are_detailed_annular_sector(points, width, height) {
        return false;
    }

    let center = (width as f64 / 2.0, height as f64 / 2.0);
    let Some((_, _, gap_radians)) = annular_sector_angles(points, center) else {
        return false;
    };
    let gap_degrees = gap_radians.to_degrees();

    (MIN_GAP_DEGREES..=MAX_GAP_DEGREES).contains(&gap_degrees)
}

#[derive(Debug, Clone, Copy)]
struct AnnularSectorMeasurements {
    inner_radius: f64,
    outer_radius: f64,
}

pub(crate) fn pixel_potrace_points_are_detailed_annular_sector(
    points: &[(f64, f64)],
    width: usize,
    height: usize,
) -> bool {
    const MAX_INNER_TO_OUTER_RATIO: f64 = 0.62;

    let Some(measurements) = annular_sector_measurements(points, width, height) else {
        return false;
    };
    measurements.inner_radius / measurements.outer_radius <= MAX_INNER_TO_OUTER_RATIO
}

pub(crate) fn pixel_potrace_points_prefer_fractional_precision_annular_sector(
    points: &[(f64, f64)],
    width: usize,
    height: usize,
) -> bool {
    const MIN_POINTS: usize = 256;
    const MIN_GAP_DEGREES: f64 = 55.0;

    if points.len() < MIN_POINTS {
        return false;
    }

    if !pixel_potrace_points_are_detailed_annular_sector(points, width, height) {
        return false;
    }

    let center = (width as f64 / 2.0, height as f64 / 2.0);
    let Some((_, _, gap_radians)) = annular_sector_angles(points, center) else {
        return false;
    };

    gap_radians.to_degrees() >= MIN_GAP_DEGREES
}

fn pixel_potrace_points_are_thin_detailed_annular_sector(
    points: &[(f64, f64)],
    width: usize,
    height: usize,
) -> bool {
    const MIN_INNER_TO_OUTER_RATIO: f64 = 0.56;
    const MAX_INNER_TO_OUTER_RATIO: f64 = 0.62;

    let Some(measurements) = annular_sector_measurements(points, width, height) else {
        return false;
    };
    let ratio = measurements.inner_radius / measurements.outer_radius;
    (MIN_INNER_TO_OUTER_RATIO..=MAX_INNER_TO_OUTER_RATIO).contains(&ratio)
}

fn annular_sector_measurements(
    points: &[(f64, f64)],
    width: usize,
    height: usize,
) -> Option<AnnularSectorMeasurements> {
    const MIN_POINTS: usize = 128;
    const MIN_OUTER_RADIUS: f64 = 48.0;
    const MIN_STROKE_WIDTH: f64 = 24.0;
    const INNER_RADIUS_PERCENTILE: f64 = 0.15;
    const OUTER_RADIUS_PERCENTILE: f64 = 0.85;
    const MAX_TRIMMED_RADIAL_ERROR: f64 = 1.5;
    const MIN_GAP_RADIANS: f64 = 0.6;

    if points.len() < MIN_POINTS {
        return None;
    }

    let center = (width as f64 / 2.0, height as f64 / 2.0);
    let bounds = FloatBounds::from_points(points)?;
    if center.0 < bounds.min_x
        || center.0 > bounds.max_x
        || center.1 < bounds.min_y
        || center.1 > bounds.max_y
    {
        return None;
    }

    let mut distances = points
        .iter()
        .map(|point| distance_float(*point, center))
        .collect::<Vec<_>>();
    distances.sort_by(f64::total_cmp);
    let inner_radius = sorted_percentile(&distances, INNER_RADIUS_PERCENTILE).round();
    let outer_radius = sorted_percentile(&distances, OUTER_RADIUS_PERCENTILE).round();
    if outer_radius < MIN_OUTER_RADIUS || outer_radius - inner_radius < MIN_STROKE_WIDTH {
        return None;
    }
    if annular_sector_trimmed_radial_error(&distances, inner_radius, outer_radius)
        > MAX_TRIMMED_RADIAL_ERROR
    {
        return None;
    }

    let (_, _, gap_radians) = annular_sector_angles(points, center)?;
    if gap_radians < MIN_GAP_RADIANS {
        return None;
    }

    Some(AnnularSectorMeasurements {
        inner_radius,
        outer_radius,
    })
}
