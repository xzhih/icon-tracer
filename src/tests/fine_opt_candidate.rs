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
