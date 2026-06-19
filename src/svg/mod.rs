use crate::trace::rasterize_path_evenodd;
mod cubic;
mod path_data;
mod templates;

pub(crate) use cubic::*;
pub(crate) use path_data::*;
pub(crate) use templates::*;

use crate::{CurveMode, SvgRenderOptions, TracePath};

pub(crate) fn svg_path_element(
    path_data: &str,
    allow_scaled_potrace_path: bool,
    canvas_height: usize,
) -> String {
    let plain = format!(r#"<path fill="black" fill-rule="evenodd" d="{path_data}"/>"#);
    if !allow_scaled_potrace_path {
        return plain;
    }
    let mut best = plain;
    let mut best_path_data_len = path_data.len();

    if let Some(scaled_path_data) = scaled_integer_svg_path_data(path_data, 100.0) {
        let scaled = format!(
            r#"<path fill="black" fill-rule="evenodd" transform="scale(.01)" d="{scaled_path_data}"/>"#
        );

        if scaled.len() < best.len() {
            best = scaled;
            best_path_data_len = scaled_path_data.len();
        }
    }

    if !path_data_has_arc_commands(path_data) {
        if let Some(one_decimal_path_data) = one_decimal_svg_path_data(path_data) {
            let one_decimal =
                format!(r#"<path fill="black" fill-rule="evenodd" d="{one_decimal_path_data}"/>"#);
            if one_decimal.len() < best.len() {
                best = one_decimal;
                best_path_data_len = one_decimal_path_data.len();
            }

            if path_data_has_quadratic_commands(&one_decimal_path_data) {
                if let Some(snapped_path_data) =
                    snap_near_integer_one_decimal_svg_path_data(&one_decimal_path_data)
                {
                    let snapped = format!(
                        r#"<path fill="black" fill-rule="evenodd" d="{snapped_path_data}"/>"#
                    );
                    if snapped.len() < best.len() {
                        best = snapped;
                        best_path_data_len = snapped_path_data.len();
                    }
                }
            }

            if let Some(potrace_path_data) =
                potrace_y_flipped_integer_svg_path_data(&one_decimal_path_data, canvas_height, 10.0)
            {
                let potrace_scaled = format!(
                    r#"<path fill="black" fill-rule="evenodd" transform="translate(0 {canvas_height}) scale(.1 -.1)" d="{potrace_path_data}"/>"#
                );

                if potrace_path_data.len() < best_path_data_len {
                    best = potrace_scaled;
                }
            }
        }
    }

    best
}

pub(crate) fn path_data_has_arc_commands(path_data: &str) -> bool {
    path_data.bytes().any(|byte| matches!(byte, b'A' | b'a'))
}

pub(crate) fn path_to_svg_data(
    path: &TracePath,
    options: SvgRenderOptions,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
) -> Option<String> {
    match options.curve_mode {
        CurveMode::Polygon => path_to_polygon_svg_data(path),
        CurveMode::Smooth => path_to_smooth_svg_data(path),
        CurveMode::Spline => path_to_spline_svg_data(path),
        CurveMode::Fit => path_to_fit_svg_data(path),
        CurveMode::Potrace => path_to_potrace_svg_data(
            path,
            options.opt_tolerance.max(0.0),
            options.pixel_potrace,
            canvas_size,
            has_holes,
        ),
    }
}

pub(crate) fn path_to_polygon_svg_data(path: &TracePath) -> Option<String> {
    let (first, rest) = path.points.split_first()?;
    let mut data = format!("M {} {}", format_float(first.0), format_float(first.1));

    for point in rest {
        data.push_str(&format!(
            " L {} {}",
            format_float(point.0),
            format_float(point.1)
        ));
    }

    data.push_str(" Z");
    Some(data)
}

pub(crate) fn path_to_smooth_svg_data(path: &TracePath) -> Option<String> {
    const CORNER_SMOOTHING: f64 = 0.25;

    if path.points.len() < 3 {
        return path_to_polygon_svg_data(path);
    }

    let first_exit = corner_exit(&path.points, 0, CORNER_SMOOTHING);
    let mut data = format!(
        "M {} {}",
        format_float(first_exit.0),
        format_float(first_exit.1)
    );

    for offset in 1..=path.points.len() {
        let vertex_index = offset % path.points.len();
        let vertex = path.points[vertex_index];
        let entry = corner_entry(&path.points, vertex_index, CORNER_SMOOTHING);
        let exit = corner_exit(&path.points, vertex_index, CORNER_SMOOTHING);
        let control1 = cubic_control_point(entry, vertex);
        let control2 = cubic_control_point(exit, vertex);

        data.push_str(&format!(
            " L {} {} C {} {}, {} {}, {} {}",
            format_float(entry.0),
            format_float(entry.1),
            format_float(control1.0),
            format_float(control1.1),
            format_float(control2.0),
            format_float(control2.1),
            format_float(exit.0),
            format_float(exit.1)
        ));
    }

    data.push_str(" Z");
    Some(data)
}

pub(crate) fn path_to_spline_svg_data(path: &TracePath) -> Option<String> {
    if path.points.len() < 3 {
        return path_to_polygon_svg_data(path);
    }

    let mut data = format!(
        "M {} {}",
        format_float(path.points[0].0),
        format_float(path.points[0].1)
    );

    for index in 0..path.points.len() {
        let (control1, control2, next) = catmull_rom_segment(path, index);

        data.push_str(&format!(
            " C {} {}, {} {}, {} {}",
            format_float(control1.0),
            format_float(control1.1),
            format_float(control2.0),
            format_float(control2.1),
            format_float(next.0),
            format_float(next.1)
        ));
    }

    data.push_str(" Z");
    Some(data)
}

pub(crate) fn path_to_fit_svg_data(path: &TracePath) -> Option<String> {
    const FIT_ERROR: f64 = 0.75;

    if path.points.len() < 3 {
        return path_to_polygon_svg_data(path);
    }

    let bounds = FloatBounds::from_points(&path.points)?;
    let segments = fit_closed_cubic_segments(&path.points, FIT_ERROR);
    let first = segments.first()?;
    let mut data = format!(
        "M {} {}",
        format_float(first.start.0),
        format_float(first.start.1)
    );

    for segment in segments {
        let control1 = bounds.clamp(segment.control1);
        let control2 = bounds.clamp(segment.control2);

        data.push_str(&format!(
            " C {} {}, {} {}, {} {}",
            format_float(control1.0),
            format_float(control1.1),
            format_float(control2.0),
            format_float(control2.1),
            format_float(segment.end.0),
            format_float(segment.end.1)
        ));
    }

    data.push_str(" Z");
    Some(data)
}

pub(crate) fn path_to_potrace_svg_data(
    path: &TracePath,
    opt_tolerance: f64,
    pixel_potrace: bool,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
) -> Option<String> {
    if path.points.len() < 3 {
        return path_to_polygon_svg_data(path);
    }

    if pixel_potrace {
        let (start, segments) =
            choose_pixel_potrace_point_set(path, opt_tolerance, canvas_size, has_holes)?;
        return Some(compact_svg_path_data_from_segments_without_arcs(
            start, &segments,
        ));
    }

    let polygon = legacy_potrace_polygon_indices(&path.points);
    let vertices = adjust_potrace_vertices(&path.points, &polygon, 1.0);
    let (mut start, mut segments) = smooth_potrace_vertices(&vertices)?;

    if segments
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
        && path.points.len() >= 12
    {
        let fitted = fit_closed_smooth_potrace_segments(&path.points, true);
        if let Some(first) = fitted.first() {
            start = first.start();
            segments = fitted;
        }
    }

    let (start, segments) = optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        STRICT_POTRACE_LINEAR_DEVIATION,
    );
    let (start, segments) = choose_non_pixel_fit_candidate(path, canvas_size, start, segments);
    Some(svg_path_data_from_segments(start, &segments))
}

pub(crate) fn choose_non_pixel_fit_candidate(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    start: (f64, f64),
    segments: Vec<SvgPathSegment>,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    const MIN_SEGMENTS: usize = 16;
    const MAX_EXTRA_SEGMENTS: usize = 2;
    const FIT_ERROR: f64 = 0.75;

    let Some((width, height)) = canvas_size else {
        return (start, segments);
    };

    if points_are_half_pixel_quantized(&path.points) || segments.len() < MIN_SEGMENTS {
        return (start, segments);
    }

    if !svg_segments_are_all_cubic(&segments) {
        return (start, segments);
    }

    let fitted = fit_closed_cubic_segments(&path.points, FIT_ERROR);
    let Some(first) = fitted.first() else {
        return (start, segments);
    };

    if fitted.len() > segments.len() + MAX_EXTRA_SEGMENTS {
        return (start, segments);
    }

    let candidate = (
        first.start,
        fitted.into_iter().map(SvgPathSegment::Cubic).collect(),
    );
    let current_error =
        pixel_potrace_candidate_mask_error(path, &(start, segments.clone()), width, height);
    let candidate_error = pixel_potrace_candidate_mask_error(path, &candidate, width, height);
    if candidate_error < current_error {
        candidate
    } else {
        (start, segments)
    }
}

pub(crate) fn choose_pixel_potrace_point_set(
    path: &TracePath,
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let mut best = pixel_potrace_segments_for_points(
        path,
        &path.points,
        opt_tolerance,
        canvas_size,
        has_holes,
    )?;
    let simplified = simplify_collinear_float_points(&path.points);

    if simplified.len() >= 3 && simplified.len() < path.points.len() {
        if let Some(candidate) = pixel_potrace_segments_for_points(
            path,
            &simplified,
            opt_tolerance,
            canvas_size,
            has_holes,
        ) {
            if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                best = candidate;
            }
        }
    }

    Some(best)
}

