use super::*;
use crate::trace::rasterize_path_evenodd;

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
