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
fn pixel_rounded_rect_union_allows_best_area_rescue_over_fitted_override() {
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
    let best_area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a best-area candidate");

    assert_eq!(
        compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1),
        compact_svg_path_data_from_segments(best_area.0, &best_area.1)
    );
    let final_error =
        pixel_potrace_candidate_mask_error(path, &final_candidate, bitmap.width(), bitmap.height());
    let fitted_error = pixel_potrace_candidate_mask_error(
        path,
        &fitted_candidate,
        bitmap.width(),
        bitmap.height(),
    );
    assert!(final_error < fitted_error);
    assert!(final_error <= 50);
}

#[test]
fn pixel_bestpolygon_area_alpha_rescue_can_replace_conservative_strict_when_closer() {
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
    let conservative_strict_candidate = strict_pixel_candidate_with_tolerance(path, 0.0);
    let area_alpha_candidate =
        bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
            .expect("fixture should produce an area-alpha candidate");
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
        compact_svg_path_data_from_segments(area_alpha_candidate.0, &area_alpha_candidate.1)
    );
    assert_ne!(
        compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1),
        compact_svg_path_data_from_segments(strict_candidate.0, &strict_candidate.1)
    );
    assert_ne!(
        compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1),
        compact_svg_path_data_from_segments(
            conservative_strict_candidate.0,
            &conservative_strict_candidate.1,
        )
    );
    assert!(
        pixel_potrace_candidate_mask_error(
            path,
            &conservative_strict_candidate,
            bitmap.width(),
            bitmap.height()
        ) < pixel_potrace_candidate_mask_error(
            path,
            &strict_candidate,
            bitmap.width(),
            bitmap.height()
        )
    );
    assert!(
        pixel_potrace_candidate_boundary_rms_error(path, &conservative_strict_candidate)
            < pixel_potrace_candidate_boundary_rms_error(path, &strict_candidate)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &final_candidate, bitmap.width(), bitmap.height())
            < pixel_potrace_candidate_mask_error(
                path,
                &conservative_strict_candidate,
                bitmap.width(),
                bitmap.height()
            )
    );
    assert!(
        pixel_potrace_candidate_boundary_rms_error(path, &final_candidate)
            < pixel_potrace_candidate_boundary_rms_error(path, &conservative_strict_candidate)
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
    strict_pixel_candidate_with_tolerance(path, 0.2)
}

fn strict_pixel_candidate_with_tolerance(
    path: &TracePath,
    opt_tolerance: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let polygon = optimal_potrace_polygon_indices(&path.points);
    let vertices = adjust_potrace_vertices(&path.points, &polygon, 0.5);
    let (start, segments) =
        smooth_potrace_vertices(&vertices).expect("fixture should produce Potrace vertices");
    optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    )
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

