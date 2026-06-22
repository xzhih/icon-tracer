use super::*;
use crate::trace::rasterize_path_evenodd;

#[test]
fn area_alpha_candidate_can_replace_bestpolygon_with_small_segment_growth() {
    let bitmap = rounded_rect_union_bitmap(&[
        (51.0, 135.0, 165.0, 177.0, 20.0),
        (81.0, 91.0, 138.0, 184.0, 28.0),
        (138.0, 137.0, 212.0, 180.0, 15.0),
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
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
        .expect("fixture should produce a selected candidate");
    let area_alpha = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce an area-alpha candidate");
    let fitted = {
        let segments = fit_closed_smooth_potrace_segments(&path.points, false);
        let first = segments.first().expect("fixture should produce a fit");
        optimize_potrace_segments(
            first.start(),
            &segments,
            0.2,
            PIXEL_POTRACE_LINEAR_DEVIATION,
        )
    };

    assert_eq!(
        compact_svg_path_data_from_segments(selected.0, &selected.1),
        compact_svg_path_data_from_segments(area_alpha.0, &area_alpha.1)
    );
    assert!(selected.1.len() < fitted.1.len());
    assert!(
        pixel_potrace_candidate_mask_error(path, &selected, bitmap.width(), bitmap.height())
            <= pixel_potrace_candidate_mask_error(path, &fitted, bitmap.width(), bitmap.height())
                + 1
    );
}

#[test]
fn sibling_paths_can_accept_bestpolygon_area_candidate() {
    let bitmap = rounded_rect_union_bitmap(&[
        (157.0, 125.0, 200.0, 191.0, 18.0),
        (115.0, 150.0, 155.0, 189.0, 12.0),
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
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let selected = choose_pixel_potrace_point_set_with_context(path, 0.2, canvas_size, false, true)
        .expect("fixture should produce a sibling selected candidate");
    let mut pre_best =
        pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
            .expect("fixture should produce a base candidate");
    let base = pre_best.clone();
    let relaxed = pixel_potrace_segments_for_points(
        path,
        &path.points,
        PIXEL_POTRACE_SIBLING_RELAXED_OPT_TOLERANCE,
        canvas_size,
        false,
    )
    .expect("fixture should produce a relaxed candidate");
    if pixel_potrace_sibling_relaxed_candidate_is_better(path, canvas_size, &relaxed, &pre_best) {
        pre_best = relaxed;
    }
    let area = area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a sibling area-alpha candidate");
    if pixel_potrace_sibling_area_candidate_is_better(path, canvas_size, &area, &pre_best) {
        pre_best = area;
    }
    let best_area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a bestpolygon area-alpha candidate");

    assert!(!pixel_potrace_sibling_area_candidate_is_better(
        path,
        canvas_size,
        &best_area,
        &pre_best,
    ));
    assert!(pixel_potrace_sibling_best_area_rescue_candidate_is_better(
        path,
        canvas_size,
        &best_area,
        &pre_best,
    ));
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(best_area.0, &best_area.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &selected, bitmap.width(), bitmap.height())
            < pixel_potrace_candidate_mask_error(path, &base, bitmap.width(), bitmap.height())
    );
}

#[test]
fn pixel_potrace_precision_uses_scaled_for_asymmetric_complex_union() {
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

    let precision = pixel_potrace_path_precision_preference(
        path,
        Some((bitmap.width(), bitmap.height())),
        false,
        false,
        0.2,
    );

    assert!(precision == SvgPathPrecision::ForceScaled);
}

#[test]
fn pixel_potrace_precision_keeps_offset_t_compact() {
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

    let precision = pixel_potrace_path_precision_preference(
        path,
        Some((bitmap.width(), bitmap.height())),
        false,
        false,
        0.2,
    );

    assert!(precision == SvgPathPrecision::Compact);
}

#[test]
fn pixel_potrace_precision_keeps_sibling_union_compact() {
    let bitmap = rounded_rect_union_bitmap(&[
        (157.0, 125.0, 200.0, 191.0, 18.0),
        (115.0, 150.0, 155.0, 189.0, 12.0),
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
    assert!(traced.paths.len() > 1, "fixture should trace sibling paths");

    let precision = traced
        .paths
        .iter()
        .map(|path| {
            pixel_potrace_path_precision_preference(
                path,
                Some((bitmap.width(), bitmap.height())),
                false,
                true,
                0.2,
            )
        })
        .fold(SvgPathPrecision::Compact, SvgPathPrecision::max);

    assert!(precision == SvgPathPrecision::Compact);
}

#[test]
fn pixel_potrace_candidate_selection_rejects_shorter_mask_regression() {
    let path = TracePath {
        is_hole: false,
        points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
    };
    let best = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (10.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 10.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 0.0),
            },
        ],
    );
    let shorter_wrong = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 0.0),
            },
        ],
    );

    assert!(pixel_potrace_candidate_is_better(
        &path,
        None,
        &shorter_wrong,
        &best
    ));
    assert!(!pixel_potrace_candidate_is_better(
        &path,
        Some((12, 12)),
        &shorter_wrong,
        &best
    ));
}

