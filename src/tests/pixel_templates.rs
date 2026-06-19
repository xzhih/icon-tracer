use super::*;

#[test]
fn closed_ellipse_potrace_fit_uses_five_cubics() {
    let points = (0..64)
        .map(|index| {
            let angle = index as f64 * std::f64::consts::TAU / 64.0;
            (40.0 + angle.cos() * 20.0, 30.0 + angle.sin() * 12.0)
        })
        .collect::<Vec<_>>();

    let segments = fit_closed_ellipse_potrace_segments(&points)
        .expect("ellipse-like points should fit the primitive");

    assert_eq!(segments.len(), 5);
    assert!(segments
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_))));
}

#[test]
fn closed_smooth_ellipse_fit_removes_half_pixel_bias() {
    let points = (0..64)
        .map(|index| {
            let angle = index as f64 * std::f64::consts::TAU / 64.0;
            (256.5 + angle.cos() * 148.5, 256.5 + angle.sin() * 148.5)
        })
        .collect::<Vec<_>>();

    let pixel_segments = fit_closed_ellipse_potrace_segments(&points)
        .expect("ellipse-like points should fit the pixel primitive");
    let smooth_segments = fit_closed_smooth_ellipse_segments(&points)
        .expect("ellipse-like points should fit the smooth primitive");

    assert_eq!(smooth_segments.len(), 5);
    assert!(smooth_segments[0].start().0 < pixel_segments[0].start().0);
    assert!(smooth_segments[0].start().1 < pixel_segments[0].start().1);
}

#[test]
fn closed_small_capsule_potrace_fit_uses_small_template() {
    let center_y = 40.0;
    let radius = 20.0;
    let left_center = (30.0, center_y);
    let right_center = (70.0, center_y);
    let mut points = Vec::new();

    for index in 0..=8 {
        points.push((30.0 + index as f64 * 5.0, center_y - radius));
    }
    for index in 1..=16 {
        let angle = -std::f64::consts::FRAC_PI_2 + index as f64 * std::f64::consts::PI / 16.0;
        points.push((
            right_center.0 + angle.cos() * radius,
            right_center.1 + angle.sin() * radius,
        ));
    }
    for index in 1..=8 {
        points.push((70.0 - index as f64 * 5.0, center_y + radius));
    }
    for index in 1..=16 {
        let angle = std::f64::consts::FRAC_PI_2 + index as f64 * std::f64::consts::PI / 16.0;
        points.push((
            left_center.0 + angle.cos() * radius,
            left_center.1 + angle.sin() * radius,
        ));
    }

    let segments = fit_closed_capsule_potrace_segments(&points)
        .expect("capsule-like points should fit the primitive");
    let cubic_count = segments
        .iter()
        .filter(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
        .count();
    let line_count = segments
        .iter()
        .filter(|segment| matches!(segment, SvgPathSegment::Line { .. }))
        .count();

    assert_eq!(segments.len(), 11);
    assert_eq!(cubic_count, 5);
    assert_eq!(line_count, 6);
}

#[test]
fn pixel_rounded_rect_trace_avoids_fragmented_stair_step_path() {
    let bitmap = parity_rounded_rect_bitmap(18.0);
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
    .expect("rounded rectangle path should render");
    let command_count = compact_path_command_count(&data);
    assert!(
        command_count <= 13,
        "rounded rectangle trace fragmented into too many commands: {data}"
    );
}

#[test]
fn pixel_small_circle_primitive_uses_potrace_three_cubic_template() {
    let bitmap = parity_two_circles_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: false,
        },
    );
    let svg = traced.to_svg_with_render_options(SvgRenderOptions {
        curve_mode: CurveMode::Potrace,
        opt_tolerance: 0.2,
        pixel_potrace: true,
    });

    assert!(svg.contains("translate(0 256) scale(.1 -.1)"), "{svg}");
    assert!(svg.contains("M730 1684c-308-82"), "{svg}");
    assert!(svg.contains("M1610 1684c-308-82"), "{svg}");
}