#[test]
fn pixel_smoothing_fallback_can_replace_complex_candidate_when_closer() {
    let bitmap = rounded_rect_union_bitmap(&[
        (69.0, 66.0, 144.0, 192.0, 13.0),
        (62.0, 171.0, 160.0, 205.0, 13.0),
        (42.0, 74.0, 92.0, 194.0, 10.0),
        (82.0, 57.0, 147.0, 155.0, 19.0),
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
    let base_candidate = pixel_potrace_segments_for_points(
        path,
        &path.points,
        0.2,
        Some((bitmap.width(), bitmap.height())),
        false,
    )
    .expect("fixture should produce a base candidate");
    let relaxed_candidate =
        choose_pixel_potrace_point_set(path, 1.0, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a relaxed candidate");
    let final_candidate =
        choose_pixel_potrace_point_set(path, 0.2, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a final candidate");

    assert!(pixel_potrace_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &relaxed_candidate,
        &base_candidate,
    ));
    let base_error =
        pixel_potrace_candidate_mask_error(path, &base_candidate, bitmap.width(), bitmap.height());
    let relaxed_error = pixel_potrace_candidate_mask_error(
        path,
        &relaxed_candidate,
        bitmap.width(),
        bitmap.height(),
    );
    let final_error =
        pixel_potrace_candidate_mask_error(path, &final_candidate, bitmap.width(), bitmap.height());

    assert!(final_candidate.1.len() < base_candidate.1.len());
    assert!(final_error < base_error);
    assert!(final_error <= relaxed_error + 8);
}

#[test]
fn pixel_diagonal_capsule_can_use_compact_candidate_when_substantially_closer() {
    let bitmap = capsule_bitmap((42.0, 188.0), (204.0, 92.0), 24.0);
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
    let compact_candidate = strict_pixel_candidate_with_tolerance(path, 0.0);
    let primitive = fit_closed_diagonal_capsule_potrace_segments(&path.points)
        .expect("fixture should fit a diagonal capsule primitive");
    let primitive_candidate = (primitive[0].start(), primitive);

    assert!(diagonal_capsule_allows_compact_replacement(&path.points));
    assert!(pixel_potrace_diagonal_capsule_compact_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &compact_candidate,
        &primitive_candidate
    ));
    assert!(
        pixel_potrace_candidate_mask_error(path, &final_candidate, bitmap.width(), bitmap.height())
            < pixel_potrace_candidate_mask_error(
                path,
                &primitive_candidate,
                bitmap.width(),
                bitmap.height()
            )
    );
}

#[test]
fn pixel_diagonal_capsule_blocks_small_low_angle_compact_replacement() {
    let bitmap = capsule_bitmap((38.0, 184.0), (218.0, 72.0), 17.0);
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

    assert!(!diagonal_capsule_allows_compact_replacement(&path.points));
}

#[test]
fn pixel_low_angle_diagonal_capsule_uses_quadratic_vertex_rescue() {
    let bitmap = capsule_bitmap((38.0, 184.0), (218.0, 72.0), 17.0);
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
    let primitive = fit_closed_diagonal_capsule_potrace_segments(&path.points)
        .expect("fixture should fit a diagonal capsule primitive");
    let primitive_candidate = (primitive[0].start(), primitive);
    let quadratic = quadratic_vertex_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce quadratic candidate");
    let primitive_error = pixel_potrace_candidate_mask_error(
        path,
        &primitive_candidate,
        bitmap.width(),
        bitmap.height(),
    );
    let final_error =
        pixel_potrace_candidate_mask_error(path, &final_candidate, bitmap.width(), bitmap.height());
    let final_data = compact_svg_path_data_from_segments(final_candidate.0, &final_candidate.1);

    assert_eq!(
        primitive_candidate.1.len(),
        6,
        "{:?}",
        primitive_candidate.1
    );
    assert!(final_error < primitive_error, "{final_data}");
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(final_candidate.0, &final_candidate.1),
        compact_svg_path_data_from_segments_without_arcs(quadratic.0, &quadratic.1)
    );
    assert!(final_error <= 75, "{final_data}");
    assert!(compact_path_command_count(&final_data) <= 6, "{final_data}");
    assert!(final_data.len() <= 260, "{final_data}");
}

#[test]
fn pixel_shallow_angle_diagonal_capsule_uses_potrace_template() {
    let bitmap = capsule_bitmap((40.0, 78.0), (210.0, 92.0), 21.0);
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
    let segments = fit_closed_diagonal_capsule_potrace_segments(&path.points)
        .expect("fixture should fit a diagonal capsule primitive");
    let candidate = (segments[0].start(), segments.clone());
    let candidate_error =
        pixel_potrace_candidate_mask_error(path, &candidate, bitmap.width(), bitmap.height());
    let final_error =
        pixel_potrace_candidate_mask_error(path, &final_candidate, bitmap.width(), bitmap.height());

    assert_eq!(segments.len(), 8, "{segments:?}");
    assert!(candidate_error <= 215, "{candidate_error}");
    assert!(final_error <= 205, "{final_error}");
}

#[test]
fn pixel_diagonal_capsule_rejects_compact_candidate_when_too_expensive() {
    let bitmap = capsule_bitmap((38.0, 190.0), (164.0, 54.0), 22.0);
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
    let compact_candidate = strict_pixel_candidate_with_tolerance(path, 0.0);
    let primitive = fit_closed_diagonal_capsule_potrace_segments(&path.points)
        .expect("fixture should fit a diagonal capsule primitive");
    let primitive_candidate = (primitive[0].start(), primitive);

    assert!(!pixel_potrace_diagonal_capsule_compact_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &compact_candidate,
        &primitive_candidate
    ));
}