pub(crate) fn pixel_potrace_segments_for_points(
    reference_path: &TracePath,
    points: &[(f64, f64)],
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let polygon = optimal_potrace_polygon_indices(points);
    let vertices = adjust_potrace_vertices(points, &polygon, 0.5);
    let (start, segments) = smooth_potrace_vertices(&vertices)?;

    Some(choose_pixel_potrace_segments(
        reference_path,
        start,
        segments,
        opt_tolerance,
        canvas_size,
        has_holes,
    ))
}

pub(crate) fn simplify_collinear_float_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    const EPSILON: f64 = 1.0e-9;

    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut simplified = Vec::new();
    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        let incoming = subtract(current, previous);
        let outgoing = subtract(next, current);

        if cross(incoming, outgoing).abs() > EPSILON {
            simplified.push(current);
        }
    }

    simplified
}

pub(crate) fn choose_pixel_potrace_segments(
    path: &TracePath,
    start: (f64, f64),
    segments: Vec<SvgPathSegment>,
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let mut best = optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    );

    if path.points.len() >= 12 {
        let mut preserve_primitive = false;

        if has_holes {
            if let Some(ring_ellipse) =
                fit_closed_ring_ellipse_potrace_segments(&path.points, path.is_hole)
            {
                if let Some(first) = ring_ellipse.first() {
                    let candidate = (first.start(), ring_ellipse);
                    if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                        best = candidate;
                        preserve_primitive = true;
                    }
                }
            }
        }

        if let Some(triangle) = fit_closed_upright_triangle_potrace_segments(&path.points) {
            if let Some(first) = triangle.first() {
                let candidate = (first.start(), triangle);
                if pixel_potrace_primitive_candidate_is_close_enough(
                    path,
                    canvas_size,
                    &candidate,
                    &best,
                ) {
                    best = candidate;
                    preserve_primitive = true;
                }
            }
        }

        if !preserve_primitive {
            if let Some(diagonal_capsule) =
                fit_closed_diagonal_capsule_potrace_segments(&path.points)
            {
                if let Some(first) = diagonal_capsule.first() {
                    let candidate = (first.start(), diagonal_capsule);
                    if pixel_potrace_primitive_candidate_is_close_enough(
                        path,
                        canvas_size,
                        &candidate,
                        &best,
                    ) {
                        best = candidate;
                        preserve_primitive = true;
                    }
                }
            }
        }

        if !preserve_primitive {
            if let Some(small_ellipse) = fit_closed_small_ellipse_potrace_segments(&path.points) {
                if let Some(first) = small_ellipse.first() {
                    let candidate = (first.start(), small_ellipse);
                    if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                        best = candidate;
                        preserve_primitive = true;
                    }
                }
            }
        }

        if !preserve_primitive {
            if let Some(rounded_rect) = fit_closed_rounded_rect_potrace_segments(&path.points) {
                if let Some(first) = rounded_rect.first() {
                    let candidate = (first.start(), rounded_rect);
                    if pixel_potrace_rounded_rect_candidate_is_better(
                        path,
                        canvas_size,
                        &candidate,
                        &best,
                    ) {
                        best = candidate;
                        preserve_primitive = true;
                    }
                }
            }
        }

        if !preserve_primitive {
            if let Some(chevron) = fit_closed_chevron_potrace_segments(&path.points) {
                if let Some(first) = chevron.first() {
                    let candidate = (first.start(), chevron);
                    if pixel_potrace_template_candidate_is_better(
                        path,
                        canvas_size,
                        &candidate,
                        &best,
                    ) {
                        best = candidate;
                        preserve_primitive = true;
                    }
                }
            }
        }

        if !preserve_primitive {
            if let Some(staple) = fit_closed_staple_potrace_segments(&path.points) {
                if let Some(first) = staple.first() {
                    let candidate = (first.start(), staple);
                    if pixel_potrace_template_candidate_is_better(
                        path,
                        canvas_size,
                        &candidate,
                        &best,
                    ) {
                        best = candidate;
                        preserve_primitive = true;
                    }
                }
            }
        }

        if !preserve_primitive {
            if let Some(stepped_f) = fit_closed_stepped_f_potrace_segments(&path.points) {
                if let Some(first) = stepped_f.first() {
                    let candidate = (first.start(), stepped_f);
                    if pixel_potrace_template_candidate_is_better(
                        path,
                        canvas_size,
                        &candidate,
                        &best,
                    ) {
                        best = candidate;
                        preserve_primitive = true;
                    }
                }
            }
        }

        if !preserve_primitive {
            if let Some(open_ring) = fit_closed_open_ring_potrace_segments(&path.points) {
                if let Some(first) = open_ring.first() {
                    let candidate = (first.start(), open_ring);
                    if pixel_potrace_template_candidate_is_better(
                        path,
                        canvas_size,
                        &candidate,
                        &best,
                    ) {
                        best = candidate;
                        preserve_primitive = true;
                    }
                }
            }
        }

        if !preserve_primitive {
            if let Some(primitive) = fit_closed_potrace_primitive_segments(&path.points) {
                if let Some(first) = primitive.first() {
                    let candidate = optimize_potrace_segments(
                        first.start(),
                        &primitive,
                        opt_tolerance,
                        PIXEL_POTRACE_LINEAR_DEVIATION,
                    );
                    if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best)
                        || pixel_potrace_fitted_candidate_is_close_enough(
                            path,
                            canvas_size,
                            &candidate,
                            &best,
                        )
                    {
                        best = candidate;
                    }
                }
            }
        }

        if !preserve_primitive {
            let fitted = fit_closed_smooth_potrace_segments(&path.points, false);
            if let Some(first) = fitted.first() {
                let candidate = optimize_potrace_segments(
                    first.start(),
                    &fitted,
                    opt_tolerance,
                    PIXEL_POTRACE_LINEAR_DEVIATION,
                );
                if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                    best = candidate;
                }
            }
        }
    }

    best
}

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

pub(crate) fn optimal_potrace_polygon_indices(points: &[(f64, f64)]) -> Vec<usize> {
    if points.len() > 3 && distance_squared_float(points[0], points[points.len() - 1]) <= 1.0e-12 {
        return optimal_potrace_polygon_indices(&points[..points.len() - 1]);
    }

    if points.len() <= 8 {
        return (0..points.len()).collect();
    }

    if !points_are_half_pixel_quantized(points) {
        return legacy_potrace_polygon_indices(points);
    }

    let mut best: Option<PolygonCandidate> = None;
    for rotation in polygon_rotation_candidates(points) {
        let rotated = rotate_float_points(points, rotation);
        let Some(candidate) = best_polygon_for_rotated_points(&rotated) else {
            continue;
        };
        let indices = candidate
            .indices
            .iter()
            .map(|index| (index + rotation) % points.len())
            .collect::<Vec<_>>();
        let candidate = PolygonCandidate {
            indices,
            segments: candidate.segments,
            penalty: candidate.penalty,
        };

        if best
            .as_ref()
            .is_none_or(|current| polygon_candidate_is_better(&candidate, current))
        {
            best = Some(candidate);
        }
    }

    best.map(|candidate| candidate.indices)
        .filter(|indices| indices.len() >= 3)
        .unwrap_or_else(|| (0..points.len()).collect())
}