#[test]
fn relaxed_point_set_candidate_selection_accepts_bounded_smoothing_sacrifice() {
    let bitmap = rounded_rect_union_bitmap(&[
        (49.0, 84.0, 168.0, 139.0, 19.0),
        (52.0, 108.0, 96.0, 210.0, 8.0),
        (99.0, 123.0, 210.0, 181.0, 25.0),
        (76.0, 65.0, 162.0, 196.0, 26.0),
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
    let relaxed_candidate = pixel_potrace_segments_for_points(
        path,
        &path.points,
        1.0,
        Some((bitmap.width(), bitmap.height())),
        false,
    )
    .expect("fixture should produce a relaxed candidate");

    assert!(pixel_potrace_relaxed_point_set_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &relaxed_candidate,
        &base_candidate,
    ));
}

#[test]
fn relaxed_point_set_candidate_selection_rejects_rendered_regression() {
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
    let base_candidate = base_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a base candidate");
    let relaxed_candidate = relaxed_pixel_potrace_segments_for_points(&path.points, 1.0)
        .expect("fixture should produce a relaxed candidate");

    assert!(!pixel_potrace_relaxed_point_set_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &relaxed_candidate,
        &base_candidate,
    ));
}

#[test]
fn sibling_paths_keep_strong_best_area_rescues_bounded() {
    let bitmap = rounded_rect_union_bitmap(&[
        (157.0, 125.0, 200.0, 191.0, 18.0),
        (115.0, 150.0, 155.0, 189.0, 12.0),
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

    assert_eq!(traced.paths.len(), 2);

    let mut errors = Vec::new();
    for path in &traced.paths {
        let sibling = choose_pixel_potrace_point_set_with_context(
            path,
            0.2,
            Some((bitmap.width(), bitmap.height())),
            false,
            true,
        )
        .expect("fixture path should produce sibling-aware candidate");

        let sibling_error =
            pixel_potrace_candidate_mask_error(path, &sibling, bitmap.width(), bitmap.height());

        assert!(sibling.1.len() <= 10, "{sibling:?}");
        errors.push(sibling_error);
    }
    errors.sort_unstable();

    assert_eq!(errors, [9, 11]);
}

#[test]
fn sibling_paths_keep_hole_templates_stable() {
    let bitmap = parity_ring_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    assert!(traced.paths.iter().any(|path| path.is_hole));

    for path in &traced.paths {
        let without_sibling = choose_pixel_potrace_point_set(path, 0.2, canvas_size, true)
            .expect("ring path should produce a non-sibling candidate");
        let with_sibling =
            choose_pixel_potrace_point_set_with_context(path, 0.2, canvas_size, true, true)
                .expect("ring path should produce a sibling candidate");

        assert_eq!(
            compact_svg_path_data_from_segments_without_arcs(with_sibling.0, &with_sibling.1),
            compact_svg_path_data_from_segments_without_arcs(without_sibling.0, &without_sibling.1)
        );
    }
}

#[test]
fn sibling_paths_can_accept_relaxed_candidate_when_it_simplifies_components() {
    let fixtures = [
        rounded_rect_union_bitmap(&[
            (48.0, 45.0, 99.0, 127.0, 25.0),
            (50.0, 100.0, 94.0, 178.0, 11.0),
            (147.0, 143.0, 183.0, 186.0, 9.0),
        ]),
        rounded_rect_union_bitmap(&[
            (157.0, 125.0, 200.0, 191.0, 18.0),
            (115.0, 150.0, 155.0, 189.0, 12.0),
        ]),
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
        assert_eq!(traced.paths.len(), 2);

        let mut accepted_relaxed_component = false;
        let mut base_error = 0usize;
        let mut selected_error = 0usize;
        let mut base_segments = 0usize;
        let mut selected_segments = 0usize;
        let mut base_bytes = 0usize;
        let mut selected_bytes = 0usize;
        for path in &traced.paths {
            let selected = choose_pixel_potrace_point_set_with_context(
                path,
                0.2,
                Some((bitmap.width(), bitmap.height())),
                false,
                true,
            )
            .expect("fixture path should produce sibling-aware candidate");
            let base = pixel_potrace_segments_for_points(
                path,
                &path.points,
                0.2,
                Some((bitmap.width(), bitmap.height())),
                false,
            )
            .expect("fixture path should produce base candidate");
            let relaxed = pixel_potrace_segments_for_points(
                path,
                &path.points,
                PIXEL_POTRACE_SIBLING_RELAXED_OPT_TOLERANCE,
                Some((bitmap.width(), bitmap.height())),
                false,
            )
            .expect("fixture path should produce relaxed candidate");

            base_error +=
                pixel_potrace_candidate_mask_error(path, &base, bitmap.width(), bitmap.height());
            selected_error += pixel_potrace_candidate_mask_error(
                path,
                &selected,
                bitmap.width(),
                bitmap.height(),
            );
            base_segments += base.1.len();
            selected_segments += selected.1.len();
            base_bytes += compact_svg_path_data_from_segments_without_arcs(base.0, &base.1).len();
            selected_bytes +=
                compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1).len();

            if pixel_potrace_sibling_relaxed_candidate_is_better(
                path,
                Some((bitmap.width(), bitmap.height())),
                &relaxed,
                &base,
            ) {
                accepted_relaxed_component = true;
            }
        }

        assert!(accepted_relaxed_component);
        assert!(selected_segments.saturating_add(4) <= base_segments);
        assert!(selected_bytes < base_bytes);
        assert!(selected_error <= base_error);
    }
}

#[test]
fn sibling_paths_reject_relaxed_candidate_when_potrace_parity_regresses() {
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
    assert_eq!(traced.paths.len(), 2);

    for path in &traced.paths {
        let base = pixel_potrace_segments_for_points(
            path,
            &path.points,
            0.2,
            Some((bitmap.width(), bitmap.height())),
            false,
        )
        .expect("fixture path should produce base candidate");
        let relaxed = pixel_potrace_segments_for_points(
            path,
            &path.points,
            PIXEL_POTRACE_SIBLING_RELAXED_OPT_TOLERANCE,
            Some((bitmap.width(), bitmap.height())),
            false,
        )
        .expect("fixture path should produce relaxed candidate");

        assert!(!pixel_potrace_sibling_relaxed_candidate_is_better(
            path,
            Some((bitmap.width(), bitmap.height())),
            &relaxed,
            &base,
        ));
    }
}

#[test]
fn sibling_paths_can_accept_bounded_area_rescue() {
    let fixtures = [
        (
            rounded_rect_union_bitmap(&[
                (48.0, 45.0, 99.0, 127.0, 25.0),
                (50.0, 100.0, 94.0, 178.0, 11.0),
                (147.0, 143.0, 183.0, 186.0, 9.0),
            ]),
            2,
        ),
        (
            rounded_rect_union_bitmap(&[
                (160.0, 65.0, 206.0, 205.0, 23.0),
                (46.0, 54.0, 139.0, 125.0, 11.0),
            ]),
            1,
        ),
    ];

    for (bitmap, expected_area_components) in fixtures {
        let traced = trace_bitmap(
            &bitmap,
            TraceOptions {
                turd_size: 2,
                opt_tolerance: 0.0,
                contour_mode: ContourMode::Pixel,
                preserve_collinear: true,
            },
        );
        let canvas_size = Some((bitmap.width(), bitmap.height()));
        assert_eq!(traced.paths.len(), 2);

        let mut accepted_area_components = 0usize;
        for path in &traced.paths {
            let mut pre_area =
                pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
                    .expect("fixture path should produce base candidate");
            let relaxed = pixel_potrace_segments_for_points(
                path,
                &path.points,
                PIXEL_POTRACE_SIBLING_RELAXED_OPT_TOLERANCE,
                canvas_size,
                false,
            )
            .expect("fixture path should produce relaxed candidate");
            if pixel_potrace_sibling_relaxed_candidate_is_better(
                path,
                canvas_size,
                &relaxed,
                &pre_area,
            ) {
                pre_area = relaxed;
            }

            let selected =
                choose_pixel_potrace_point_set_with_context(path, 0.2, canvas_size, false, true)
                    .expect("fixture path should produce sibling-aware candidate");
            let area = area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
                .expect("fixture path should produce area candidate");

            if pixel_potrace_sibling_area_candidate_is_better(path, canvas_size, &area, &pre_area) {
                accepted_area_components += 1;
                assert_eq!(
                    compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
                    compact_svg_path_data_from_segments_without_arcs(area.0, &area.1)
                );
            }
        }

        assert_eq!(accepted_area_components, expected_area_components);
    }
}

#[test]
fn sibling_area_candidate_can_recover_relaxed_mask_loss() {
    let bitmap = rounded_rect_union_bitmap(&[
        (48.0, 45.0, 99.0, 127.0, 25.0),
        (50.0, 100.0, 94.0, 178.0, 11.0),
        (147.0, 143.0, 183.0, 186.0, 9.0),
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
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    assert_eq!(traced.paths.len(), 2);

    let path = traced
        .paths
        .iter()
        .max_by_key(|path| path.points.len())
        .expect("fixture should have a main component");
    let base = pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
        .expect("fixture path should produce base candidate");
    let relaxed = pixel_potrace_segments_for_points(
        path,
        &path.points,
        PIXEL_POTRACE_SIBLING_RELAXED_OPT_TOLERANCE,
        canvas_size,
        false,
    )
    .expect("fixture path should produce relaxed candidate");
    let area = area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture path should produce area candidate");
    let selected = choose_pixel_potrace_point_set_with_context(path, 0.2, canvas_size, false, true)
        .expect("fixture path should produce sibling-aware candidate");

    assert!(pixel_potrace_sibling_relaxed_candidate_is_better(
        path,
        canvas_size,
        &relaxed,
        &base,
    ));
    assert!(pixel_potrace_sibling_area_candidate_is_better(
        path,
        canvas_size,
        &area,
        &relaxed,
    ));
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(area.0, &area.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &area, bitmap.width(), bitmap.height()) + 12
            <= pixel_potrace_candidate_mask_error(path, &relaxed, bitmap.width(), bitmap.height())
    );
}

#[test]
fn coverage_threshold_rasterizer_matches_integer_square_pixels() {
    let path = TracePath {
        is_hole: false,
        points: vec![(2.0, 2.0), (8.0, 2.0), (8.0, 8.0), (2.0, 8.0)],
    };
    let mut center_sampled = vec![false; 10 * 10];
    let mut coverage_sampled = vec![false; 10 * 10];

    rasterize_path_evenodd(&path, 10, 10, &mut center_sampled);
    rasterize_path_evenodd_coverage_threshold(
        &path,
        10,
        10,
        CANDIDATE_MASK_SUPERSAMPLE,
        &mut coverage_sampled,
    );

    assert_eq!(coverage_sampled, center_sampled);
}

#[test]
fn pixel_trace_can_preserve_collinear_boundary_points() {
    let bitmap =
        Bitmap::from_rows(3, 1, &[true, true, true]).expect("bitmap dimensions should match");
    let simplified = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 0,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: false,
        },
    );
    let preserved = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 0,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );

    assert_eq!(simplified.paths[0].points.len(), 4);
    assert_eq!(preserved.paths[0].points.len(), 8);
}

