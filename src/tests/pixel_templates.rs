use super::*;

mod open_shapes;

fn mirror_bitmap_x(bitmap: &Bitmap) -> Bitmap {
    let width = bitmap.width();
    let height = bitmap.height();
    let pixels = (0..height)
        .flat_map(|y| (0..width).map(move |x| bitmap.is_black(width - 1 - x, y)))
        .collect::<Vec<_>>();

    Bitmap::from_rows(width, height, &pixels).expect("mirrored fixture pixels should match canvas")
}

fn mirror_bitmap_y(bitmap: &Bitmap) -> Bitmap {
    let width = bitmap.width();
    let height = bitmap.height();
    let pixels = (0..height)
        .flat_map(|y| (0..width).map(move |x| bitmap.is_black(x, height - 1 - y)))
        .collect::<Vec<_>>();

    Bitmap::from_rows(width, height, &pixels).expect("mirrored fixture pixels should match canvas")
}

fn rotate_bitmap_clockwise(bitmap: &Bitmap) -> Bitmap {
    let width = bitmap.width();
    let height = bitmap.height();
    assert_eq!(
        width, height,
        "test rotation helper currently expects square fixtures"
    );
    let pixels = (0..height)
        .flat_map(|y| (0..width).map(move |x| bitmap.is_black(y, height - 1 - x)))
        .collect::<Vec<_>>();

    Bitmap::from_rows(width, height, &pixels).expect("rotated fixture pixels should match canvas")
}

fn rotate_bitmap_counter_clockwise(bitmap: &Bitmap) -> Bitmap {
    let width = bitmap.width();
    let height = bitmap.height();
    assert_eq!(
        width, height,
        "test rotation helper currently expects square fixtures"
    );
    let pixels = (0..height)
        .flat_map(|y| (0..width).map(move |x| bitmap.is_black(width - 1 - y, x)))
        .collect::<Vec<_>>();

    Bitmap::from_rows(width, height, &pixels).expect("rotated fixture pixels should match canvas")
}