pub(crate) fn legacy_potrace_polygon_indices(points: &[(f64, f64)]) -> Vec<usize> {
    const POLYGON_TOLERANCE: f64 = 0.75;

    let n = points.len();
    let mut dp: Vec<Option<PolygonDpState>> = vec![None; n + 1];
    dp[0] = Some(PolygonDpState {
        previous: 0,
        segments: 0,
        penalty: 0.0,
    });

    for start in 0..n {
        let Some(state) = dp[start] else {
            continue;
        };

        let mut end = start + 1;
        while end <= n {
            if end == n && state.segments < 2 {
                end += 1;
                continue;
            }

            if !legacy_potrace_arc_is_straight(points, start, end, POLYGON_TOLERANCE) {
                if end == start + 1 {
                    end += 1;
                    continue;
                }
                break;
            }

            let penalty =
                state.penalty + legacy_potrace_polygon_segment_penalty(points, start, end);
            let candidate = PolygonDpState {
                previous: start,
                segments: state.segments + 1,
                penalty,
            };

            if dp[end].is_none_or(|best| polygon_dp_state_is_better(candidate, best)) {
                dp[end] = Some(candidate);
            }

            end += 1;
        }
    }

    let Some(_) = dp[n] else {
        return (0..points.len()).collect();
    };

    let mut indices = Vec::new();
    let mut cursor = n;

    while cursor != 0 {
        let state = dp[cursor].expect("legacy dp cursor should be reachable");
        indices.push(state.previous % n);
        cursor = state.previous;
    }

    indices.reverse();
    indices.dedup();

    if indices.len() < 3 {
        (0..points.len()).collect()
    } else {
        indices
    }
}

pub(crate) fn legacy_potrace_arc_is_straight(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
    tolerance: f64,
) -> bool {
    if end <= start + 1 {
        return true;
    }

    let start_point = closed_point(points, start);
    let end_point = closed_point(points, end);
    let tolerance_squared = tolerance * tolerance;

    for index in start + 1..end {
        let point = closed_point(points, index);
        if distance_squared_to_segment(point, start_point, end_point).0 > tolerance_squared {
            return false;
        }
    }

    true
}

pub(crate) fn legacy_potrace_polygon_segment_penalty(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
) -> f64 {
    if end <= start + 1 {
        return 0.0;
    }

    let start_point = closed_point(points, start);
    let end_point = closed_point(points, end);

    (start + 1..end)
        .map(|index| {
            distance_squared_to_segment(closed_point(points, index), start_point, end_point).0
        })
        .sum()
}

#[derive(Debug, Clone)]
pub(crate) struct PolygonCandidate {
    indices: Vec<usize>,
    segments: usize,
    penalty: f64,
}

pub(crate) fn polygon_candidate_is_better(
    candidate: &PolygonCandidate,
    best: &PolygonCandidate,
) -> bool {
    candidate.segments < best.segments
        || (candidate.segments == best.segments && candidate.penalty < best.penalty)
}

