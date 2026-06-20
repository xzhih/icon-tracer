use super::*;

#[cfg(test)]
pub(crate) fn choose_pixel_potrace_point_set(
    path: &TracePath,
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    choose_pixel_potrace_point_set_with_context(path, opt_tolerance, canvas_size, has_holes, false)
}

pub(crate) fn choose_pixel_potrace_point_set_with_context(
    path: &TracePath,
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
    has_holes: bool,
    has_sibling_paths: bool,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let mut best = pixel_potrace_segments_for_points(
        path,
        &path.points,
        opt_tolerance,
        canvas_size,
        has_holes,
    )?;
    let base = base_pixel_potrace_segments_for_points(&path.points, opt_tolerance)?;
    let mut best_is_base = pixel_potrace_candidates_have_same_path_data(&best, &base);
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
                    best_is_base = false;
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
                best_is_base = false;
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
            if (has_sibling_paths
                && pixel_potrace_candidate_is_better(path, canvas_size, &strict_candidate, &best))
                || pixel_potrace_compact_candidate_is_better(
                    path,
                    canvas_size,
                    &strict_candidate,
                    &best,
                    true,
                )
            {
                best = strict_candidate;
                best_is_base = false;
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
            let should_replace =
                pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best)
                    || (best.1.len() >= MIN_RELAXED_SMOOTHING_SACRIFICE_SEGMENTS
                        && pixel_potrace_relaxed_point_set_candidate_is_better(
                            path,
                            canvas_size,
                            &candidate,
                            &best,
                        ));
            if should_replace {
                best = candidate;
                best_is_base = false;
            }
        }
    }

    if has_sibling_paths {
        return Some(best);
    }

    if best_is_base {
        if let Some(candidate) =
            area_alpha_pixel_potrace_segments_for_points(&path.points, opt_tolerance)
        {
            if pixel_potrace_area_alpha_candidate_is_better(path, canvas_size, &candidate, &best) {
                best = candidate;
            }
        }
    } else {
        if let Some(candidate) =
            area_alpha_pixel_potrace_segments_for_points(&path.points, opt_tolerance)
        {
            if pixel_potrace_area_alpha_final_candidate_is_better(
                path,
                canvas_size,
                &candidate,
                &best,
            ) {
                best = candidate;
            }
        }
    }

    Some(best)
}

fn pixel_potrace_candidates_have_same_path_data(
    left: &((f64, f64), Vec<SvgPathSegment>),
    right: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    compact_svg_path_data_from_segments_without_arcs(left.0, &left.1)
        == compact_svg_path_data_from_segments_without_arcs(right.0, &right.1)
}