fn rotate_bitmap_half_turn(bitmap: &Bitmap) -> Bitmap {
    mirror_bitmap_y(&mirror_bitmap_x(bitmap))
}

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
fn pixel_complex_residue_union_uses_scaled_precision() {
    let bitmap = rounded_rect_union_bitmap(&[
        (106.0, 42.0, 150.0, 212.0, 17.0),
        (58.0, 64.0, 198.0, 112.0, 20.0),
    ]);
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: false,
        },
    );
    let svg = traced.to_svg_with_render_options(SvgRenderOptions {
        curve_mode: CurveMode::Potrace,
        opt_tolerance: 0.2,
        pixel_potrace: true,
    });

    assert!(svg.contains(r#"transform="scale(.01)""#), "{svg}");
    assert!(!svg.contains("scale(.1 -.1)"), "{svg}");
}

#[test]
fn pixel_diagonal_bar_keeps_y_flipped_integer_precision() {
    let bitmap = parity_diagonal_bar_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
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
    assert!(!svg.contains(r#"transform="scale(.01)""#), "{svg}");
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
fn pixel_l_shape_template_accepts_mirrored_orientation() {
    let bitmap = mirror_bitmap_x(&parity_l_shape_bitmap());
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
    let segments = fit_closed_l_potrace_segments(&path.points)
        .expect("mirrored L shape should fit the transformed Potrace template");

    assert!(matches!(segments.len(), 12..=14), "{segments:?}");
}

#[test]
fn pixel_l_shape_template_accepts_mirrored_and_rotated_orientations() {
    for bitmap in [
        mirror_bitmap_x(&parity_l_shape_bitmap()),
        mirror_bitmap_y(&parity_l_shape_bitmap()),
        rotate_bitmap_clockwise(&parity_l_shape_bitmap()),
        rotate_bitmap_half_turn(&parity_l_shape_bitmap()),
        rotate_bitmap_counter_clockwise(&parity_l_shape_bitmap()),
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
        let segments = fit_closed_l_potrace_segments(&path.points)
            .expect("oriented L shape should fit a direction-specific Potrace template");
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
        .expect("oriented L path should render");

        assert!(matches!(segments.len(), 12..=14), "{segments:?}");
        assert!(compact_path_command_count(&data) <= 15, "{data}");
    }
}

#[test]
fn pixel_l_shape_template_accepts_variable_stroke_archetypes() {
    for (bitmap, expected_segments) in variable_l_shape_bitmaps().into_iter().zip([15, 20, 19]) {
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
        let segments = fit_closed_l_potrace_segments(&path.points)
            .expect("variable L shape should fit a Potrace-derived template");
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
        .expect("variable L path should render");

        assert_eq!(segments.len(), expected_segments, "{segments:?}");
        assert!(
            compact_path_command_count(&data) <= expected_segments,
            "{data}"
        );
    }
}

#[test]
fn pixel_t_shape_uses_potrace_template() {
    let bitmap = parity_t_shape_bitmap();
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
    assert!(svg.contains("M1060 1600l0-467"), "{svg}");
    assert!(svg.contains("s167 30 201 105"), "{svg}");
    assert!(svg.contains("c-37 17-78 19-595 19"), "{svg}");
}

#[test]
fn pixel_t_shape_template_accepts_mirrored_and_rotated_orientations() {
    for bitmap in [
        mirror_bitmap_x(&parity_t_shape_bitmap()),
        mirror_bitmap_y(&parity_t_shape_bitmap()),
        rotate_bitmap_clockwise(&parity_t_shape_bitmap()),
        rotate_bitmap_half_turn(&parity_t_shape_bitmap()),
        rotate_bitmap_counter_clockwise(&parity_t_shape_bitmap()),
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
        let segments = fit_closed_t_potrace_segments(&path.points)
            .expect("oriented T shape should fit a direction-specific Potrace template");
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
        .expect("oriented T path should render");

        assert_eq!(segments.len(), 16, "{segments:?}");
        assert!(compact_path_command_count(&data) <= 17, "{data}");
    }
}

#[test]
fn pixel_h_shape_uses_potrace_template() {
    let bitmap = parity_h_shape_bitmap();
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
    assert!(svg.contains("M1540 1060l0-197"), "{svg}");
    assert!(svg.contains("23 55 23 1239 0 1294"), "{svg}");
    assert!(svg.contains("l0-198-260 0-260 0 0 198"), "{svg}");
}

#[test]
fn pixel_h_shape_template_accepts_mirrored_and_rotated_orientations() {
    for bitmap in [
        mirror_bitmap_x(&parity_h_shape_bitmap()),
        mirror_bitmap_y(&parity_h_shape_bitmap()),
        rotate_bitmap_clockwise(&parity_h_shape_bitmap()),
        rotate_bitmap_half_turn(&parity_h_shape_bitmap()),
        rotate_bitmap_counter_clockwise(&parity_h_shape_bitmap()),
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
        let segments = fit_closed_h_potrace_segments(&path.points)
            .expect("oriented H shape should fit a direction-specific Potrace template");
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
        .expect("oriented H path should render");

        assert_eq!(segments.len(), 23, "{segments:?}");
        assert!(compact_path_command_count(&data) <= 23, "{data}");
    }
}

#[test]
fn pixel_hooked_l_shape_uses_potrace_template() {
    let bitmap = parity_hooked_l_bitmap();
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
    assert!(svg.contains("48 22 69 44 90 94"), "{svg}");
    assert!(svg.contains("0 305-4 334-48 384"), "{svg}");
    assert!(svg.contains("v-148Z"), "{svg}");
}

#[test]
fn pixel_hooked_l_shape_template_accepts_mirrored_and_rotated_orientations() {
    for bitmap in [
        mirror_bitmap_x(&parity_hooked_l_bitmap()),
        mirror_bitmap_y(&parity_hooked_l_bitmap()),
        rotate_bitmap_clockwise(&parity_hooked_l_bitmap()),
        rotate_bitmap_half_turn(&parity_hooked_l_bitmap()),
        rotate_bitmap_counter_clockwise(&parity_hooked_l_bitmap()),
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
        let segments = fit_closed_hooked_l_potrace_segments(&path.points)
            .expect("oriented hooked-L shape should fit a direction-specific Potrace template");
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
        .expect("oriented hooked-L path should render");

        assert!(matches!(segments.len(), 16..=18), "{segments:?}");
        assert!(compact_path_command_count(&data) <= 18, "{data}");
    }
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
fn pixel_chevron_template_accepts_mirrored_and_rotated_orientations() {
    for bitmap in [
        mirror_bitmap_y(&parity_chevron_bitmap()),
        rotate_bitmap_clockwise(&parity_chevron_bitmap()),
        rotate_bitmap_half_turn(&parity_chevron_bitmap()),
        rotate_bitmap_counter_clockwise(&parity_chevron_bitmap()),
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
        let segments = fit_closed_chevron_potrace_segments(&path.points)
            .expect("oriented chevron should fit a direction-specific Potrace template");
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
        .expect("oriented chevron path should render");

        assert!(matches!(segments.len(), 10 | 12), "{segments:?}");
        assert!(compact_path_command_count(&data) <= 14, "{data}");
    }
}
