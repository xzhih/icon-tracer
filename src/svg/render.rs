use super::*;
use crate::{CurveMode, SvgRenderOptions, TracePath};

#[cfg(all(test, feature = "slow-tests"))]
pub(crate) fn path_to_svg_data(
    path: &TracePath,
    options: SvgRenderOptions,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
) -> Option<String> {
    path_to_svg_data_with_context(path, options, canvas_size, has_holes, false)
}

pub(crate) fn path_to_svg_data_with_context(
    path: &TracePath,
    options: SvgRenderOptions,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
    has_sibling_paths: bool,
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
            has_sibling_paths,
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
    has_sibling_paths: bool,
) -> Option<String> {
    if path.points.len() < 3 {
        return path_to_polygon_svg_data(path);
    }

    if pixel_potrace {
        let (start, segments) = choose_pixel_potrace_point_set_with_context(
            path,
            opt_tolerance,
            canvas_size,
            has_holes,
            has_sibling_paths,
        )?;
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
