use super::*;

#[test]
fn loose_opt_candidate_can_rescue_oversegmented_union() {
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
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
        .expect("fixture should produce a selected candidate");
    let base = pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce a base candidate");
    let loose = pixel_potrace_segments_for_points(
        path,
        &path.points,
        PIXEL_POTRACE_LOOSE_OPT_TOLERANCE,
        canvas_size,
        false,
    )
    .expect("fixture should produce a loose candidate");

    assert!(pixel_potrace_loose_candidate_is_better(
        path,
        canvas_size,
        &loose,
        &base,
    ));
    assert_eq!(
        compact_svg_path_data_from_segments(selected.0, &selected.1),
        compact_svg_path_data_from_segments(loose.0, &loose.1)
    );
    assert!(selected.1.len() < base.1.len());
}

#[test]
fn loose_opt_candidate_rejects_known_global_tolerance_regressions() {
    let fixtures = [
        rounded_rect_union_bitmap(&[
            (58.0, 52.0, 102.0, 204.0, 18.0),
            (58.0, 52.0, 198.0, 96.0, 18.0),
            (154.0, 52.0, 198.0, 142.0, 18.0),
        ]),
        local_capsule_bitmap((34.0, 128.0), (222.0, 116.0), 18.0),
        rounded_rect_union_bitmap(&[
            (102.0, 162.0, 163.0, 219.0, 19.0),
            (107.0, 156.0, 142.0, 191.0, 12.0),
            (124.0, 93.0, 180.0, 213.0, 19.0),
        ]),
        rounded_rect_union_bitmap(&[
            (70.0, 54.0, 106.0, 200.0, 14.0),
            (150.0, 54.0, 186.0, 200.0, 14.0),
            (70.0, 112.0, 186.0, 148.0, 14.0),
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
        let path = traced.paths.first().expect("fixture should trace one path");
        let canvas_size = Some((bitmap.width(), bitmap.height()));
        let base = pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
            .expect("fixture should produce a base candidate");
        let loose = pixel_potrace_segments_for_points(
            path,
            &path.points,
            PIXEL_POTRACE_LOOSE_OPT_TOLERANCE,
            canvas_size,
            false,
        )
        .expect("fixture should produce a loose candidate");

        assert!(!pixel_potrace_loose_candidate_is_better(
            path,
            canvas_size,
            &loose,
            &base,
        ));
    }
}

#[test]
fn bestpolygon_area_alpha_candidate_stays_bounded_when_fine_detail_wins() {
    let bitmap = rounded_rect_union_bitmap(&[
        (86.0, 49.0, 168.0, 132.0, 23.0),
        (43.0, 60.0, 156.0, 143.0, 10.0),
        (56.0, 50.0, 114.0, 138.0, 18.0),
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
    let base = pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce a base candidate");
    let best_area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a bestpolygon area-alpha candidate");
    let fine =
        choose_pixel_potrace_point_set(path, PIXEL_POTRACE_FINE_OPT_TOLERANCE, canvas_size, false)
            .expect("fixture should produce a full fine candidate");

    assert!(pixel_potrace_best_area_candidate_is_better(
        path,
        canvas_size,
        &best_area,
        &base,
    ));
    assert_eq!(
        compact_svg_path_data_from_segments(selected.0, &selected.1),
        compact_svg_path_data_from_segments(fine.0, &fine.1)
    );
    let selected_error =
        pixel_potrace_candidate_mask_error(path, &selected, bitmap.width(), bitmap.height());
    let best_area_error =
        pixel_potrace_candidate_mask_error(path, &best_area, bitmap.width(), bitmap.height());
    let base_error =
        pixel_potrace_candidate_mask_error(path, &base, bitmap.width(), bitmap.height());

    assert!(selected.1.len() < base.1.len());
    assert!(selected_error <= best_area_error.saturating_add(4));
    assert!(selected_error <= base_error + 16);
}

#[test]
fn bestpolygon_area_alpha_candidate_can_rescue_mask_better_unions() {
    let fixtures = [
        rounded_rect_union_bitmap(&[
            (62.0, 65.0, 104.0, 125.0, 15.0),
            (88.0, 97.0, 145.0, 172.0, 27.0),
            (36.0, 66.0, 146.0, 205.0, 28.0),
        ]),
        rounded_rect_union_bitmap(&[
            (76.0, 46.0, 118.0, 170.0, 10.0),
            (155.0, 73.0, 190.0, 124.0, 17.0),
            (80.0, 55.0, 188.0, 147.0, 25.0),
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
        let path = traced.paths.first().expect("fixture should trace one path");
        let canvas_size = Some((bitmap.width(), bitmap.height()));
        let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
            .expect("fixture should produce a selected candidate");
        let base = pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
            .expect("fixture should produce a base candidate");
        let best_area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
            .expect("fixture should produce a bestpolygon area-alpha candidate");

        assert!(pixel_potrace_best_area_candidate_is_better(
            path,
            canvas_size,
            &best_area,
            &base,
        ));
        assert_eq!(
            compact_svg_path_data_from_segments(selected.0, &selected.1),
            compact_svg_path_data_from_segments(best_area.0, &best_area.1)
        );

        let selected_error =
            pixel_potrace_candidate_mask_error(path, &selected, bitmap.width(), bitmap.height());
        let base_error =
            pixel_potrace_candidate_mask_error(path, &base, bitmap.width(), bitmap.height());
        assert!(selected_error < base_error);
        assert!(selected.1.len().saturating_add(8) <= base.1.len());
    }
}

#[test]
fn bestpolygon_area_alpha_candidate_can_accept_strict_area_improvement() {
    let fixtures = [
        rounded_rect_union_bitmap(&[
            (84.0, 50.0, 203.0, 195.0, 12.0),
            (74.0, 95.0, 116.0, 149.0, 18.0),
            (98.0, 186.0, 159.0, 219.0, 10.0),
        ]),
        rounded_rect_union_bitmap(&[
            (102.0, 162.0, 163.0, 219.0, 19.0),
            (107.0, 156.0, 142.0, 191.0, 12.0),
            (124.0, 93.0, 180.0, 213.0, 19.0),
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
        let path = traced.paths.first().expect("fixture should trace one path");
        let canvas_size = Some((bitmap.width(), bitmap.height()));
        let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
            .expect("fixture should produce selected candidate");
        let area = area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
            .expect("fixture should produce area-alpha candidate");
        let best_area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
            .expect("fixture should produce bestpolygon area-alpha candidate");

        assert!(pixel_potrace_best_area_candidate_is_better(
            path,
            canvas_size,
            &best_area,
            &area,
        ));
        assert_eq!(
            compact_svg_path_data_from_segments(selected.0, &selected.1),
            compact_svg_path_data_from_segments(best_area.0, &best_area.1)
        );
        assert!(best_area.1.len() <= area.1.len());
        assert!(
            compact_svg_path_data_from_segments_without_arcs(best_area.0, &best_area.1).len()
                <= compact_svg_path_data_from_segments_without_arcs(area.0, &area.1).len()
        );
        assert!(
            pixel_potrace_candidate_mask_error(path, &best_area, bitmap.width(), bitmap.height())
                < pixel_potrace_candidate_mask_error(path, &area, bitmap.width(), bitmap.height())
        );
        assert!(
            pixel_potrace_candidate_boundary_rms_error(path, &best_area)
                < pixel_potrace_candidate_boundary_rms_error(path, &area)
        );
        assert!(
            pixel_potrace_candidate_foreground_delta(
                path,
                &best_area,
                bitmap.width(),
                bitmap.height()
            )
            .unsigned_abs()
                < pixel_potrace_candidate_foreground_delta(
                    path,
                    &area,
                    bitmap.width(),
                    bitmap.height()
                )
                .unsigned_abs()
        );
    }
}

#[test]
fn bestpolygon_area_alpha_candidate_rejects_template_regressions() {
    let fixtures = [
        rounded_rect_union_bitmap(&[
            (68.0, 54.0, 104.0, 202.0, 12.0),
            (68.0, 54.0, 194.0, 88.0, 12.0),
            (68.0, 112.0, 176.0, 146.0, 12.0),
            (68.0, 168.0, 194.0, 202.0, 12.0),
        ]),
        rounded_rect_union_bitmap(&[
            (48.0, 44.0, 92.0, 190.0, 18.0),
            (154.0, 58.0, 204.0, 198.0, 20.0),
            (48.0, 146.0, 204.0, 204.0, 22.0),
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
        let path = traced.paths.first().expect("fixture should trace one path");
        let canvas_size = Some((bitmap.width(), bitmap.height()));
        let base = pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
            .expect("fixture should produce a base candidate");
        let best_area = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
            .expect("fixture should produce a bestpolygon area-alpha candidate");

        assert!(!pixel_potrace_best_area_candidate_is_better(
            path,
            canvas_size,
            &best_area,
            &base,
        ));
    }
}

fn local_capsule_bitmap(start: (f64, f64), end: (f64, f64), half_width: f64) -> Bitmap {
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
