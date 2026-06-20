use super::*;

#[test]
fn pixel_diagonal_capsule_uses_narrow_potrace_template() {
    let bitmap = narrow_diagonal_capsule_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
    let segments = fit_closed_diagonal_capsule_potrace_segments(&path.points)
        .expect("narrow diagonal capsule should fit a Potrace-derived template");
    let data = path_to_svg_data(
        path,
        SvgRenderOptions {
            curve_mode: CurveMode::Potrace,
            opt_tolerance: 0.2,
            pixel_potrace: true,
        },
        Some((bitmap.width(), bitmap.height())),
        false,
    )
    .expect("narrow diagonal capsule path should render");

    assert_eq!(segments.len(), 6, "{segments:?}");
    assert!(compact_path_command_count(&data) <= 8, "{data}");
}

#[test]
fn pixel_vertical_capsule_prefers_regular_template_when_boundary_is_closer() {
    let bitmap = rounded_rect_union_bitmap(&[
        (160.0, 65.0, 206.0, 205.0, 23.0),
        (46.0, 54.0, 139.0, 125.0, 11.0),
    ]);
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced
        .paths
        .iter()
        .find(|path| {
            FloatBounds::from_points(&path.points).is_some_and(|bounds| bounds.min_x > 150.0)
        })
        .expect("fixture should include the vertical capsule");
    let bounds = FloatBounds::from_points(&path.points).unwrap();
    let radius = (bounds.max_x - bounds.min_x) / 2.0;
    let expected = vertical_capsule_segments(bounds, radius);
    let segments = fit_closed_capsule_potrace_segments(&path.points)
        .expect("vertical capsule should fit a capsule primitive");
    let candidate = (segments[0].start(), segments);
    let expected_candidate = (expected[0].start(), expected);

    assert_eq!(
        compact_svg_path_data_from_segments(candidate.0, &candidate.1),
        compact_svg_path_data_from_segments(expected_candidate.0, &expected_candidate.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &candidate, bitmap.width(), bitmap.height()) <= 12,
        "{}",
        compact_svg_path_data_from_segments(candidate.0, &candidate.1)
    );
}

#[test]
fn pixel_rounded_rect_union_allows_closer_fitted_override() {
    let bitmap = rounded_rect_union_bitmap(&[
        (62.0, 65.0, 104.0, 125.0, 15.0),
        (88.0, 97.0, 145.0, 172.0, 27.0),
        (36.0, 66.0, 146.0, 205.0, 28.0),
    ]);
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
    let final_candidate =
        choose_pixel_potrace_point_set(path, 0.2, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a candidate");
    let fitted = fit_closed_smooth_potrace_segments(&path.points, false);
    let first = fitted.first().expect("fixture should fit smooth segments");
    let fitted_candidate =
        optimize_potrace_segments(first.start(), &fitted, 0.2, PIXEL_POTRACE_LINEAR_DEVIATION);

    assert_eq!(
        compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1),
        compact_svg_path_data_from_segments(fitted_candidate.0, &fitted_candidate.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &final_candidate, bitmap.width(), bitmap.height())
            <= 65
    );
}

#[test]
fn pixel_compact_fallback_keeps_asymmetric_strict_polygon() {
    let bitmap = rounded_rect_union_bitmap(&[
        (54.0, 143.0, 122.0, 175.0, 16.0),
        (108.0, 76.0, 187.0, 186.0, 12.0),
        (42.0, 65.0, 162.0, 206.0, 25.0),
    ]);
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
    let final_candidate =
        choose_pixel_potrace_point_set(path, 0.2, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a candidate");
    let strict_candidate = strict_pixel_candidate(path);
    let fitted = fit_closed_smooth_potrace_segments(&path.points, false);
    let fitted_first = fitted.first().expect("fixture should fit smooth segments");
    let fitted_candidate = optimize_potrace_segments(
        fitted_first.start(),
        &fitted,
        0.2,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    );

    assert_eq!(
        compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1),
        compact_svg_path_data_from_segments(strict_candidate.0, &strict_candidate.1)
    );
    assert!(
        compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1).len()
            < compact_svg_path_data_from_segments(fitted_candidate.0, &fitted_candidate.1).len()
    );
    assert!(
        pixel_potrace_horizontal_mirror_mismatch_ratio(path, bitmap.width(), bitmap.height())
            >= 0.3
    );
}