#[test]
fn pixel_ring_primitive_uses_potrace_hole_template() {
    let bitmap = parity_ring_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: false,
        },
    );
    let svg = traced.to_svg_with_render_options(SvgRenderOptions {
        curve_mode: CurveMode::Potrace,
        opt_tolerance: 0.2,
        pixel_potrace: true,
    });

    assert!(svg.contains("translate(0 256) scale(.1 -.1)"), "{svg}");
    assert!(svg.contains("M523 1090c60-223"), "{svg}");
    assert!(svg.contains("M1385 1685c312-81"), "{svg}");
}

#[test]
fn pixel_triangle_primitive_uses_potrace_like_segments() {
    let bitmap = parity_triangle_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: false,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
    let primitive = fit_closed_upright_triangle_potrace_segments(&path.points)
        .expect("upright triangle should fit the primitive");
    let candidate = (primitive[0].start(), primitive.clone());
    let candidate_error =
        pixel_potrace_candidate_mask_error(path, &candidate, bitmap.width(), bitmap.height());
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
    .expect("triangle path should render");

    assert_eq!(primitive.len(), 5);
    assert!(
        primitive
            .iter()
            .filter(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
            .count()
            >= 2
    );
    assert_eq!(
        data, "M42 214l172 0-42.7-85.5c-23.6-47-43-85.5-43.3-85.5s-19.7 38.5-43.3 85.5Z",
        "candidate_error={candidate_error}"
    );
}

#[test]
fn pixel_capsule_primitive_uses_potrace_like_cubics() {
    let bounds = FloatBounds {
        min_x: 40.0,
        max_x: 216.0,
        min_y: 80.0,
        max_y: 176.0,
    };
    let segments = cleanup_potrace_segments(
        horizontal_capsule_segments(bounds, 48.0),
        PIXEL_POTRACE_LINEAR_DEVIATION,
    );
    let path_data = compact_svg_path_data_from_segments(segments[0].start(), &segments);

    assert_eq!(
        path_data,
        "M76.1 81.6c-25.3 6.8-41 33.1-34.6 57.9 4.5 17.2 17.9 30.5 35.2 35 8.5 2.2 94.2 2.2 102.8 0 25.6-6.7 41.5-32.9 35-57.8-4.5-17.3-17.8-30.7-35-35.2-8.4-2.2-95.2-2.1-103.4.1Z"
    );
}

#[test]
fn pixel_double_pill_uses_small_capsule_template() {
    let bitmap = parity_double_pill_bitmap();
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
    assert!(svg.contains("M1986 1405l41 27"), "{svg}");
    assert!(svg.contains("M1986 765l41 27"), "{svg}");
    assert!(svg.contains("-690 2c-419 1-704-2-724-8"), "{svg}");
}

#[test]
fn pixel_plus_shape_uses_potrace_template() {
    let bitmap = parity_plus_bitmap();
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
    assert!(svg.contains("M1500 1500l0 218"), "{svg}");
    assert!(svg.contains("c0 270-12 304-113 347"), "{svg}");
    assert!(svg.contains("c-225 0-261-6-309-48"), "{svg}");
}

#[test]
fn pixel_l_shape_uses_potrace_template() {
    let bitmap = parity_l_shape_bitmap();
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
    assert!(svg.contains("M1020 960v468"), "{svg}");
    assert!(svg.contains("21 10 50 27 57 37"), "{svg}");
    assert!(svg.contains("-52 46-70 48-499 48Z"), "{svg}");
}

#[test]
fn pixel_diagonal_capsule_primitive_uses_potrace_like_cubics() {
    let bitmap = parity_diagonal_bar_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: false,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
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
    .expect("diagonal capsule path should render");

    assert_eq!(
        data,
        "M49.57 199.24c5.33 4.73 13.46 5.96 19.67 3.07 2-.9 34.32-28.61 71.64-61.52 72.04-63.23 71.24-62.53 71.21-71.06-.01-3.81-3.04-10.55-5.66-12.97-5.33-4.73-13.46-5.96-19.67-3.07-2 .9-34.32 28.61-71.64 61.52-72.04 63.23-71.24 62.53-71.21 71.06.01 3.81 3.04 10.55 5.66 12.97Z"
    );
}

#[test]
fn pixel_chevron_primitive_uses_potrace_template() {
    let bitmap = parity_chevron_bitmap();
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
    .expect("chevron path should render");
    let command_count = compact_path_command_count(&data);

    assert!(command_count <= 5, "{data}");
    assert!(data.contains("59.6 124.2"), "{data}");
}

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