#[test]
fn area_alpha_final_gate_accepts_fragmented_complex_union() {
    let bitmap = rounded_rect_union_bitmap(&[
        (121.0, 58.0, 202.0, 203.0, 8.0),
        (57.0, 114.0, 148.0, 191.0, 11.0),
        (63.0, 109.0, 158.0, 148.0, 10.0),
        (86.0, 99.0, 149.0, 181.0, 10.0),
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
    let best = pixel_potrace_segments_for_points(
        path,
        &path.points,
        0.2,
        Some((bitmap.width(), bitmap.height())),
        false,
    )
    .expect("fixture should produce a candidate");
    let area = area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce area-alpha candidate");

    assert!(
        pixel_potrace_area_alpha_final_candidate_is_better(
            path,
            Some((bitmap.width(), bitmap.height())),
            &area,
            &best,
            true,
        ) || pixel_potrace_area_alpha_smoothing_candidate_is_better(
            path,
            Some((bitmap.width(), bitmap.height())),
            &area,
            &best,
        )
    );
}

#[test]
fn area_alpha_final_gate_rejects_simple_underfit_union() {
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
    let path = traced.paths.first().expect("fixture should trace one path");
    let best = pixel_potrace_segments_for_points(
        path,
        &path.points,
        0.2,
        Some((bitmap.width(), bitmap.height())),
        false,
    )
    .expect("fixture should produce a candidate");
    let area = area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce area-alpha candidate");

    assert!(!pixel_potrace_area_alpha_final_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &area,
        &best,
        true,
    ));
}

#[test]
fn bestpolygon_area_alpha_gate_accepts_offset_t_rescue() {
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
    let best = bestpolygon_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a bestpolygon candidate");
    let area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a bestpolygon area-alpha candidate");

    let selected =
        choose_pixel_potrace_point_set(path, 0.2, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a selected candidate");
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(area.0, &area.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &selected, bitmap.width(), bitmap.height())
            < pixel_potrace_candidate_mask_error(path, &best, bitmap.width(), bitmap.height())
    );
}

#[test]
fn bestpolygon_area_alpha_gate_rejects_wide_h_regression() {
    let bitmap = rounded_rect_union_bitmap(&[
        (48.0, 50.0, 94.0, 204.0, 18.0),
        (162.0, 50.0, 208.0, 204.0, 18.0),
        (48.0, 112.0, 208.0, 152.0, 16.0),
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
    let best = bestpolygon_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a bestpolygon candidate");
    let area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a bestpolygon area-alpha candidate");

    assert!(!pixel_potrace_area_alpha_final_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &area,
        &best,
        false,
    ));

    let selected =
        choose_pixel_potrace_point_set(path, 0.2, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a selected candidate");
    assert_ne!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(area.0, &area.1)
    );
}

#[test]
fn bestpolygon_area_alpha_smoothing_gate_defers_to_capsule_primitive() {
    const CANVAS: usize = 256;
    let start = (34.0, 128.0);
    let end = (222.0, 116.0);
    let half_width = 18.0;
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
    let bitmap = Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture should build");
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
    let best = bestpolygon_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a bestpolygon candidate");
    let area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a bestpolygon area-alpha candidate");
    let primitive = fit_closed_diagonal_capsule_potrace_segments(&path.points)
        .expect("fixture should fit a diagonal capsule primitive");
    let primitive_candidate = (primitive[0].start(), primitive);

    let selected =
        choose_pixel_potrace_point_set(path, 0.2, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a selected candidate");
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(
            primitive_candidate.0,
            &primitive_candidate.1
        )
    );
    assert_ne!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(area.0, &area.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &selected, bitmap.width(), bitmap.height())
            < pixel_potrace_candidate_mask_error(path, &best, bitmap.width(), bitmap.height())
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &selected, bitmap.width(), bitmap.height()) <= 245
    );
}