#[test]
fn pixel_compact_fallback_skips_axis_symmetric_t_union() {
    let bitmap = rounded_rect_union_bitmap(&[
        (110.0, 48.0, 154.0, 210.0, 17.0),
        (44.0, 58.0, 206.0, 102.0, 17.0),
    ]);
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
    let final_candidate =
        choose_pixel_potrace_point_set(path, 0.2, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a candidate");
    let strict_candidate = strict_pixel_candidate(path);

    assert_ne!(
        compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1),
        compact_svg_path_data_from_segments(strict_candidate.0, &strict_candidate.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &final_candidate, bitmap.width(), bitmap.height())
            <= 60
    );
    assert!(
        pixel_potrace_horizontal_mirror_mismatch_ratio(path, bitmap.width(), bitmap.height()) < 0.3
    );
}

fn strict_pixel_candidate(path: &TracePath) -> ((f64, f64), Vec<SvgPathSegment>) {
    let polygon = optimal_potrace_polygon_indices(&path.points);
    let vertices = adjust_potrace_vertices(&path.points, &polygon, 0.5);
    let (start, segments) =
        smooth_potrace_vertices(&vertices).expect("fixture should produce Potrace vertices");
    optimize_potrace_segments(start, &segments, 0.2, PIXEL_POTRACE_LINEAR_DEVIATION)
}

#[test]
fn pixel_compact_fallback_can_replace_exact_off_center_rounded_rect() {
    let bitmap = rounded_rect_union_bitmap(&[
        (160.0, 65.0, 206.0, 205.0, 23.0),
        (46.0, 54.0, 139.0, 125.0, 11.0),
    ]);
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced
        .paths
        .iter()
        .find(|path| {
            FloatBounds::from_points(&path.points).is_some_and(|bounds| bounds.min_x < 100.0)
        })
        .expect("fixture should include the left rounded rect");
    let final_candidate =
        choose_pixel_potrace_point_set(path, 0.2, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a candidate");
    let strict_candidate = strict_pixel_candidate(path);
    let rounded_rect = fit_closed_rounded_rect_potrace_segments(&path.points)
        .expect("fixture should fit a rounded rect primitive");
    let rounded_rect_candidate = (rounded_rect[0].start(), rounded_rect);

    assert_eq!(
        compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1),
        compact_svg_path_data_from_segments(strict_candidate.0, &strict_candidate.1)
    );
    assert!(pixel_potrace_compact_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &strict_candidate,
        &rounded_rect_candidate,
        true,
    ));
    assert!(!pixel_potrace_compact_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &strict_candidate,
        &rounded_rect_candidate,
        false,
    ));
    assert_ne!(
        compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1),
        compact_svg_path_data_from_segments(rounded_rect_candidate.0, &rounded_rect_candidate.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(
            path,
            &rounded_rect_candidate,
            bitmap.width(),
            bitmap.height()
        ) <= 1
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &final_candidate, bitmap.width(), bitmap.height())
            <= 190
    );
    assert!(
        pixel_potrace_horizontal_mirror_mismatch_ratio(path, bitmap.width(), bitmap.height())
            >= 0.3
    );
}

fn compact_path_command_count(data: &str) -> usize {
    data.chars()
        .filter(|character| {
            matches!(
                character,
                'M' | 'L'
                    | 'H'
                    | 'V'
                    | 'C'
                    | 'S'
                    | 'Q'
                    | 'T'
                    | 'A'
                    | 'Z'
                    | 'm'
                    | 'l'
                    | 'h'
                    | 'v'
                    | 'c'
                    | 's'
                    | 'q'
                    | 't'
                    | 'a'
                    | 'z'
            )
        })
        .count()
}