pub(crate) fn polygon_rotation_candidates(points: &[(f64, f64)]) -> Vec<usize> {
    const MAX_ROTATIONS: usize = 24;

    if points.len() <= MAX_ROTATIONS {
        return (0..points.len()).collect();
    }

    let mut scored = (0..points.len())
        .map(|index| {
            let previous = points[(index + points.len() - 1) % points.len()];
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            let turn = vector_turn_angle(subtract(current, previous), subtract(next, current));
            (index, turn)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut candidates = Vec::new();
    candidates.push(0);
    for (index, turn) in scored {
        if turn <= 1.0e-6 {
            continue;
        }

        if !candidates.contains(&index) {
            candidates.push(index);
        }

        if candidates.len() >= MAX_ROTATIONS {
            break;
        }
    }

    let stride = (points.len() / MAX_ROTATIONS).max(1);
    for index in (0..points.len()).step_by(stride) {
        if candidates.len() >= MAX_ROTATIONS {
            break;
        }
        if !candidates.contains(&index) {
            candidates.push(index);
        }
    }

    candidates
}

pub(crate) fn rotate_float_points(points: &[(f64, f64)], start_index: usize) -> Vec<(f64, f64)> {
    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

pub(crate) fn best_polygon_for_rotated_points(points: &[(f64, f64)]) -> Option<PolygonCandidate> {
    let n = points.len();
    let sums = PathSums::for_closed_points(points);
    let mut dp: Vec<Option<PolygonDpState>> = vec![None; n + 1];
    dp[0] = Some(PolygonDpState {
        previous: 0,
        segments: 0,
        penalty: 0.0,
    });

    for start in 0..n {
        let Some(state) = dp[start] else {
            continue;
        };

        let mut end = start + 1;
        while end <= n {
            if end - start > n.saturating_sub(3) {
                break;
            }

            if !potrace_possible_segment_is_straight(points, start, end) {
                if end == start + 1 {
                    end += 1;
                    continue;
                }
                break;
            }

            let penalty =
                state.penalty + potrace_polygon_segment_penalty(points, &sums, start, end);
            let candidate = PolygonDpState {
                previous: start,
                segments: state.segments + 1,
                penalty,
            };

            if dp[end].is_none_or(|best| polygon_dp_state_is_better(candidate, best)) {
                dp[end] = Some(candidate);
            }

            end += 1;
        }
    }

    let final_state = dp[n]?;

    let mut indices = Vec::new();
    let mut cursor = n;

    while cursor != 0 {
        let state = dp[cursor].expect("dp cursor should be reachable");
        indices.push(state.previous % n);
        cursor = state.previous;
    }

    indices.reverse();
    indices.dedup();

    if indices.len() < 3 {
        None
    } else {
        Some(PolygonCandidate {
            indices,
            segments: final_state.segments,
            penalty: final_state.penalty,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PolygonDpState {
    previous: usize,
    segments: usize,
    penalty: f64,
}

pub(crate) fn polygon_dp_state_is_better(candidate: PolygonDpState, best: PolygonDpState) -> bool {
    candidate.segments < best.segments
        || (candidate.segments == best.segments && candidate.penalty < best.penalty)
}

#[derive(Debug, Clone)]
pub(crate) struct PathSums {
    x: Vec<f64>,
    y: Vec<f64>,
    x2: Vec<f64>,
    xy: Vec<f64>,
    y2: Vec<f64>,
}

impl PathSums {
    fn for_closed_points(points: &[(f64, f64)]) -> Self {
        let count = points.len() * 2 + 1;
        let mut sums = Self {
            x: Vec::with_capacity(count + 1),
            y: Vec::with_capacity(count + 1),
            x2: Vec::with_capacity(count + 1),
            xy: Vec::with_capacity(count + 1),
            y2: Vec::with_capacity(count + 1),
        };
        sums.x.push(0.0);
        sums.y.push(0.0);
        sums.x2.push(0.0);
        sums.xy.push(0.0);
        sums.y2.push(0.0);

        for index in 0..count {
            let point = points[index % points.len()];
            sums.x.push(sums.x[index] + point.0);
            sums.y.push(sums.y[index] + point.1);
            sums.x2.push(sums.x2[index] + point.0 * point.0);
            sums.xy.push(sums.xy[index] + point.0 * point.1);
            sums.y2.push(sums.y2[index] + point.1 * point.1);
        }

        sums
    }

    fn range(&self, start: usize, end: usize) -> PathSumRange {
        let end = end + 1;
        PathSumRange {
            count: (end - start) as f64,
            x: self.x[end] - self.x[start],
            y: self.y[end] - self.y[start],
            x2: self.x2[end] - self.x2[start],
            xy: self.xy[end] - self.xy[start],
            y2: self.y2[end] - self.y2[start],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PathSumRange {
    count: f64,
    x: f64,
    y: f64,
    x2: f64,
    xy: f64,
    y2: f64,
}

pub(crate) fn potrace_possible_segment_is_straight(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
) -> bool {
    if end <= start + 1 {
        return true;
    }

    if end - start > points.len().saturating_sub(3) {
        return false;
    }

    potrace_subpath_is_straight(points, start as isize - 1, end as isize + 1)
}

pub(crate) fn potrace_subpath_is_straight(points: &[(f64, f64)], start: isize, end: isize) -> bool {
    const MAX_DISTANCE: f64 = 1.0;

    if end <= start + 2 {
        return true;
    }

    if potrace_subpath_uses_all_four_directions(points, start, end) {
        return false;
    }

    let start_point = cyclic_point(points, start);
    let end_point = cyclic_point(points, end);
    if distance_squared_float(start_point, end_point) <= f64::EPSILON {
        return false;
    }

    for index in (start + 1)..end {
        let point = cyclic_point(points, index);
        if max_distance_to_infinite_line(point, start_point, end_point) > MAX_DISTANCE {
            return false;
        }
    }

    true
}

pub(crate) fn potrace_subpath_uses_all_four_directions(
    points: &[(f64, f64)],
    start: isize,
    end: isize,
) -> bool {
    let mut mask = 0u8;

    for index in start..end {
        let from = cyclic_point(points, index);
        let to = cyclic_point(points, index + 1);
        mask |= cardinal_direction_mask(subtract(to, from));
        if mask == 0b1111 {
            return true;
        }
    }

    false
}

pub(crate) fn cardinal_direction_mask(vector: (f64, f64)) -> u8 {
    if vector.0.abs() <= f64::EPSILON && vector.1.abs() <= f64::EPSILON {
        return 0;
    }

    if vector.0.abs() >= vector.1.abs() {
        if vector.0 >= 0.0 {
            0b0001
        } else {
            0b0010
        }
    } else if vector.1 >= 0.0 {
        0b0100
    } else {
        0b1000
    }
}

pub(crate) fn max_distance_to_infinite_line(
    point: (f64, f64),
    line_start: (f64, f64),
    line_end: (f64, f64),
) -> f64 {
    let line = subtract(line_end, line_start);
    let length_squared = vector_length_squared(line);

    if length_squared <= f64::EPSILON {
        return (point.0 - line_start.0)
            .abs()
            .max((point.1 - line_start.1).abs());
    }

    let amount = dot(subtract(point, line_start), line) / length_squared;
    let projection = add(line_start, scale(line, amount));
    (point.0 - projection.0)
        .abs()
        .max((point.1 - projection.1).abs())
}

pub(crate) fn cyclic_point(points: &[(f64, f64)], index: isize) -> (f64, f64) {
    let len = points.len() as isize;
    let index = index.rem_euclid(len) as usize;
    points[index]
}

pub(crate) fn potrace_polygon_segment_penalty(
    points: &[(f64, f64)],
    sums: &PathSums,
    start: usize,
    end: usize,
) -> f64 {
    if end <= start + 1 {
        return 0.0;
    }

    let start_point = closed_point(points, start);
    let end_point = closed_point(points, end);
    let chord = subtract(end_point, start_point);
    let range = sums.range(start, end);
    let a = -chord.1;
    let b = chord.0;
    let c = chord.1 * start_point.0 - chord.0 * start_point.1;
    let squared_error = a * a * range.x2
        + 2.0 * a * b * range.xy
        + b * b * range.y2
        + 2.0 * a * c * range.x
        + 2.0 * b * c * range.y
        + range.count * c * c;

    (squared_error.max(0.0) / range.count).sqrt()
}

pub(crate) fn adjust_potrace_vertices(
    points: &[(f64, f64)],
    polygon: &[usize],
    max_vertex_adjustment: f64,
) -> Vec<(f64, f64)> {
    if polygon.len() < 3 {
        return polygon.iter().map(|index| points[*index]).collect();
    }

    let mut adjusted = Vec::with_capacity(polygon.len());

    for index in 0..polygon.len() {
        let previous = polygon[(index + polygon.len() - 1) % polygon.len()];
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        let incoming = best_fit_line_for_closed_arc(points, previous, current);
        let outgoing = best_fit_line_for_closed_arc(points, current, next);
        let vertex = line_intersection(incoming, outgoing)
            .map(|point| clamp_point_to_box(point, points[current], max_vertex_adjustment))
            .unwrap_or(points[current]);

        adjusted.push(vertex);
    }

    adjusted
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FitLine {
    point: (f64, f64),
    direction: (f64, f64),
}

pub(crate) fn best_fit_line_for_closed_arc(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
) -> FitLine {
    let arc = closed_arc_points_by_index(points, start, end);

    if arc.len() <= 2 {
        return FitLine {
            point: arc[0],
            direction: unit_vector(subtract(*arc.last().unwrap_or(&arc[0]), arc[0])),
        };
    }

    let centroid = arc_centroid(&arc);
    let mut xx = 0.0;
    let mut xy = 0.0;
    let mut yy = 0.0;

    for point in &arc {
        let centered = subtract(*point, centroid);
        xx += centered.0 * centered.0;
        xy += centered.0 * centered.1;
        yy += centered.1 * centered.1;
    }

    let fallback = unit_vector(subtract(*arc.last().unwrap_or(&arc[0]), arc[0]));
    let direction = principal_axis_2x2(xx, xy, yy).unwrap_or(fallback);

    FitLine {
        point: centroid,
        direction,
    }
}

pub(crate) fn closed_arc_points_by_index(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
) -> Vec<(f64, f64)> {
    let mut arc = Vec::new();
    let mut index = start;

    loop {
        arc.push(points[index]);

        if index == end {
            break;
        }

        index = (index + 1) % points.len();
    }

    arc
}

pub(crate) fn arc_centroid(points: &[(f64, f64)]) -> (f64, f64) {
    let sum = points.iter().copied().fold((0.0, 0.0), add);

    scale(sum, 1.0 / points.len() as f64)
}

pub(crate) fn largest_eigenvalue_2x2(xx: f64, xy: f64, yy: f64) -> f64 {
    let trace = xx + yy;
    let determinant = xx * yy - xy * xy;
    let discriminant = (trace * trace - 4.0 * determinant).max(0.0).sqrt();

    (trace + discriminant) / 2.0
}

pub(crate) fn principal_axis_2x2(xx: f64, xy: f64, yy: f64) -> Option<(f64, f64)> {
    if xx.abs() <= f64::EPSILON && xy.abs() <= f64::EPSILON && yy.abs() <= f64::EPSILON {
        return None;
    }

    let lambda = largest_eigenvalue_2x2(xx, xy, yy);
    let candidates = [(xy, lambda - xx), (lambda - yy, xy)];

    candidates
        .into_iter()
        .find(|candidate| vector_length_squared(*candidate) > f64::EPSILON)
        .map(unit_vector)
        .or({
            if xx >= yy {
                Some((1.0, 0.0))
            } else {
                Some((0.0, 1.0))
            }
        })
}

pub(crate) fn line_intersection(a: FitLine, b: FitLine) -> Option<(f64, f64)> {
    let denominator = cross(a.direction, b.direction);

    if denominator.abs() <= 1.0e-9 {
        return None;
    }

    let amount = cross(subtract(b.point, a.point), b.direction) / denominator;
    Some(add(a.point, scale(a.direction, amount)))
}

pub(crate) fn clamp_point_to_box(point: (f64, f64), center: (f64, f64), radius: f64) -> (f64, f64) {
    (
        point.0.clamp(center.0 - radius, center.0 + radius),
        point.1.clamp(center.1 - radius, center.1 + radius),
    )
}

pub(crate) fn smooth_potrace_vertices(
    points: &[(f64, f64)],
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    const ALPHA_MIN: f64 = 0.55;
    const ALPHA_MAX: f64 = 1.0;

    if points.len() < 3 {
        return None;
    }

    let first = edge_midpoint(points[points.len() - 1], points[0]);
    let mut segments = Vec::new();
    let mut start = first;

    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let vertex = points[index];
        let next = points[(index + 1) % points.len()];
        let entry = edge_midpoint(previous, vertex);
        let exit = edge_midpoint(vertex, next);
        let alpha = potrace_curve_alpha(previous, vertex, next);

        if alpha > ALPHA_MAX {
            segments.push(SvgPathSegment::Line { start, end: vertex });
            segments.push(SvgPathSegment::Line {
                start: vertex,
                end: exit,
            });
        } else {
            let alpha = alpha.clamp(ALPHA_MIN, ALPHA_MAX);
            segments.push(SvgPathSegment::Cubic(CubicSegment {
                start: entry,
                control1: interpolate(entry, vertex, alpha),
                control2: interpolate(exit, vertex, alpha),
                end: exit,
            }));
        }

        start = exit;
    }

    Some((first, segments))
}

pub(crate) fn closed_point(points: &[(f64, f64)], index: usize) -> (f64, f64) {
    points[index % points.len()]
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SvgPathSegment {
    Line { start: (f64, f64), end: (f64, f64) },
    Cubic(CubicSegment),
}

impl SvgPathSegment {
    pub(crate) fn start(self) -> (f64, f64) {
        match self {
            SvgPathSegment::Line { start, .. } => start,
            SvgPathSegment::Cubic(cubic) => cubic.start,
        }
    }

    pub(crate) fn end(self) -> (f64, f64) {
        match self {
            SvgPathSegment::Line { end, .. } => end,
            SvgPathSegment::Cubic(cubic) => cubic.end,
        }
    }
}

pub(crate) fn optimize_potrace_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
    opt_tolerance: f64,
    max_linear_deviation: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    if segments.len() < 3 {
        return (start, segments.to_vec());
    }

    if segments
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        let optimized = optimize_closed_potrace_curve_run(
            &segments
                .iter()
                .filter_map(|segment| match segment {
                    SvgPathSegment::Cubic(cubic) => Some(*cubic),
                    SvgPathSegment::Line { .. } => None,
                })
                .collect::<Vec<_>>(),
            opt_tolerance,
        );

        return finish_potrace_segments(start, optimized, opt_tolerance, max_linear_deviation);
    }

    let (start, optimized) = optimize_mixed_potrace_curve_runs_once(start, segments, opt_tolerance);
    finish_potrace_segments(start, optimized, opt_tolerance, max_linear_deviation)
}

pub(crate) fn finish_potrace_segments(
    start: (f64, f64),
    segments: Vec<SvgPathSegment>,
    opt_tolerance: f64,
    max_linear_deviation: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let optimized = cleanup_potrace_segments(segments, max_linear_deviation);
    let start = cleanup_potrace_start(start, &optimized);
    if optimized
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        return (start, optimized);
    }

    let (start, optimized) =
        optimize_mixed_potrace_curve_runs_once(start, &optimized, opt_tolerance);
    let optimized = cleanup_potrace_segments(optimized, max_linear_deviation);
    let start = cleanup_potrace_start(start, &optimized);
    (start, optimized)
}

pub(crate) fn cleanup_potrace_start(start: (f64, f64), segments: &[SvgPathSegment]) -> (f64, f64) {
    segments.first().map_or(start, |segment| segment.start())
}

pub(crate) fn optimize_mixed_potrace_curve_runs_once(
    start: (f64, f64),
    segments: &[SvgPathSegment],
    opt_tolerance: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let rotated = rotate_potrace_segments_after_last_line(segments);
    let start = rotated.first().map_or(start, |segment| segment.start());
    let mut optimized = Vec::new();
    let mut curve_run = Vec::new();

    for segment in rotated {
        match segment {
            SvgPathSegment::Cubic(cubic) => curve_run.push(cubic),
            SvgPathSegment::Line { .. } => {
                flush_potrace_curve_run(&mut optimized, &mut curve_run, opt_tolerance);
                optimized.push(segment);
            }
        }
    }

    flush_potrace_curve_run(&mut optimized, &mut curve_run, opt_tolerance);
    (start, optimized)
}

pub(crate) fn cleanup_potrace_segments(
    segments: Vec<SvgPathSegment>,
    max_linear_deviation: f64,
) -> Vec<SvgPathSegment> {
    let optimized = prune_tiny_potrace_curve_segments(segments);
    let optimized = regularize_potrace_orthogonal_corners(optimized);
    let optimized = demote_nearly_linear_potrace_cubics(optimized, max_linear_deviation);
    let optimized = snap_near_axis_potrace_lines(optimized);
    merge_collinear_potrace_lines(optimized)
}

pub(crate) fn snap_near_axis_potrace_lines(segments: Vec<SvgPathSegment>) -> Vec<SvgPathSegment> {
    const MAX_AXIS_DRIFT: f64 = 0.75;

    if segments.len() < 2 {
        return segments;
    }

    let mut nodes = Vec::with_capacity(segments.len() + 1);
    nodes.push(segments[0].start());
    nodes.extend(segments.iter().map(|segment| segment.end()));

    let mut x_constraints: Vec<(f64, usize)> = vec![(0.0, 0); nodes.len()];
    let mut y_constraints: Vec<(f64, usize)> = vec![(0.0, 0); nodes.len()];

    for (index, segment) in segments.iter().enumerate() {
        let SvgPathSegment::Line { start, end } = *segment else {
            continue;
        };

        let dx = (end.0 - start.0).abs();
        let dy = (end.1 - start.1).abs();
        if dx <= MAX_AXIS_DRIFT && dy > MAX_AXIS_DRIFT {
            let snapped_x = (start.0 + end.0) / 2.0;
            add_axis_constraint(&mut x_constraints[index], snapped_x);
            add_axis_constraint(&mut x_constraints[index + 1], snapped_x);
        } else if dy <= MAX_AXIS_DRIFT && dx > MAX_AXIS_DRIFT {
            let snapped_y = (start.1 + end.1) / 2.0;
            add_axis_constraint(&mut y_constraints[index], snapped_y);
            add_axis_constraint(&mut y_constraints[index + 1], snapped_y);
        }
    }

    if compact_segments_are_closed(nodes[0], &segments) {
        merge_closed_axis_constraints(&mut x_constraints);
        merge_closed_axis_constraints(&mut y_constraints);
    }

    let mut changed = false;
    for index in 0..nodes.len() {
        if x_constraints[index].1 > 0 {
            let snapped = x_constraints[index].0 / x_constraints[index].1 as f64;
            changed |= (nodes[index].0 - snapped).abs() > 1.0e-9;
            nodes[index].0 = snapped;
        }
        if y_constraints[index].1 > 0 {
            let snapped = y_constraints[index].0 / y_constraints[index].1 as f64;
            changed |= (nodes[index].1 - snapped).abs() > 1.0e-9;
            nodes[index].1 = snapped;
        }
    }

    if !changed {
        return segments;
    }

    segments
        .into_iter()
        .enumerate()
        .map(|(index, segment)| snap_segment_endpoints(segment, nodes[index], nodes[index + 1]))
        .collect()
}

pub(crate) fn add_axis_constraint(constraint: &mut (f64, usize), value: f64) {
    constraint.0 += value;
    constraint.1 += 1;
}

pub(crate) fn merge_closed_axis_constraints(constraints: &mut [(f64, usize)]) {
    if constraints.len() < 2 {
        return;
    }

    let last = constraints.len() - 1;
    let sum = constraints[0].0 + constraints[last].0;
    let count = constraints[0].1 + constraints[last].1;
    constraints[0] = (sum, count);
    constraints[last] = (sum, count);
}

pub(crate) fn snap_segment_endpoints(
    segment: SvgPathSegment,
    snapped_start: (f64, f64),
    snapped_end: (f64, f64),
) -> SvgPathSegment {
    match segment {
        SvgPathSegment::Line { .. } => SvgPathSegment::Line {
            start: snapped_start,
            end: snapped_end,
        },
        SvgPathSegment::Cubic(cubic) => {
            let start_delta = subtract(snapped_start, cubic.start);
            let end_delta = subtract(snapped_end, cubic.end);
            SvgPathSegment::Cubic(CubicSegment {
                start: snapped_start,
                control1: add(cubic.control1, start_delta),
                control2: add(cubic.control2, end_delta),
                end: snapped_end,
            })
        }
    }
}

pub(crate) fn merge_collinear_potrace_lines(segments: Vec<SvgPathSegment>) -> Vec<SvgPathSegment> {
    if segments.len() < 2 {
        return segments;
    }

    let mut merged: Vec<SvgPathSegment> = Vec::with_capacity(segments.len());

    for segment in segments {
        if let Some(previous) = merged.last_mut() {
            if let Some(combined) = merge_collinear_potrace_line_pair(*previous, segment) {
                *previous = combined;
                continue;
            }
        }

        merged.push(segment);
    }

    merged
}

pub(crate) fn merge_collinear_potrace_line_pair(
    previous: SvgPathSegment,
    current: SvgPathSegment,
) -> Option<SvgPathSegment> {
    let (
        SvgPathSegment::Line { start, end: middle },
        SvgPathSegment::Line {
            start: current_start,
            end,
        },
    ) = (previous, current)
    else {
        return None;
    };

    if distance_squared_float(middle, current_start) > 1.0e-12 {
        return None;
    }

    let first = subtract(middle, start);
    let second = subtract(end, middle);
    if vector_length_squared(first) <= f64::EPSILON
        || vector_length_squared(second) <= f64::EPSILON
        || cross(first, second).abs() > 1.0e-9
        || dot(first, second) < 0.0
    {
        return None;
    }

    Some(SvgPathSegment::Line { start, end })
}

pub(crate) fn demote_nearly_linear_potrace_cubics(
    segments: Vec<SvgPathSegment>,
    max_linear_deviation: f64,
) -> Vec<SvgPathSegment> {
    segments
        .into_iter()
        .map(|segment| match segment {
            SvgPathSegment::Cubic(cubic)
                if potrace_cubic_is_nearly_linear(cubic, max_linear_deviation) =>
            {
                SvgPathSegment::Line {
                    start: cubic.start,
                    end: cubic.end,
                }
            }
            segment => segment,
        })
        .collect()
}

pub(crate) const STRICT_POTRACE_LINEAR_DEVIATION: f64 = 0.25;
pub(crate) const PIXEL_POTRACE_LINEAR_DEVIATION: f64 = 1.0;

pub(crate) fn potrace_cubic_is_nearly_linear(
    cubic: CubicSegment,
    max_linear_deviation: f64,
) -> bool {
    const MIN_LINEAR_LENGTH: f64 = 16.0;

    cubic_chord_length(cubic) >= MIN_LINEAR_LENGTH
        && cubic_chord_deviation(cubic) <= max_linear_deviation
}

pub(crate) fn prune_tiny_potrace_curve_segments(
    segments: Vec<SvgPathSegment>,
) -> Vec<SvgPathSegment> {
    if segments.len() < 5 {
        return segments;
    }

    let mut pruned = Vec::with_capacity(segments.len());
    for index in 0..segments.len() {
        if potrace_segment_is_tiny_spike(&segments, index) {
            continue;
        }

        pruned.push(segments[index]);
    }

    if pruned.len() >= 3 && pruned.len() < segments.len() {
        pruned
    } else {
        segments
    }
}

pub(crate) fn potrace_segment_is_tiny_spike(segments: &[SvgPathSegment], index: usize) -> bool {
    const TINY_CHORD_LENGTH: f64 = 2.1;
    const TINY_BOUNDS_DIAGONAL: f64 = 2.1;
    const MIN_NEIGHBOR_CHORD_LENGTH: f64 = 4.0;

    if segments.len() < 3 {
        return false;
    }

    let previous_index = (index + segments.len() - 1) % segments.len();
    let next_index = (index + 1) % segments.len();
    let (
        SvgPathSegment::Cubic(previous),
        SvgPathSegment::Cubic(current),
        SvgPathSegment::Cubic(next),
    ) = (
        segments[previous_index],
        segments[index],
        segments[next_index],
    )
    else {
        return false;
    };

    if distance_squared_float(previous.end, current.start) > 1.0e-9
        || distance_squared_float(current.end, next.start) > 1.0e-9
    {
        return false;
    }

    cubic_chord_length(current) <= TINY_CHORD_LENGTH
        && cubic_bounds_diagonal(current) <= TINY_BOUNDS_DIAGONAL
        && cubic_chord_length(previous) >= MIN_NEIGHBOR_CHORD_LENGTH
        && cubic_chord_length(next) >= MIN_NEIGHBOR_CHORD_LENGTH
        && potrace_segment_has_spike_turn(previous, current, next)
}

pub(crate) fn potrace_segment_has_spike_turn(
    previous: CubicSegment,
    current: CubicSegment,
    next: CubicSegment,
) -> bool {
    const MIN_SPIKE_TURN_RADIANS: f64 = 1.0;
    const MIN_BRIDGED_TURN_RADIANS: f64 = 0.35;

    let previous_vector = cubic_chord_vector(previous);
    let current_vector = cubic_chord_vector(current);
    let next_vector = cubic_chord_vector(next);
    let entry_turn = vector_turn_angle(previous_vector, current_vector);
    let exit_turn = vector_turn_angle(current_vector, next_vector);
    let bridged_turn = vector_turn_angle(previous_vector, next_vector);

    entry_turn.max(exit_turn) >= MIN_SPIKE_TURN_RADIANS
        && (bridged_turn >= MIN_BRIDGED_TURN_RADIANS
            || (entry_turn >= MIN_SPIKE_TURN_RADIANS && exit_turn >= MIN_SPIKE_TURN_RADIANS))
}

pub(crate) fn regularize_potrace_orthogonal_corners(
    segments: Vec<SvgPathSegment>,
) -> Vec<SvgPathSegment> {
    if segments.len() < 5 {
        return segments;
    }

    let mut regularized = Vec::with_capacity(segments.len());
    let mut index = 0usize;
    let mut changed = false;

    while index < segments.len() {
        if let Some(cubic) = regularized_potrace_corner_pair(&segments, index) {
            regularized.push(SvgPathSegment::Cubic(cubic));
            changed = true;
            index += 2;
            continue;
        }

        if let Some(cubic) = regularized_potrace_corner(&segments, index) {
            regularized.push(SvgPathSegment::Cubic(cubic));
            changed = true;
            index += 1;
            continue;
        }

        regularized.push(segments[index]);
        index += 1;
    }

    if changed && regularized.len() >= 3 {
        regularized
    } else {
        segments
    }
}

pub(crate) fn regularized_potrace_corner_pair(
    segments: &[SvgPathSegment],
    index: usize,
) -> Option<CubicSegment> {
    const MAX_LEAD_TURN_RADIANS: f64 = 0.35;

    if index == 0 || index + 2 >= segments.len() {
        return None;
    }

    let (
        SvgPathSegment::Cubic(previous),
        SvgPathSegment::Cubic(lead),
        SvgPathSegment::Cubic(turn),
        SvgPathSegment::Cubic(next),
    ) = (
        segments[index - 1],
        segments[index],
        segments[index + 1],
        segments[index + 2],
    )
    else {
        return None;
    };

    if !potrace_segment_is_straight_edge(previous)
        || !potrace_segment_is_straight_edge(next)
        || !potrace_segment_is_short_straight_lead(lead)
        || !potrace_segment_is_roundable_corner(turn)
    {
        return None;
    }

    let previous_vector = cubic_chord_vector(previous);
    let lead_vector = cubic_chord_vector(lead);
    let next_vector = cubic_chord_vector(next);
    if vector_turn_angle(previous_vector, lead_vector) > MAX_LEAD_TURN_RADIANS
        || !vectors_are_roughly_orthogonal(previous_vector, next_vector)
    {
        return None;
    }

    let candidate = tangent_corner_cubic(lead.start, turn.end, previous_vector, next_vector)?;
    potrace_regularized_corner_is_close(&[lead, turn], candidate, 5.0).then_some(candidate)
}

pub(crate) fn regularized_potrace_corner(
    segments: &[SvgPathSegment],
    index: usize,
) -> Option<CubicSegment> {
    if index == 0 || index + 1 >= segments.len() {
        return None;
    }

    let (
        SvgPathSegment::Cubic(previous),
        SvgPathSegment::Cubic(current),
        SvgPathSegment::Cubic(next),
    ) = (segments[index - 1], segments[index], segments[index + 1])
    else {
        return None;
    };

    if !potrace_segment_is_straight_edge(previous)
        || !potrace_segment_is_straight_edge(next)
        || !potrace_segment_is_roundable_corner(current)
        || !vectors_are_roughly_orthogonal(cubic_chord_vector(previous), cubic_chord_vector(next))
    {
        return None;
    }

    let candidate = tangent_corner_cubic(
        current.start,
        current.end,
        cubic_chord_vector(previous),
        cubic_chord_vector(next),
    )?;
    potrace_regularized_corner_is_close(&[current], candidate, 3.5).then_some(candidate)
}

pub(crate) fn potrace_segment_is_straight_edge(cubic: CubicSegment) -> bool {
    const MIN_STRAIGHT_LENGTH: f64 = 40.0;
    const MAX_STRAIGHT_DEVIATION: f64 = 1.5;

    cubic_chord_length(cubic) >= MIN_STRAIGHT_LENGTH
        && cubic_chord_deviation(cubic) <= MAX_STRAIGHT_DEVIATION
}

pub(crate) fn potrace_segment_is_short_straight_lead(cubic: CubicSegment) -> bool {
    const MIN_LEAD_LENGTH: f64 = 4.0;
    const MAX_LEAD_LENGTH: f64 = 32.0;
    const MAX_LEAD_DEVIATION: f64 = 1.5;

    let length = cubic_chord_length(cubic);
    (MIN_LEAD_LENGTH..=MAX_LEAD_LENGTH).contains(&length)
        && cubic_chord_deviation(cubic) <= MAX_LEAD_DEVIATION
}

pub(crate) fn potrace_segment_is_roundable_corner(cubic: CubicSegment) -> bool {
    const MIN_CORNER_LENGTH: f64 = 6.0;
    const MAX_CORNER_LENGTH: f64 = 36.0;
    const MIN_CORNER_DEVIATION: f64 = 1.5;

    let length = cubic_chord_length(cubic);
    (MIN_CORNER_LENGTH..=MAX_CORNER_LENGTH).contains(&length)
        && cubic_chord_deviation(cubic) >= MIN_CORNER_DEVIATION
}

pub(crate) fn vectors_are_roughly_orthogonal(a: (f64, f64), b: (f64, f64)) -> bool {
    const MIN_ORTHOGONAL_TURN: f64 = 1.0;
    const MAX_ORTHOGONAL_TURN: f64 = 2.15;

    let turn = vector_turn_angle(a, b);
    (MIN_ORTHOGONAL_TURN..=MAX_ORTHOGONAL_TURN).contains(&turn)
}

pub(crate) fn tangent_corner_cubic(
    start: (f64, f64),
    end: (f64, f64),
    incoming: (f64, f64),
    outgoing: (f64, f64),
) -> Option<CubicSegment> {
    const CIRCLE_ARC_KAPPA: f64 = 0.552_284_749_830_793_6;
    const MIN_HANDLE_LENGTH: f64 = 2.0;

    let incoming = unit_vector(incoming);
    let outgoing = unit_vector(outgoing);
    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return None;
    }

    let delta = subtract(end, start);
    let incoming_projection = dot(delta, incoming);
    let outgoing_projection = dot(delta, outgoing);
    if incoming_projection <= 0.0 || outgoing_projection <= 0.0 {
        return None;
    }

    let handle = incoming_projection.min(outgoing_projection) * CIRCLE_ARC_KAPPA;
    if handle < MIN_HANDLE_LENGTH {
        return None;
    }

    Some(CubicSegment {
        start,
        control1: add(start, scale(incoming, handle)),
        control2: subtract(end, scale(outgoing, handle)),
        end,
    })
}

pub(crate) fn potrace_regularized_corner_is_close(
    source: &[CubicSegment],
    candidate: CubicSegment,
    tolerance: f64,
) -> bool {
    let samples = sample_cubic_run(source);
    cubic_runs_are_close(&samples, &[candidate], tolerance)
}

pub(crate) fn rotate_potrace_segments_after_last_line(
    segments: &[SvgPathSegment],
) -> Vec<SvgPathSegment> {
    let Some(line_index) = segments
        .iter()
        .rposition(|segment| matches!(segment, SvgPathSegment::Line { .. }))
    else {
        return segments.to_vec();
    };

    let start = (line_index + 1) % segments.len();
    segments[start..]
        .iter()
        .chain(segments[..start].iter())
        .copied()
        .collect()
}

pub(crate) fn optimize_closed_potrace_curve_run(
    run: &[CubicSegment],
    opt_tolerance: f64,
) -> Vec<SvgPathSegment> {
    const CLOSED_SPLITS: usize = 4;

    if run.len() < CLOSED_SPLITS * 2 {
        return run.iter().copied().map(SvgPathSegment::Cubic).collect();
    }

    let mut optimized = Vec::new();

    for split in 0..CLOSED_SPLITS {
        let start = split * run.len() / CLOSED_SPLITS;
        let end = (split + 1) * run.len() / CLOSED_SPLITS;
        append_optimized_potrace_curve_run(&mut optimized, &run[start..end], opt_tolerance);
    }

    optimized
}

pub(crate) fn flush_potrace_curve_run(
    output: &mut Vec<SvgPathSegment>,
    run: &mut Vec<CubicSegment>,
    opt_tolerance: f64,
) {
    append_optimized_potrace_curve_run(output, run, opt_tolerance);
    run.clear();
}

pub(crate) fn append_optimized_potrace_curve_run(
    output: &mut Vec<SvgPathSegment>,
    run: &[CubicSegment],
    opt_tolerance: f64,
) {
    if run.is_empty() {
        return;
    }

    if run.len() <= 1 {
        output.extend(run.iter().copied().map(SvgPathSegment::Cubic));
        return;
    }

    output.extend(
        optimize_potrace_curve_run_graph(run, opt_tolerance)
            .into_iter()
            .map(SvgPathSegment::Cubic),
    );
}

pub(crate) fn optimize_potrace_curve_run_graph(
    run: &[CubicSegment],
    opt_tolerance: f64,
) -> Vec<CubicSegment> {
    let mut dp: Vec<Option<OpticurveState>> = vec![None; run.len() + 1];
    let mut edges: Vec<Vec<OpticurveEdge>> = vec![Vec::new(); run.len()];
    dp[0] = Some(OpticurveState {
        previous: 0,
        edge_index: 0,
        segments: 0,
        penalty: 0.0,
    });

    for start in 0..run.len() {
        let Some(state) = dp[start] else {
            continue;
        };

        let mut end = start + 1;
        while end <= run.len() {
            let Some(edge) = opticurve_edge(run, start, end, opt_tolerance) else {
                end += 1;
                continue;
            };
            let edge_index = edges[start].len();
            edges[start].push(edge);
            let candidate = OpticurveState {
                previous: start,
                edge_index,
                segments: state.segments + 1,
                penalty: state.penalty + edge.penalty,
            };

            if dp[end].is_none_or(|best| opticurve_state_is_better(candidate, best)) {
                dp[end] = Some(candidate);
            }

            end += 1;
        }
    }

    let Some(_) = dp[run.len()] else {
        return run.to_vec();
    };

    let mut merged = Vec::new();
    let mut cursor = run.len();

    while cursor != 0 {
        let state = dp[cursor].expect("opticurve cursor should be reachable");
        let edge = edges[state.previous][state.edge_index];
        merged.push(edge.cubic);
        cursor = state.previous;
    }

    merged.reverse();

    if merged.len() <= run.len() {
        merged
    } else {
        run.to_vec()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OpticurveState {
    previous: usize,
    edge_index: usize,
    segments: usize,
    penalty: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OpticurveEdge {
    cubic: CubicSegment,
    penalty: f64,
}

pub(crate) fn opticurve_state_is_better(candidate: OpticurveState, best: OpticurveState) -> bool {
    candidate.segments < best.segments
        || (candidate.segments == best.segments && candidate.penalty < best.penalty)
}

pub(crate) fn opticurve_edge(
    run: &[CubicSegment],
    start: usize,
    end: usize,
    opt_tolerance: f64,
) -> Option<OpticurveEdge> {
    let opt_tolerance = opt_tolerance.max(0.0);
    if end <= start {
        return None;
    }

    if end == start + 1 {
        return Some(OpticurveEdge {
            cubic: run[start],
            penalty: 0.0,
        });
    }

    if !cubic_run_has_consistent_convexity(&run[start..end]) {
        return None;
    }

    if let Some(edge) = potrace_area_opticurve_edge(run, start, end, opt_tolerance) {
        return Some(edge);
    }

    let samples = sample_cubic_run(&run[start..end]);
    let mut fitted = Vec::new();
    fit_open_cubic_segments_raw(&samples, opt_tolerance * opt_tolerance, &mut fitted);

    if fitted.len() != 1 || !cubic_runs_are_close(&samples, &fitted, opt_tolerance) {
        return None;
    }

    Some(OpticurveEdge {
        cubic: fitted[0],
        penalty: cubic_run_fit_penalty(&samples, fitted[0]),
    })
}

pub(crate) fn cubic_run_has_consistent_convexity(run: &[CubicSegment]) -> bool {
    let mut sign = 0.0_f64;

    for cubic in run {
        let start_tangent = subtract(cubic.control1, cubic.start);
        let end_tangent = subtract(cubic.end, cubic.control2);
        let turn = cross(start_tangent, end_tangent);

        if turn.abs() <= 1.0e-9 {
            continue;
        }

        if sign == 0.0 {
            sign = turn.signum();
        } else if turn.signum() != sign {
            return false;
        }
    }

    true
}

pub(crate) fn cubic_run_fit_penalty(samples: &[(f64, f64)], cubic: CubicSegment) -> f64 {
    samples
        .iter()
        .map(|sample| distance_squared_to_cubic_segments(*sample, &[cubic]))
        .sum()
}

pub(crate) struct ReconstructedPotraceRun {
    vertices: Vec<(f64, f64)>,
    alphas: Vec<f64>,
}

impl ReconstructedPotraceRun {
    fn from_cubics(run: &[CubicSegment]) -> Option<Self> {
        let mut vertices = Vec::with_capacity(run.len());
        let mut alphas = Vec::with_capacity(run.len());

        for cubic in run {
            let vertex = potrace_cubic_vertex(*cubic)?;
            let alpha = potrace_cubic_alpha(*cubic, vertex)?;
            vertices.push(vertex);
            alphas.push(alpha);
        }

        Some(Self { vertices, alphas })
    }
}

pub(crate) fn potrace_area_opticurve_edge(
    run: &[CubicSegment],
    start: usize,
    end: usize,
    opt_tolerance: f64,
) -> Option<OpticurveEdge> {
    if end <= start + 1 {
        return None;
    }

    let reconstructed = ReconstructedPotraceRun::from_cubics(run)?;
    let p0 = run[start].start;
    let p1 = reconstructed.vertices[start];
    let p2 = reconstructed.vertices[end - 1];
    let p3 = run[end - 1].end;
    let area = reconstructed_potrace_curve_area(&reconstructed, run, start, end);
    let a1 = signed_area_twice(p0, p1, p2);
    let a2 = signed_area_twice(p0, p1, p3);
    let a3 = signed_area_twice(p0, p2, p3);
    let a4 = a1 + a3 - a2;
    let t_denominator = a3 - a4;
    let s_denominator = a2 - a1;
    if t_denominator.abs() <= f64::EPSILON || s_denominator.abs() <= f64::EPSILON {
        return None;
    }

    let t = a3 / t_denominator;
    let s = a2 / s_denominator;
    let triangle_area = a2 * t / 2.0;
    if triangle_area.abs() <= f64::EPSILON {
        return None;
    }

    let radicand = 4.0 - area / triangle_area / 0.3;
    if radicand < 0.0 {
        return None;
    }

    let alpha = 2.0 - radicand.sqrt();
    if !alpha.is_finite() {
        return None;
    }

    let candidate = CubicSegment {
        start: p0,
        control1: interpolate(p0, p1, t * alpha),
        control2: interpolate(p3, p2, s * alpha),
        end: p3,
    };
    let penalty =
        potrace_area_opticurve_penalty(&reconstructed, run, start, end, candidate, opt_tolerance)?;

    Some(OpticurveEdge {
        cubic: candidate,
        penalty,
    })
}

pub(crate) fn reconstructed_potrace_curve_area(
    reconstructed: &ReconstructedPotraceRun,
    run: &[CubicSegment],
    start: usize,
    end: usize,
) -> f64 {
    let reference = reconstructed.vertices[0];
    let edge_start = run[start].start;
    let edge_end = run[end - 1].end;
    let mut area = 0.0;

    for index in start..end {
        let previous_end = if index == start {
            edge_start
        } else {
            run[index - 1].end
        };
        let end_point = run[index].end;
        let vertex = reconstructed.vertices[index];
        let alpha = reconstructed.alphas[index];
        area +=
            0.3 * alpha * (4.0 - alpha) * signed_area_twice(previous_end, vertex, end_point) / 2.0;
        area += signed_area_twice(reference, previous_end, end_point) / 2.0;
    }

    area - signed_area_twice(reference, edge_start, edge_end) / 2.0
}

pub(crate) fn potrace_area_opticurve_penalty(
    reconstructed: &ReconstructedPotraceRun,
    run: &[CubicSegment],
    start: usize,
    end: usize,
    candidate: CubicSegment,
    opt_tolerance: f64,
) -> Option<f64> {
    let mut penalty = 0.0;

    for index in start..end - 1 {
        let from = reconstructed.vertices[index];
        let to = reconstructed.vertices[index + 1];
        let parameter = bezier_tangent_parameter(candidate, from, to)?;
        let point = cubic_point(candidate, parameter);
        let length = distance(from, to);
        if length <= f64::EPSILON {
            return None;
        }

        let signed_distance = signed_area_twice(from, to, point) / length;
        if signed_distance.abs() > opt_tolerance {
            return None;
        }
        if dot(subtract(to, from), subtract(point, from)) < 0.0
            || dot(subtract(from, to), subtract(point, to)) < 0.0
        {
            return None;
        }

        penalty += signed_distance * signed_distance;
    }

    let edge_start = run[start].start;
    for index in start..end {
        let previous_end = if index == start {
            edge_start
        } else {
            run[index - 1].end
        };
        let end_point = run[index].end;
        let parameter = bezier_tangent_parameter(candidate, previous_end, end_point)?;
        let point = cubic_point(candidate, parameter);
        let length = distance(previous_end, end_point);
        if length <= f64::EPSILON {
            return None;
        }

        let mut signed_distance = signed_area_twice(previous_end, end_point, point) / length;
        let mut corner_distance =
            signed_area_twice(previous_end, end_point, reconstructed.vertices[index]) / length;
        corner_distance *= 0.75 * reconstructed.alphas[index];
        if corner_distance < 0.0 {
            signed_distance = -signed_distance;
            corner_distance = -corner_distance;
        }

        if signed_distance < corner_distance - opt_tolerance {
            return None;
        }
        if signed_distance < corner_distance {
            let delta = signed_distance - corner_distance;
            penalty += delta * delta;
        }
    }

    Some(penalty)
}

pub(crate) fn potrace_cubic_vertex(cubic: CubicSegment) -> Option<(f64, f64)> {
    let incoming = subtract(cubic.control1, cubic.start);
    let outgoing = subtract(cubic.control2, cubic.end);
    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return None;
    }

    line_intersection(
        FitLine {
            point: cubic.start,
            direction: incoming,
        },
        FitLine {
            point: cubic.end,
            direction: outgoing,
        },
    )
}

pub(crate) fn potrace_cubic_alpha(cubic: CubicSegment, vertex: (f64, f64)) -> Option<f64> {
    let entry_alpha = projected_fraction(cubic.start, vertex, cubic.control1)?;
    let exit_alpha = projected_fraction(cubic.end, vertex, cubic.control2)?;
    let alpha = (entry_alpha + exit_alpha) / 2.0;

    (alpha.is_finite() && alpha > 0.0 && alpha <= 2.0).then_some(alpha)
}

pub(crate) fn projected_fraction(
    start: (f64, f64),
    end: (f64, f64),
    point: (f64, f64),
) -> Option<f64> {
    let vector = subtract(end, start);
    let length_squared = vector_length_squared(vector);
    if length_squared <= f64::EPSILON {
        return None;
    }

    Some(dot(subtract(point, start), vector) / length_squared)
}

pub(crate) fn bezier_tangent_parameter(
    cubic: CubicSegment,
    line_start: (f64, f64),
    line_end: (f64, f64),
) -> Option<f64> {
    let a = cross_lines(cubic.start, cubic.control1, line_start, line_end);
    let b = cross_lines(cubic.control1, cubic.control2, line_start, line_end);
    let c = cross_lines(cubic.control2, cubic.end, line_start, line_end);
    let quadratic_a = a - 2.0 * b + c;
    let quadratic_b = -2.0 * a + 2.0 * b;
    let quadratic_c = a;
    let discriminant = quadratic_b * quadratic_b - 4.0 * quadratic_a * quadratic_c;

    if quadratic_a.abs() <= f64::EPSILON {
        if quadratic_b.abs() <= f64::EPSILON {
            return None;
        }

        let linear = -quadratic_c / quadratic_b;
        return (0.0..=1.0).contains(&linear).then_some(linear);
    }

    if discriminant < 0.0 {
        return None;
    }

    let root = discriminant.sqrt();
    let first = (-quadratic_b + root) / (2.0 * quadratic_a);
    let second = (-quadratic_b - root) / (2.0 * quadratic_a);

    if (0.0..=1.0).contains(&first) {
        Some(first)
    } else if (0.0..=1.0).contains(&second) {
        Some(second)
    } else {
        None
    }
}

pub(crate) fn cross_lines(
    first_start: (f64, f64),
    first_end: (f64, f64),
    second_start: (f64, f64),
    second_end: (f64, f64),
) -> f64 {
    cross(
        subtract(first_end, first_start),
        subtract(second_end, second_start),
    )
}

pub(crate) fn signed_area_twice(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    cross(subtract(b, a), subtract(c, a))
}
