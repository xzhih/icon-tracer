use super::*;

pub(crate) fn fit_closed_annular_sector_potrace_segments(
    points: &[(f64, f64)],
    canvas_size: Option<(usize, usize)>,
) -> Option<Vec<SvgPathSegment>> {
    const MIN_OUTER_RADIUS: f64 = 24.0;
    const MIN_STROKE_WIDTH: f64 = 8.0;
    const MAX_INNER_TO_OUTER_RATIO: f64 = 0.53;
    const INNER_RADIUS_PERCENTILE: f64 = 0.15;
    const OUTER_RADIUS_PERCENTILE: f64 = 0.85;
    const MAX_TRIMMED_RADIAL_ERROR: f64 = 1.35;
    const MIN_GAP_RADIANS: f64 = 0.45;
    const MIN_FALLBACK_GAP_RADIANS: f64 = 1.75;
    const MIN_SPAN_RADIANS: f64 = 1.2;
    const MAX_SPAN_RADIANS: f64 = std::f64::consts::TAU - 0.25;

    if points.len() < 24 {
        return None;
    }

    let (width, height) = canvas_size?;
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
    if inner_radius / outer_radius > MAX_INNER_TO_OUTER_RATIO {
        return None;
    }

    if annular_sector_trimmed_radial_error(&distances, inner_radius, outer_radius)
        > MAX_TRIMMED_RADIAL_ERROR
    {
        return None;
    }

    let (start_angle, end_angle, gap) = annular_sector_angles(points, center)?;
    let start_angle = snap_angle_to_degrees(start_angle, 10.0);
    let mut end_angle = snap_angle_to_degrees(end_angle, 10.0);
    if end_angle <= start_angle {
        end_angle += std::f64::consts::TAU;
    }
    let span = end_angle - start_angle;
    if gap < MIN_GAP_RADIANS
        || gap < MIN_FALLBACK_GAP_RADIANS
        || !(MIN_SPAN_RADIANS..=MAX_SPAN_RADIANS).contains(&span)
    {
        return None;
    }

    Some(annular_sector_segments(
        center,
        inner_radius,
        outer_radius,
        start_angle,
        end_angle,
    ))
}

pub(crate) fn sorted_percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let position = (values.len() - 1) as f64 * percentile.clamp(0.0, 1.0);
    let lower = position.floor() as usize;
    let upper = (lower + 1).min(values.len() - 1);
    let amount = position - lower as f64;
    values[lower] * (1.0 - amount) + values[upper] * amount
}

pub(crate) fn annular_sector_trimmed_radial_error(
    distances: &[f64],
    inner_radius: f64,
    outer_radius: f64,
) -> f64 {
    const TRIMMED_FRACTION: f64 = 0.7;

    let mut errors = distances
        .iter()
        .map(|distance| {
            (distance - inner_radius)
                .abs()
                .min((distance - outer_radius).abs())
        })
        .collect::<Vec<_>>();
    errors.sort_by(f64::total_cmp);

    let kept = ((errors.len() as f64 * TRIMMED_FRACTION).round() as usize).max(1);
    errors.iter().take(kept).sum::<f64>() / kept as f64
}

pub(crate) fn annular_sector_angles(
    points: &[(f64, f64)],
    center: (f64, f64),
) -> Option<(f64, f64, f64)> {
    let mut angles = points
        .iter()
        .map(|point| {
            (point.1 - center.1)
                .atan2(point.0 - center.0)
                .rem_euclid(std::f64::consts::TAU)
        })
        .collect::<Vec<_>>();
    if angles.len() < 2 {
        return None;
    }
    angles.sort_by(f64::total_cmp);

    let mut best_gap = 0.0;
    let mut best_index = 0usize;
    for index in 0..angles.len() {
        let next = if index + 1 == angles.len() {
            angles[0] + std::f64::consts::TAU
        } else {
            angles[index + 1]
        };
        let gap = next - angles[index];
        if gap > best_gap {
            best_gap = gap;
            best_index = index;
        }
    }

    let start = angles[(best_index + 1) % angles.len()];
    let mut end = angles[best_index];
    if end < start {
        end += std::f64::consts::TAU;
    }

    Some((start, end, best_gap))
}

pub(crate) fn snap_angle_to_degrees(angle: f64, step_degrees: f64) -> f64 {
    let degrees = angle.to_degrees();
    (degrees / step_degrees).round() * step_degrees.to_radians()
}

pub(crate) fn annular_sector_segments(
    center: (f64, f64),
    inner_radius: f64,
    outer_radius: f64,
    start_angle: f64,
    end_angle: f64,
) -> Vec<SvgPathSegment> {
    let mut segments = Vec::new();
    append_circular_arc_segments(&mut segments, center, outer_radius, start_angle, end_angle);

    let outer_end = circle_point(center, outer_radius, end_angle);
    let inner_end = circle_point(center, inner_radius, end_angle);
    segments.push(SvgPathSegment::Line {
        start: outer_end,
        end: inner_end,
    });

    append_circular_arc_segments(&mut segments, center, inner_radius, end_angle, start_angle);

    let inner_start = circle_point(center, inner_radius, start_angle);
    let outer_start = circle_point(center, outer_radius, start_angle);
    segments.push(SvgPathSegment::Line {
        start: inner_start,
        end: outer_start,
    });

    segments
}

pub(crate) fn append_circular_arc_segments(
    segments: &mut Vec<SvgPathSegment>,
    center: (f64, f64),
    radius: f64,
    start_angle: f64,
    end_angle: f64,
) {
    const MAX_ARC_STEP: f64 = std::f64::consts::FRAC_PI_3;

    let total = end_angle - start_angle;
    let count = (total.abs() / MAX_ARC_STEP).ceil().max(1.0) as usize;
    for index in 0..count {
        let start = start_angle + total * index as f64 / count as f64;
        let end = start_angle + total * (index + 1) as f64 / count as f64;
        segments.push(SvgPathSegment::Cubic(circular_arc_cubic(
            center, radius, start, end,
        )));
    }
}

pub(crate) fn circular_arc_cubic(
    center: (f64, f64),
    radius: f64,
    start_angle: f64,
    end_angle: f64,
) -> CubicSegment {
    let delta = end_angle - start_angle;
    let handle = 4.0 / 3.0 * (delta / 4.0).tan();
    let start = circle_point(center, radius, start_angle);
    let end = circle_point(center, radius, end_angle);
    let start_tangent = (-start_angle.sin(), start_angle.cos());
    let end_tangent = (-end_angle.sin(), end_angle.cos());

    CubicSegment {
        start,
        control1: add(start, scale(start_tangent, radius * handle)),
        control2: subtract(end, scale(end_tangent, radius * handle)),
        end,
    }
}

pub(crate) fn circle_point(center: (f64, f64), radius: f64, angle: f64) -> (f64, f64) {
    (
        center.0 + radius * angle.cos(),
        center.1 + radius * angle.sin(),
    )
}

pub(crate) fn distance_float(a: (f64, f64), b: (f64, f64)) -> f64 {
    distance_squared_float(a, b).sqrt()
}
