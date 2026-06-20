use super::*;

#[test]
fn pixel_ring_sector_uses_annular_sector_fallback() {
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
