mod cubic;
mod path_data;
mod pixel_candidate;
mod potrace_optimize;
mod potrace_polygon;
mod templates;

pub(crate) use cubic::*;
pub(crate) use path_data::*;
pub(crate) use pixel_candidate::*;
pub(crate) use potrace_optimize::*;
pub(crate) use potrace_polygon::*;
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
    let protected_template = pixel_potrace_points_match_protected_template(&path.points);

    if simplified.len() >= 3 && simplified.len() < path.points.len() {
        if let Some(capsule) = fit_closed_capsule_potrace_segments(&simplified) {
            if let Some(first) = capsule.first() {
                let candidate = (first.start(), capsule);
                if pixel_potrace_primitive_candidate_is_close_enough(
                    path,
                    canvas_size,
                    &candidate,
                    &best,
                ) {
                    best = candidate;
                }
            }
        }

        if let Some(candidate) = pixel_potrace_segments_for_points(
            path,
            &simplified,
            opt_tolerance,
            canvas_size,
            has_holes,
        ) {
            if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best)
                && (protected_template
                    || !pixel_potrace_compact_candidate_is_better(
                        path,
                        canvas_size,
                        &best,
                        &candidate,
                        false,
                    ))
            {
                best = candidate;
            }
        }
    }

    if !protected_template {
        if let Some(strict_candidate) = compact_strict_pixel_potrace_segments_for_points(
            path,
            &path.points,
            opt_tolerance,
            canvas_size,
        ) {
            if pixel_potrace_compact_candidate_is_better(
                path,
                canvas_size,
                &strict_candidate,
                &best,
                true,
            ) {
                best = strict_candidate;
            }
        }
    }

    if opt_tolerance < PIXEL_POTRACE_RELAXED_POINT_SET_TOLERANCE
        && best.1.len() >= MIN_RELAXED_POINT_SET_SEGMENTS
    {
        if let Some(candidate) = pixel_potrace_segments_for_points(
            path,
            &path.points,
            PIXEL_POTRACE_RELAXED_POINT_SET_TOLERANCE,
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
    pixel_potrace_segments_for_polygon_indices(
        reference_path,
        points,
        &polygon,
        opt_tolerance,
        canvas_size,
        has_holes,
    )
}

fn compact_strict_pixel_potrace_segments_for_points(
    path: &TracePath,
    points: &[(f64, f64)],
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let polygon = optimal_potrace_polygon_indices(points);
    let (start, segments) = smooth_pixel_potrace_segments_for_polygon_indices(points, &polygon)?;
    Some(select_compact_strict_potrace_candidate(
        path,
        canvas_size,
        start,
        &segments,
        opt_tolerance,
    ))
}

const PIXEL_POTRACE_COMPACT_TOLERANCE: f64 = 0.0;
const PIXEL_POTRACE_RELAXED_POINT_SET_TOLERANCE: f64 = 1.0;
const MIN_RELAXED_POINT_SET_SEGMENTS: usize = 24;

fn select_compact_strict_potrace_candidate(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    start: (f64, f64),
    segments: &[SvgPathSegment],
    opt_tolerance: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let optimized = optimize_potrace_segments(
        start,
        segments,
        opt_tolerance,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    );
    if opt_tolerance <= PIXEL_POTRACE_COMPACT_TOLERANCE {
        return optimized;
    }

    let mut selected = optimized;
    let conservative = optimize_potrace_segments(
        start,
        segments,
        PIXEL_POTRACE_COMPACT_TOLERANCE,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    );
    if pixel_potrace_candidate_is_better(path, canvas_size, &conservative, &selected) {
        selected = conservative;
    }

    selected
}

fn smooth_pixel_potrace_segments_for_polygon_indices(
    points: &[(f64, f64)],
    polygon: &[usize],
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let vertices = adjust_potrace_vertices(points, polygon, 0.5);
    smooth_potrace_vertices(&vertices)
}

pub(crate) fn relaxed_pixel_potrace_segments_for_points(
    points: &[(f64, f64)],
    opt_tolerance: f64,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let polygon = relaxed_optimal_potrace_polygon_indices(points);
    let vertices = adjust_potrace_vertices(points, &polygon, 0.5);
    let (start, segments) = smooth_potrace_vertices(&vertices)?;
    Some(optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    ))
}