#[test]
fn pixel_low_angle_diagonal_capsule_can_use_tiny_fine_rescue() {
    let bitmap = capsule_bitmap((38.0, 184.0), (218.0, 72.0), 17.0);
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
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
        .expect("fixture should produce selected candidate");
    let fine = pixel_potrace_segments_for_points(
        path,
        &path.points,
        PIXEL_POTRACE_FINE_OPT_TOLERANCE,
        canvas_size,
        false,
    )
    .expect("fixture should produce fine candidate");

    let quadratic = quadratic_vertex_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce quadratic candidate");
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(quadratic.0, &quadratic.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &selected, bitmap.width(), bitmap.height())
            < pixel_potrace_candidate_mask_error(path, &fine, bitmap.width(), bitmap.height())
    );
    assert!(pixel_potrace_candidate_boundary_rms_error(path, &selected) <= 0.47);
}

#[test]
fn pixel_low_angle_diagonal_capsule_rejects_tiny_fine_canaries() {
    let fixtures = [
        capsule_bitmap((42.0, 188.0), (204.0, 92.0), 24.0),
        capsule_bitmap((38.0, 190.0), (164.0, 54.0), 22.0),
        capsule_bitmap((40.0, 78.0), (210.0, 92.0), 21.0),
    ];

    for bitmap in fixtures {
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
        let canvas_size = Some((bitmap.width(), bitmap.height()));
        let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
            .expect("fixture should produce selected candidate");
        let fine = pixel_potrace_segments_for_points(
            path,
            &path.points,
            PIXEL_POTRACE_FINE_OPT_TOLERANCE,
            canvas_size,
            false,
        )
        .expect("fixture should produce fine candidate");

        assert!(!pixel_potrace_diagonal_capsule_fine_candidate_is_better(
            path,
            canvas_size,
            &fine,
            &selected,
        ));
    }
}

#[test]
fn pixel_diagonal_capsule_can_accept_small_best_area_rescue() {
    let bitmap = capsule_bitmap((42.0, 104.0), (218.0, 154.0), 16.0);
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
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let base = pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce base candidate");
    let best_area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce best-area candidate");

    assert!(
        pixel_potrace_diagonal_capsule_best_area_candidate_is_better(
            path,
            canvas_size,
            &best_area,
            &base,
        )
    );

    let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
        .expect("fixture should produce selected candidate");
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(best_area.0, &best_area.1)
    );
}

#[test]
fn pixel_diagonal_capsule_rejects_small_best_area_canaries() {
    let fixtures = [
        capsule_bitmap((38.0, 184.0), (218.0, 72.0), 17.0),
        capsule_bitmap((38.0, 190.0), (164.0, 54.0), 22.0),
    ];

    for bitmap in fixtures {
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
        let canvas_size = Some((bitmap.width(), bitmap.height()));
        let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
            .expect("fixture should produce selected candidate");
        let best_area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
            .expect("fixture should produce best-area candidate");

        assert!(
            !pixel_potrace_diagonal_capsule_best_area_candidate_is_better(
                path,
                canvas_size,
                &best_area,
                &selected,
            )
        );
    }
}

fn capsule_bitmap(start: (f64, f64), end: (f64, f64), half_width: f64) -> Bitmap {
    const CANVAS: usize = 256;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                distance_squared_to_segment((x as f64 + 0.5, y as f64 + 0.5), start, end)
                    .0
                    .sqrt()
                    <= half_width
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
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
