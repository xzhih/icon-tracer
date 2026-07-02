use super::*;

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