#[test]
fn high_tolerance_bestpolygon_candidate_can_rescue_smooth_union() {
    let bitmap = rounded_rect_union_bitmap(&[
        (67.0, 137.0, 145.0, 207.0, 13.0),
        (109.0, 112.0, 172.0, 154.0, 16.0),
        (79.0, 64.0, 174.0, 208.0, 17.0),
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
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let base = pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce a base candidate");
    let high_tolerance = bestpolygon_area_alpha_pixel_potrace_segments_for_points(
        &path.points,
        PIXEL_POTRACE_HIGH_OPT_TOLERANCE,
    )
    .expect("fixture should produce high-tolerance best-area candidate");
    let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
        .expect("fixture should produce selected candidate");

    assert!(pixel_potrace_high_tolerance_candidate_is_better(
        path,
        canvas_size,
        &high_tolerance,
        &base,
    ));
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(high_tolerance.0, &high_tolerance.1)
    );
    assert!(selected.1.len().saturating_add(6) <= base.1.len());
}

#[test]
fn high_tolerance_candidate_defers_to_stepped_e_template() {
    let bitmap = rounded_rect_union_bitmap(&[
        (68.0, 54.0, 104.0, 202.0, 12.0),
        (68.0, 54.0, 194.0, 88.0, 12.0),
        (68.0, 112.0, 176.0, 146.0, 12.0),
        (68.0, 168.0, 194.0, 202.0, 12.0),
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
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
        .expect("fixture should produce selected candidate");
    let high_tolerance = bestpolygon_area_alpha_pixel_potrace_segments_for_points(
        &path.points,
        PIXEL_POTRACE_HIGH_OPT_TOLERANCE,
    )
    .expect("fixture should produce high-tolerance best-area candidate");

    assert!(pixel_potrace_points_match_high_tolerance_protected_template(&path.points));
    assert!(pixel_potrace_high_tolerance_candidate_is_better(
        path,
        canvas_size,
        &high_tolerance,
        &selected,
    ));
    assert_ne!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(high_tolerance.0, &high_tolerance.1)
    );
    let selected_path = compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1);
    let stepped_e_candidates = closed_stepped_e_potrace_candidates(&path.points)
        .expect("fixture should produce stepped-E candidates");
    assert!(stepped_e_candidates.into_iter().any(|segments| {
        compact_svg_path_data_from_segments_without_arcs(segments[0].start(), &segments)
            == selected_path
    }));
}