pub(crate) fn pixel_potrace_segments_for_polygon_indices(
    reference_path: &TracePath,
    points: &[(f64, f64)],
    polygon: &[usize],
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let (start, segments) = smooth_pixel_potrace_segments_for_polygon_indices(points, polygon)?;

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

pub(crate) fn pixel_potrace_points_match_protected_template(points: &[(f64, f64)]) -> bool {
    fit_closed_t_potrace_segments(points).is_some()
        || fit_closed_h_potrace_segments(points).is_some()
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
    let compact_strict_candidate =
        select_compact_strict_potrace_candidate(path, canvas_size, start, &segments, opt_tolerance);

    if path.points.len() >= 12 {
        let mut preserve_primitive = false;
        let mut allow_fitted_override = false;
        let protected_template = pixel_potrace_points_match_protected_template(&path.points);

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
            if let Some(capsule) = fit_closed_capsule_potrace_segments(&path.points) {
                if let Some(first) = capsule.first() {
                    let candidate = (first.start(), capsule);
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
                        allow_fitted_override = true;
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
            if let Some(plus) = fit_closed_plus_potrace_segments(&path.points) {
                if let Some(first) = plus.first() {
                    let candidate = (first.start(), plus);
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
            if let Some(t_shape) = fit_closed_t_potrace_segments(&path.points) {
                if let Some(first) = t_shape.first() {
                    let candidate = (first.start(), t_shape);
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
            if let Some(h_shape) = fit_closed_h_potrace_segments(&path.points) {
                if let Some(first) = h_shape.first() {
                    let candidate = (first.start(), h_shape);
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
            if let Some(hooked_l) = fit_closed_hooked_l_potrace_segments(&path.points) {
                if let Some(first) = hooked_l.first() {
                    let candidate = (first.start(), hooked_l);
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
            if let Some(l_shape) = fit_closed_l_potrace_segments(&path.points) {
                if let Some(first) = l_shape.first() {
                    let candidate = (first.start(), l_shape);
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
            if let Some(stepped_e_candidates) = closed_stepped_e_potrace_candidates(&path.points) {
                const MAX_STEPPED_E_TEMPLATE_BOUNDARY_ERROR: f64 = 0.75;
                let mut selected_stepped_e = None;
                for stepped_e in stepped_e_candidates {
                    let Some(first) = stepped_e.first() else {
                        continue;
                    };
                    let candidate = (first.start(), stepped_e);
                    let candidate_boundary_error =
                        pixel_potrace_candidate_boundary_rms_error(path, &candidate);
                    if candidate_boundary_error <= MAX_STEPPED_E_TEMPLATE_BOUNDARY_ERROR {
                        let Some((width, height)) = canvas_size else {
                            continue;
                        };
                        let candidate_score = (
                            pixel_potrace_candidate_mask_error(path, &candidate, width, height),
                            candidate_boundary_error,
                            compact_svg_path_data_from_segments(candidate.0, &candidate.1).len(),
                        );
                        let should_replace = selected_stepped_e
                            .as_ref()
                            .is_none_or(|(score, _)| candidate_score < *score);
                        if should_replace {
                            selected_stepped_e = Some((candidate_score, candidate));
                        }
                    }
                }
                if let Some((_, candidate)) = selected_stepped_e {
                    best = candidate;
                    preserve_primitive = true;
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
            if let Some(annular_sector) =
                fit_closed_annular_sector_potrace_segments(&path.points, canvas_size)
            {
                if let Some(first) = annular_sector.first() {
                    let candidate = (first.start(), annular_sector);
                    if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                        best = candidate;
                    }
                }
            }
        }

        if !preserve_primitive {
            if let Some(candidate) =
                relaxed_pixel_potrace_segments_for_points(&path.points, opt_tolerance)
            {
                if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                    best = candidate;
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
                        || pixel_potrace_primitive_candidate_is_close_enough(
                            path,
                            canvas_size,
                            &candidate,
                            &best,
                        )
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

        if !preserve_primitive || allow_fitted_override {
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

        if !protected_template
            && (!preserve_primitive || allow_fitted_override)
            && pixel_potrace_compact_candidate_is_better(
                path,
                canvas_size,
                &compact_strict_candidate,
                &best,
                true,
            )
        {
            best = compact_strict_candidate;
        }
    }

    best
}
