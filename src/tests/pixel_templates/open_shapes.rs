use super::*;

#[test]
fn pixel_u_shape_template_matches_potrace_mask() {
    let bitmap = parity_u_shape_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
    let final_data = path_to_svg_data(
        path,
        SvgRenderOptions {
            curve_mode: CurveMode::Potrace,
            opt_tolerance: 0.2,
            pixel_potrace: true,
        },
        Some((bitmap.width(), bitmap.height())),
        false,
    )
    .expect("U-shaped path should render");

    assert_eq!(compact_path_command_count(&final_data), 12, "{final_data}");
    assert!(
        final_data.starts_with("M160 152l-32 0-32 0"),
        "{final_data}"
    );
    assert!(
        final_data.contains("c-3.4-6.4-8.8-9-18.7-9"),
        "{final_data}"
    );
}

#[test]
fn pixel_u_shape_template_accepts_inverted_orientation() {
    let bitmap = mirror_bitmap_y(&parity_u_shape_bitmap());
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
    let segments = fit_closed_staple_potrace_segments(&path.points)
        .expect("inverted U shape should fit a direction-specific Potrace template");

    assert_eq!(segments.len(), 24);
}

#[test]
fn pixel_u_shape_template_accepts_rotated_orientations() {
    for bitmap in [
        rotate_bitmap_clockwise(&parity_u_shape_bitmap()),
        rotate_bitmap_counter_clockwise(&parity_u_shape_bitmap()),
    ] {
        let traced = trace_bitmap(
            &bitmap,
            TraceOptions {
                turd_size: 2,
                opt_tolerance: 0.2,
                contour_mode: ContourMode::Pixel,
                preserve_collinear: true,
            },
        );
        let path = traced.paths.first().expect("fixture should trace one path");
        let segments = fit_closed_staple_potrace_segments(&path.points)
            .expect("rotated U shape should fit a direction-specific Potrace template");

        assert!(matches!(segments.len(), 24 | 26), "{segments:?}");
    }
}

#[test]
fn pixel_c_shape_template_matches_potrace_output() {
    let bitmap = parity_c_shape_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
    let final_data = path_to_svg_data(
        path,
        SvgRenderOptions {
            curve_mode: CurveMode::Potrace,
            opt_tolerance: 0.2,
            pixel_potrace: true,
        },
        Some((bitmap.width(), bitmap.height())),
        false,
    )
    .expect("C-shaped path should render");

    assert_eq!(compact_path_command_count(&final_data), 8, "{final_data}");
    assert_eq!(
        final_data,
        "M195.4 89c-4.1-7.5-12.4-17.1-19.5-22.5-7.5-5.6-19.9-11.8-28.4-14.1-8.3-2.2-26.4-2.7-34.5-.9-29.8 6.3-52.8 28.2-60.7 57.5-2.4 8.9-2.4 29.1 0 38 6 22.3 20.9 40.5 41.2 50.6 12.9 6.3 19.6 7.9 34.5 7.9 14.5 0 21.3-1.5 33.5-7.4 14.5-7 27-18.4 33.9-31.1l2.7-5h-21.5c-13.5 0-21.7.4-22.1 1-1 1.6-10.5 6.2-16 7.6-26.8 7.2-54.5-14.5-54.5-42.6 0-15.7 9.6-31.7 23.1-38.6 14.1-7.1 27.6-7.2 41.6-.1 2.8 1.5 5.4 3.1 5.8 3.7s8.6 1 22.1 1h21.5Z"
    );
}

#[test]
fn pixel_c_shape_template_accepts_mirrored_and_rotated_orientations() {
    for bitmap in [
        mirror_bitmap_x(&parity_c_shape_bitmap()),
        rotate_bitmap_clockwise(&parity_c_shape_bitmap()),
        rotate_bitmap_counter_clockwise(&parity_c_shape_bitmap()),
    ] {
        let traced = trace_bitmap(
            &bitmap,
            TraceOptions {
                turd_size: 2,
                opt_tolerance: 0.2,
                contour_mode: ContourMode::Pixel,
                preserve_collinear: true,
            },
        );
        let path = traced.paths.first().expect("fixture should trace one path");
        let segments = fit_closed_open_ring_potrace_segments(&path.points)
            .expect("oriented C shape should fit a direction-specific Potrace template");

        assert!(matches!(segments.len(), 19..=21), "{segments:?}");
    }
}

#[test]
fn pixel_c_shape_svg_keeps_potrace_scaled_precision() {
    let bitmap = parity_c_shape_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let svg = traced.to_svg_with_render_options(SvgRenderOptions {
        curve_mode: CurveMode::Potrace,
        opt_tolerance: 0.2,
        pixel_potrace: true,
    });

    assert!(svg.contains("translate(0 256) scale(.1 -.1)"), "{svg}");
    assert!(!svg.contains(r#"transform="scale(.01)""#), "{svg}");
}

#[test]
fn pixel_f_shape_template_matches_potrace_mask() {
    let bitmap = parity_f_shape_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let svg = traced.to_svg_with_render_options(SvgRenderOptions {
        curve_mode: CurveMode::Potrace,
        opt_tolerance: 0.2,
        pixel_potrace: true,
    });

    assert!(svg.contains("translate(0 256) scale(.1 -.1)"), "{svg}");
    assert!(svg.contains("M920 1882l0-131"), "{svg}");
    assert!(svg.contains("281 0 281 0"), "{svg}");
}

#[test]
fn pixel_e_shape_template_matches_potrace_mask() {
    let bitmap = parity_e_shape_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let svg = traced.to_svg_with_render_options(SvgRenderOptions {
        curve_mode: CurveMode::Potrace,
        opt_tolerance: 0.2,
        pixel_potrace: true,
    });

    assert!(svg.contains("translate(0 256) scale(.1 -.1)"), "{svg}");
    assert!(svg.contains("M1040 1599l419 3"), "{svg}");
    assert!(svg.contains("339 3c325 3"), "{svg}");
}

#[test]
fn pixel_e_shape_template_accepts_mirrored_and_rotated_orientations() {
    for bitmap in [
        mirror_bitmap_x(&parity_e_shape_bitmap()),
        rotate_bitmap_clockwise(&parity_e_shape_bitmap()),
        rotate_bitmap_half_turn(&parity_e_shape_bitmap()),
        rotate_bitmap_counter_clockwise(&parity_e_shape_bitmap()),
    ] {
        let traced = trace_bitmap(
            &bitmap,
            TraceOptions {
                turd_size: 2,
                opt_tolerance: 0.2,
                contour_mode: ContourMode::Pixel,
                preserve_collinear: true,
            },
        );
        let path = traced.paths.first().expect("fixture should trace one path");
        let segments = fit_closed_stepped_e_potrace_segments(&path.points)
            .expect("oriented E shape should fit a direction-specific Potrace template");
        let final_data = path_to_svg_data(
            path,
            SvgRenderOptions {
                curve_mode: CurveMode::Potrace,
                opt_tolerance: 0.2,
                pixel_potrace: true,
            },
            Some((bitmap.width(), bitmap.height())),
            false,
        )
        .expect("oriented E path should render");

        assert!(matches!(segments.len(), 26 | 27), "{segments:?}");
        assert!(
            compact_path_command_count(&final_data) <= 27,
            "{final_data}"
        );
    }
}
