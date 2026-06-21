use super::*;

#[test]
fn pixel_vertical_rounded_rect_union_uses_potrace_template() {
    let bitmap = rounded_rect_union_bitmap(&[
        (106.0, 71.0, 195.0, 154.0, 12.0),
        (106.0, 53.0, 193.0, 195.0, 21.0),
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
    let segments = fit_closed_rounded_rect_potrace_segments(&path.points)
        .expect("vertical rounded rect should fit a Potrace-derived template");
    let candidate = (segments[0].start(), segments.clone());
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
    .expect("vertical rounded rect path should render");

    assert_eq!(segments.len(), 10, "{segments:?}");
    assert!(
        pixel_potrace_candidate_mask_error(path, &candidate, bitmap.width(), bitmap.height())
            <= 170
    );
    assert!((230..=260).contains(&data.len()), "{data}");
}
