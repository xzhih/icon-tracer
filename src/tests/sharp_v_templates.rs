use super::*;

#[test]
fn pixel_sharp_v_keeps_straight_edges_compact() {
    let bitmap = sharp_v_polygon_bitmap();
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
    let polygon = relaxed_optimal_potrace_polygon_indices(&path.points);
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
    .expect("sharp-v path should render");

    assert!(polygon.len() <= 9, "{polygon:?}");
    assert!(compact_path_command_count(&data) <= 17, "{data}");
}
