use super::*;
use crate::TracePath;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SvgPathPrecision {
    Compact,
    PreserveFractional,
    ForceScaled,
}

impl SvgPathPrecision {
    pub(crate) fn max(self, other: Self) -> Self {
        match (self, other) {
            (Self::ForceScaled, _) | (_, Self::ForceScaled) => Self::ForceScaled,
            (Self::PreserveFractional, _) | (_, Self::PreserveFractional) => {
                Self::PreserveFractional
            }
            (Self::Compact, Self::Compact) => Self::Compact,
        }
    }
}

#[cfg(test)]
pub(crate) fn svg_path_element(
    path_data: &str,
    allow_scaled_potrace_path: bool,
    canvas_height: usize,
) -> String {
    svg_path_element_with_precision(
        path_data,
        allow_scaled_potrace_path,
        canvas_height,
        SvgPathPrecision::Compact,
    )
}

pub(crate) fn svg_path_element_with_precision(
    path_data: &str,
    allow_scaled_potrace_path: bool,
    canvas_height: usize,
    precision: SvgPathPrecision,
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

        if precision == SvgPathPrecision::ForceScaled || scaled.len() < best.len() {
            best = scaled;
            best_path_data_len = scaled_path_data.len();
        }
    }

    if precision == SvgPathPrecision::Compact && !path_data_has_arc_commands(path_data) {
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

            let potrace_path_data = potrace_y_flipped_integer_svg_path_data(
                &one_decimal_path_data,
                canvas_height,
                10.0,
            );
            if let Some(potrace_path_data) = potrace_path_data {
                let potrace_scaled = format!(
                    r#"<path fill="black" fill-rule="evenodd" transform="translate(0 {canvas_height}) scale(.1 -.1)" d="{potrace_path_data}"/>"#
                );

                if pixel_potrace_y_flipped_integer_path_is_preferred(
                    &one_decimal_path_data,
                    potrace_path_data.len(),
                    best_path_data_len,
                ) {
                    best = potrace_scaled;
                }
            }
        }
    }

    best
}

pub(crate) fn pixel_potrace_path_precision_preference(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
    has_sibling_paths: bool,
    opt_tolerance: f64,
) -> SvgPathPrecision {
    const MIN_TENTH_QUANTIZATION_REGRESSION_PIXELS: usize = 8;
    const MIN_LINE_POLYGON_TENTH_QUANTIZATION_REGRESSION_PIXELS: usize = 2;

    let Some((width, height)) = canvas_size else {
        return SvgPathPrecision::Compact;
    };

    if pixel_potrace_points_prefer_fractional_precision_annular_sector(&path.points, width, height)
    {
        return SvgPathPrecision::PreserveFractional;
    }

    let Some(candidate) = choose_pixel_potrace_point_set_with_context(
        path,
        opt_tolerance,
        canvas_size,
        has_holes,
        has_sibling_paths,
    ) else {
        return SvgPathPrecision::Compact;
    };
    let tenth = quantize_potrace_candidate_to_tenth(&candidate);
    let candidate_error = pixel_potrace_candidate_mask_error(path, &candidate, width, height);
    let tenth_error = pixel_potrace_candidate_mask_error(path, &tenth, width, height);

    if pixel_potrace_candidate_is_simple_line_polygon(&candidate)
        && candidate_error.saturating_add(MIN_LINE_POLYGON_TENTH_QUANTIZATION_REGRESSION_PIXELS)
            <= tenth_error
    {
        return SvgPathPrecision::PreserveFractional;
    }
    if pixel_potrace_candidate_is_rounded_quadrilateral_curve(&candidate) {
        return SvgPathPrecision::PreserveFractional;
    }

    let prefers_legacy_scaled_precision =
        pixel_potrace_candidate_prefers_scaled_precision(&candidate, 14);
    let prefers_expanded_scaled_precision = if has_sibling_paths {
        false
    } else {
        let mirror_mismatch = pixel_potrace_horizontal_mirror_mismatch_ratio(path, width, height);
        !(0.05..0.3).contains(&mirror_mismatch)
            && pixel_potrace_candidate_prefers_scaled_precision(&candidate, 8)
    };
    if fit_closed_diagonal_capsule_potrace_segments(&path.points).is_none()
        && (prefers_legacy_scaled_precision || prefers_expanded_scaled_precision)
    {
        return SvgPathPrecision::ForceScaled;
    }

    if candidate_error.saturating_add(MIN_TENTH_QUANTIZATION_REGRESSION_PIXELS) <= tenth_error {
        SvgPathPrecision::ForceScaled
    } else {
        SvgPathPrecision::Compact
    }
}

