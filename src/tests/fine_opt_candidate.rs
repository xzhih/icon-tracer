use super::*;

#[test]
fn fine_opt_candidate_can_rescue_fragmented_union() {
    let bitmap = rounded_rect_union_bitmap(&[
        (167.0, 143.0, 208.0, 190.0, 10.0),
        (98.0, 143.0, 173.0, 194.0, 9.0),
        (76.0, 123.0, 193.0, 164.0, 19.0),
        (99.0, 110.0, 175.0, 218.0, 22.0),
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
    let coarse = pixel_potrace_segments_for_points(path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce a coarse candidate");
    let fine = pixel_potrace_segments_for_points(
        path,
        &path.points,
        PIXEL_POTRACE_FINE_OPT_TOLERANCE,
        canvas_size,
        false,
    )
    .expect("fixture should produce a fine candidate");

    assert_eq!(
        compact_svg_path_data_from_segments(selected.0, &selected.1),
        compact_svg_path_data_from_segments(fine.0, &fine.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &selected, bitmap.width(), bitmap.height())
            < pixel_potrace_candidate_mask_error(path, &coarse, bitmap.width(), bitmap.height())
    );
}

#[test]
fn fine_opt_candidate_rejects_tiny_input_gain_regression() {
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
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let selected = choose_pixel_potrace_point_set(path, 0.2, canvas_size, false)
        .expect("fixture should produce a selected candidate");
    let fine = pixel_potrace_segments_for_points(
        path,
        &path.points,
        PIXEL_POTRACE_FINE_OPT_TOLERANCE,
        canvas_size,
        false,
    )
    .expect("fixture should produce a fine candidate");

    assert!(!pixel_potrace_fine_candidate_is_better(
        path,
        canvas_size,
        &fine,
        &selected,
    ));
    assert_ne!(
        compact_svg_path_data_from_segments(selected.0, &selected.1),
        compact_svg_path_data_from_segments(fine.0, &fine.1)
    );
}

#[test]
fn fine_detail_candidate_rejects_boundary_regressing_detail_growth() {
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
        .expect("fixture should produce selected candidate");
    let pre_fine = bestpolygon_area_alpha_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a pre-fine best-area candidate");
    let fine =
        choose_pixel_potrace_point_set(path, PIXEL_POTRACE_FINE_OPT_TOLERANCE, canvas_size, false)
            .expect("fixture should produce a full fine candidate");

    assert!(!pixel_potrace_fine_detail_candidate_is_better(
        path,
        canvas_size,
        &fine,
        &pre_fine,
    ));
    assert_eq!(
        compact_svg_path_data_from_segments(selected.0, &selected.1),
        compact_svg_path_data_from_segments(pre_fine.0, &pre_fine.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &fine, bitmap.width(), bitmap.height())
            <= pixel_potrace_candidate_mask_error(path, &pre_fine, bitmap.width(), bitmap.height())
                .saturating_add(4)
    );
    assert!(
        pixel_potrace_candidate_boundary_rms_error(path, &fine)
            > pixel_potrace_candidate_boundary_rms_error(path, &pre_fine)
    );
}

#[test]
fn fine_opt_candidate_rejects_bounded_detail_mask_regression() {
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
    .expect("fixture should produce a fine candidate");

    assert!(!pixel_potrace_fine_candidate_is_better(
        path,
        canvas_size,
        &fine,
        &selected,
    ));
    assert_ne!(
        compact_svg_path_data_from_segments(selected.0, &selected.1),
        compact_svg_path_data_from_segments(fine.0, &fine.1)
    );
}

#[test]
fn fine_detail_candidate_rejects_simplifying_fine_result() {
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
        .expect("fixture should produce selected candidate");
    let fine =
        choose_pixel_potrace_point_set(path, PIXEL_POTRACE_FINE_OPT_TOLERANCE, canvas_size, false)
            .expect("fixture should produce full fine candidate");

    assert_ne!(
        compact_svg_path_data_from_segments(selected.0, &selected.1),
        compact_svg_path_data_from_segments(fine.0, &fine.1)
    );
}
