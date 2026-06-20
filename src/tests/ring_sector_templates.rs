use super::*;

#[test]
fn pixel_ring_sector_uses_annular_sector_fallback() {
    let bitmap = ring_sector_bitmap(70.0, 290.0, 38.0, 80.0);
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
    let segments = fit_closed_annular_sector_potrace_segments(
        &path.points,
        Some((bitmap.width(), bitmap.height())),
    )
    .expect("ring sector should fit annular-sector fallback");
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
    .expect("ring sector path should render");

    assert!(segments.len() <= 12, "{segments:?}");
    assert!(compact_path_command_count(&data) <= 24, "{data}");
}

#[test]
fn pixel_thin_ring_sector_rejects_annular_sector_fallback() {
    let bitmap = ring_sector_bitmap(120.0, 420.0, 48.0, 82.0);
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

    assert!(fit_closed_annular_sector_potrace_segments(
        &path.points,
        Some((bitmap.width(), bitmap.height()))
    )
    .is_none());
}

#[test]
fn pixel_narrow_gap_ring_sector_keeps_existing_trace_path() {
    let bitmap = ring_sector_bitmap(30.0, 310.0, 42.0, 78.0);
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

    assert!(fit_closed_annular_sector_potrace_segments(
        &path.points,
        Some((bitmap.width(), bitmap.height()))
    )
    .is_none());
}

#[test]
fn pixel_narrow_gap_ring_sector_can_use_bestpolygon_candidate() {
    let bitmap = ring_sector_bitmap(30.0, 310.0, 42.0, 78.0);
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
    let base = base_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a base candidate");
    let bestpolygon = bestpolygon_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce a bestpolygon candidate");

    let base_error =
        pixel_potrace_candidate_mask_error(path, &base, bitmap.width(), bitmap.height());
    let bestpolygon_error =
        pixel_potrace_candidate_mask_error(path, &bestpolygon, bitmap.width(), bitmap.height());
    let base_bytes = compact_svg_path_data_from_segments_without_arcs(base.0, &base.1).len();
    let bestpolygon_bytes =
        compact_svg_path_data_from_segments_without_arcs(bestpolygon.0, &bestpolygon.1).len();

    assert!(bestpolygon_error <= base_error);
    assert!(
        bestpolygon_bytes < base_bytes,
        "{base_bytes} <= {bestpolygon_bytes}"
    );
    assert!(pixel_potrace_candidate_is_better(
        path,
        Some((bitmap.width(), bitmap.height())),
        &bestpolygon,
        &base
    ));

    let selected =
        choose_pixel_potrace_point_set(path, 0.2, Some((bitmap.width(), bitmap.height())), false)
            .expect("fixture should produce a selected candidate");
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(bestpolygon.0, &bestpolygon.1)
    );
}
