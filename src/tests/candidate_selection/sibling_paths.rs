use super::*;

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
