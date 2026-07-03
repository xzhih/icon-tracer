use icon_tracer::{trace_image_to_svg, ContourMode, CurveMode, TraceImageOptions, TracePreset};

#[test]
fn trace_image_to_svg_accepts_encoded_bitmap_bytes() {
    let svg = trace_image_to_svg(
        b"P1\n2 2\n1 0\n0 1\n",
        TraceImageOptions::preset(TracePreset::Default),
    )
    .expect("valid PBM should trace");

    assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#));
    assert!(svg.contains(r#"viewBox="0 0 2 2""#));
    assert!(svg.contains("<path"));
}

#[test]
fn trace_image_options_expose_icon_preset_defaults() {
    let options = TraceImageOptions::preset(TracePreset::Icon);

    assert_eq!(options.trace_options.contour_mode, ContourMode::Subpixel);
    assert_eq!(options.svg_render_options.curve_mode, CurveMode::Potrace);
    assert_eq!(options.trace_options.turd_size, 2);
    assert_eq!(options.trace_options.opt_tolerance, 0.75);
    assert!(!options.optimize_icon);
    assert!(!options.isolate_foreground);
}
