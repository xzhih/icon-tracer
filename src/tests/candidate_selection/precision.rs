use super::*;

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
