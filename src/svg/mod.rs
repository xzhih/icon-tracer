mod cubic;
mod geometry;
mod path_data;
mod path_tokens;
mod pixel_candidate;
mod pixel_candidate_annular;
mod pixel_candidate_capsule;
mod pixel_candidate_metrics;
mod pixel_candidate_polygon;
mod pixel_trace;
mod potrace_bestpolygon;
mod potrace_cleanup;
mod potrace_optimize;
mod potrace_polygon;
mod potrace_vertex;
mod precision;
mod relaxed_polygon;
mod render;
mod templates;

pub(crate) use cubic::*;
pub(crate) use geometry::*;
pub(crate) use path_data::*;
pub(crate) use path_tokens::*;
pub(crate) use pixel_candidate::*;
pub(crate) use pixel_candidate_annular::*;
pub(crate) use pixel_candidate_capsule::*;
pub(crate) use pixel_candidate_metrics::*;
pub(crate) use pixel_candidate_polygon::*;
pub(crate) use pixel_trace::*;
pub(crate) use potrace_bestpolygon::*;
pub(crate) use potrace_cleanup::*;
pub(crate) use potrace_optimize::*;
pub(crate) use potrace_polygon::*;
pub(crate) use potrace_vertex::*;
pub(crate) use precision::*;
pub(crate) use relaxed_polygon::*;
pub(crate) use render::*;
pub(crate) use templates::*;

use crate::TracePath;

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

pub(crate) fn base_pixel_potrace_segments_for_points(
    points: &[(f64, f64)],
    opt_tolerance: f64,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let polygon = optimal_potrace_polygon_indices(points);
    let (start, segments) = smooth_pixel_potrace_segments_for_polygon_indices(points, &polygon)?;
    Some(optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    ))
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
pub(crate) const PIXEL_POTRACE_FINE_OPT_TOLERANCE: f64 = 0.1;
pub(crate) const PIXEL_POTRACE_LOOSE_OPT_TOLERANCE: f64 = 0.3;
pub(crate) const PIXEL_POTRACE_HIGH_OPT_TOLERANCE: f64 = 0.4;
pub(crate) const PIXEL_POTRACE_SIBLING_RELAXED_OPT_TOLERANCE: f64 = 0.75;
const PIXEL_POTRACE_RELAXED_POINT_SET_TOLERANCE: f64 = 1.0;
const MIN_RELAXED_POINT_SET_SEGMENTS: usize = 24;
const MIN_RELAXED_SMOOTHING_SACRIFICE_SEGMENTS: usize = 36;

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

pub(crate) fn area_alpha_pixel_potrace_segments_for_points(
    points: &[(f64, f64)],
    opt_tolerance: f64,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let polygon = optimal_potrace_polygon_indices(points);
    let vertices = adjust_potrace_vertices(points, &polygon, 0.5);
    let (start, segments) = smooth_area_alpha_potrace_vertices(&vertices)?;
    Some(optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    ))
}

pub(crate) fn bestpolygon_pixel_potrace_segments_for_points(
    points: &[(f64, f64)],
    opt_tolerance: f64,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let polygon = potrace_best_polygon_indices(points)?;
    let (start, segments) = smooth_pixel_potrace_segments_for_polygon_indices(points, &polygon)?;
    Some(optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    ))
}

pub(crate) fn bestpolygon_area_alpha_pixel_potrace_segments_for_points(
    points: &[(f64, f64)],
    opt_tolerance: f64,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    bestpolygon_area_alpha_pixel_potrace_segments_for_points_with_vertex_adjustment(
        points,
        opt_tolerance,
        0.5,
    )
}

pub(crate) fn bestpolygon_area_alpha_pixel_potrace_segments_for_points_with_vertex_adjustment(
    points: &[(f64, f64)],
    opt_tolerance: f64,
    max_vertex_adjustment: f64,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let polygon = potrace_best_polygon_indices(points)?;
    let vertices = adjust_potrace_vertices(points, &polygon, max_vertex_adjustment);
    let (start, segments) = smooth_area_alpha_potrace_vertices(&vertices)?;
    Some(optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    ))
}

pub(crate) fn quadratic_vertex_pixel_potrace_segments_for_points(
    points: &[(f64, f64)],
    opt_tolerance: f64,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let polygon = optimal_potrace_polygon_indices(points);
    let vertices = adjust_potrace_vertices_quadratic(points, &polygon, 0.5);
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
        || diagonal_capsule_prefers_medium_low_template(points)
}

pub(crate) fn pixel_potrace_points_match_high_tolerance_protected_template(
    points: &[(f64, f64)],
) -> bool {
    pixel_potrace_points_match_protected_template(points)
        || fit_closed_chevron_potrace_segments(points).is_some()
        || fit_closed_plus_potrace_segments(points).is_some()
        || fit_closed_hooked_l_potrace_segments(points).is_some()
        || fit_closed_l_potrace_segments(points).is_some()
        || fit_closed_staple_potrace_segments(points).is_some()
        || closed_stepped_e_potrace_candidates(points).is_some()
        || fit_closed_stepped_f_potrace_segments(points).is_some()
        || fit_closed_open_ring_potrace_segments(points).is_some()
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
                    if diagonal_capsule_allows_compact_replacement(&path.points)
                        && pixel_potrace_diagonal_capsule_compact_candidate_is_better(
                            path,
                            canvas_size,
                            &compact_strict_candidate,
                            &candidate,
                        )
                    {
                        best = compact_strict_candidate.clone();
                        preserve_primitive = true;
                    } else if pixel_potrace_diagonal_capsule_template_candidate_is_better(
                        path,
                        canvas_size,
                        &candidate,
                        &best,
                    ) || pixel_potrace_primitive_candidate_is_close_enough(
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
            if let Some(rounded_rect) =
                fit_closed_vertical_rounded_rect_potrace_segments(&path.points)
            {
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
            if let Some(ring_sector) =
                fit_closed_moderate_gap_annular_sector_potrace_segments(&path.points, canvas_size)
            {
                if let Some(first) = ring_sector.first() {
                    best = (first.start(), ring_sector);
                    preserve_primitive = true;
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
            if let Some(candidate) = relaxed_quadrilateral_line_candidate(path) {
                if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                    best = candidate;
                }
            }
        }

        if !preserve_primitive {
            if let Some(candidate) =
                bestpolygon_pixel_potrace_segments_for_points(&path.points, opt_tolerance)
            {
                if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                    best = candidate;
                }
            }
        }

        if !preserve_primitive {
            if let Some(candidate) = bestpolygon_area_alpha_pixel_potrace_segments_for_points(
                &path.points,
                opt_tolerance,
            ) {
                let final_better = pixel_potrace_area_alpha_final_candidate_is_better(
                    path,
                    canvas_size,
                    &candidate,
                    &best,
                    !protected_template,
                );
                let smoothing_better = pixel_potrace_area_alpha_smoothing_candidate_is_better(
                    path,
                    canvas_size,
                    &candidate,
                    &best,
                );
                if final_better || smoothing_better {
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
                        || (pixel_potrace_candidate_is_no_more_complex(&candidate, &best)
                            && pixel_potrace_primitive_candidate_is_close_enough(
                                path,
                                canvas_size,
                                &candidate,
                                &best,
                            ))
                        || pixel_potrace_fitted_candidate_is_materially_better(
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
                if pixel_potrace_fitted_candidate_is_materially_better(
                    path,
                    canvas_size,
                    &candidate,
                    &best,
                ) {
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