fn pixel_potrace_candidate_is_simple_line_polygon(
    candidate: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const MAX_LINE_SEGMENTS: usize = 8;

    !candidate.1.is_empty()
        && candidate.1.len() <= MAX_LINE_SEGMENTS
        && candidate
            .1
            .iter()
            .all(|segment| matches!(segment, SvgPathSegment::Line { .. }))
}

fn pixel_potrace_candidate_is_rounded_quadrilateral_curve(
    candidate: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    const QUADRILATERAL_CURVE_SEGMENTS: usize = 8;

    candidate.1.len() == QUADRILATERAL_CURVE_SEGMENTS
        && candidate.1.iter().enumerate().all(|(index, segment)| {
            if index % 2 == 0 {
                matches!(segment, SvgPathSegment::Line { .. })
            } else {
                matches!(segment, SvgPathSegment::Cubic(_))
            }
        })
}

fn pixel_potrace_candidate_prefers_scaled_precision(
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    min_complex_cubic_count: usize,
) -> bool {
    const MIN_COMPLEX_LINE_COUNT: usize = 15;

    let stats = scaled_precision_stats(candidate);

    stats.has_tenth_residue
        && (stats.cubic_count >= min_complex_cubic_count
            || stats.line_count >= MIN_COMPLEX_LINE_COUNT)
}

struct ScaledPrecisionStats {
    has_tenth_residue: bool,
    cubic_count: usize,
    line_count: usize,
}

fn scaled_precision_stats(candidate: &((f64, f64), Vec<SvgPathSegment>)) -> ScaledPrecisionStats {
    let mut stats = ScaledPrecisionStats {
        has_tenth_residue: false,
        cubic_count: 0,
        line_count: 0,
    };

    add_point_scaled_precision_stats(&mut stats, candidate.0);
    for segment in &candidate.1 {
        match *segment {
            SvgPathSegment::Line { end, .. } => {
                stats.line_count += 1;
                add_point_scaled_precision_stats(&mut stats, end);
            }
            SvgPathSegment::Cubic(cubic) => {
                stats.cubic_count += 1;
                add_point_scaled_precision_stats(&mut stats, cubic.control1);
                add_point_scaled_precision_stats(&mut stats, cubic.control2);
                add_point_scaled_precision_stats(&mut stats, cubic.end);
            }
        }
    }

    stats
}

fn add_point_scaled_precision_stats(stats: &mut ScaledPrecisionStats, point: (f64, f64)) {
    add_coordinate_scaled_precision_stats(stats, point.0);
    add_coordinate_scaled_precision_stats(stats, point.1);
}

fn add_coordinate_scaled_precision_stats(stats: &mut ScaledPrecisionStats, value: f64) {
    const EPSILON: f64 = 1.0e-3;

    let residue = (value * 100.0 - (value * 10.0).round() * 10.0).abs();
    if residue > EPSILON {
        stats.has_tenth_residue = true;
    }
}

pub(crate) fn path_data_has_arc_commands(path_data: &str) -> bool {
    path_data.bytes().any(|byte| matches!(byte, b'A' | b'a'))
}

fn pixel_potrace_y_flipped_integer_path_is_preferred(
    one_decimal_path_data: &str,
    potrace_path_data_len: usize,
    best_path_data_len: usize,
) -> bool {
    potrace_path_data_len < best_path_data_len
        || path_data_has_bezier_commands(one_decimal_path_data)
}
