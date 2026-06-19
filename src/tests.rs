use super::*;

fn parity_triangle_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let top = (128.0, 42.0);
    let right = (214.0, 214.0);
    let left = (42.0, 214.0);
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                point_is_inside_triangle((x as f64 + 0.5, y as f64 + 0.5), top, right, left)
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn parity_ring_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let center = (128.0, 128.0);
    let outer_radius_squared = 78.0_f64 * 78.0;
    let inner_radius_squared = 42.0_f64 * 42.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let delta = (x as f64 + 0.5 - center.0, y as f64 + 0.5 - center.1);
                let distance_squared = delta.0 * delta.0 + delta.1 * delta.1;
                inner_radius_squared < distance_squared && distance_squared <= outer_radius_squared
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn parity_c_shape_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let center = (128.0, 128.0);
    let outer_radius_squared = 78.0_f64 * 78.0;
    let inner_radius_squared = 44.0_f64 * 44.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                let delta = (point.0 - center.0, point.1 - center.1);
                let distance_squared = delta.0 * delta.0 + delta.1 * delta.1;
                inner_radius_squared < distance_squared
                    && distance_squared <= outer_radius_squared
                    && !(point.0 > center.0 && delta.1.abs() < 34.0)
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn parity_f_shape_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    const RUNS: &[(usize, usize, usize, usize)] = &[
        (62, 62, 98, 181),
        (63, 63, 96, 183),
        (64, 64, 95, 184),
        (65, 65, 94, 185),
        (66, 67, 93, 186),
        (68, 87, 92, 187),
        (88, 89, 92, 186),
        (90, 90, 92, 185),
        (91, 91, 92, 184),
        (92, 92, 92, 183),
        (93, 93, 92, 181),
        (94, 94, 68, 125),
        (95, 95, 66, 125),
        (96, 96, 65, 125),
        (97, 97, 64, 125),
        (98, 99, 63, 125),
        (100, 119, 62, 125),
        (120, 121, 63, 125),
        (122, 122, 64, 125),
        (123, 123, 65, 125),
        (124, 124, 66, 125),
        (125, 125, 68, 125),
        (126, 126, 92, 181),
        (127, 127, 92, 183),
        (128, 128, 92, 184),
        (129, 129, 92, 185),
        (130, 131, 92, 186),
        (132, 151, 92, 187),
        (152, 153, 92, 186),
        (154, 154, 92, 185),
        (155, 155, 92, 184),
        (156, 156, 92, 183),
        (157, 157, 92, 181),
        (158, 158, 68, 125),
        (159, 159, 66, 125),
        (160, 160, 65, 125),
        (161, 161, 64, 125),
        (162, 163, 63, 125),
        (164, 187, 62, 125),
        (188, 189, 63, 124),
        (190, 190, 64, 123),
        (191, 191, 65, 122),
        (192, 192, 66, 121),
        (193, 193, 68, 119),
    ];
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                RUNS.iter().any(|(top, bottom, left, right)| {
                    (*top..=*bottom).contains(&y) && (*left..=*right).contains(&x)
                })
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn parity_two_circles_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let left_center = (84.0, 128.0);
    let right_center = (172.0, 128.0);
    let radius_squared = 42.0_f64 * 42.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                let left_delta = (point.0 - left_center.0, point.1 - left_center.1);
                let right_delta = (point.0 - right_center.0, point.1 - right_center.1);
                left_delta.0 * left_delta.0 + left_delta.1 * left_delta.1 <= radius_squared
                    || right_delta.0 * right_delta.0 + right_delta.1 * right_delta.1
                        <= radius_squared
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn parity_diagonal_bar_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let start = (62.0, 186.0);
    let end = (194.0, 70.0);
    let half_width = 18.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                distance_squared_to_segment((x as f64 + 0.5, y as f64 + 0.5), start, end)
                    .0
                    .sqrt()
                    <= half_width
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn parity_chevron_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let left = (70.0, 70.0);
    let bottom = (128.0, 186.0);
    let right = (186.0, 70.0);
    let half_width = 16.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                distance_squared_to_segment(point, left, bottom).0.sqrt() <= half_width
                    || distance_squared_to_segment(point, bottom, right).0.sqrt() <= half_width
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn parity_rounded_rect_bitmap(radius: f64) -> Bitmap {
    const CANVAS: usize = 256;
    let left = 54.0;
    let top = 62.0;
    let right = 202.0;
    let bottom = 194.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                let nearest_x = point.0.clamp(left + radius, right - radius);
                let nearest_y = point.1.clamp(top + radius, bottom - radius);
                (point.0 - nearest_x).powi(2) + (point.1 - nearest_y).powi(2) <= radius * radius
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn parity_u_shape_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let rects = [
        (54.0, 50.0, 96.0, 194.0, 17.0),
        (160.0, 50.0, 202.0, 194.0, 17.0),
        (54.0, 152.0, 202.0, 202.0, 20.0),
    ];
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                rects.iter().any(|(left, top, right, bottom, radius)| {
                    let nearest_x = point.0.clamp(left + radius, right - radius);
                    let nearest_y = point.1.clamp(top + radius, bottom - radius);
                    (point.0 - nearest_x).powi(2) + (point.1 - nearest_y).powi(2) <= radius * radius
                })
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn point_is_inside_triangle(
    point: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
) -> bool {
    fn sign(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> f64 {
        (p1.0 - p3.0) * (p2.1 - p3.1) - (p2.0 - p3.0) * (p1.1 - p3.1)
    }

    let d1 = sign(point, a, b);
    let d2 = sign(point, b, c);
    let d3 = sign(point, c, a);
    !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
}

#[test]
fn optimal_potrace_polygon_reduces_nearly_straight_stair_steps() {
    let mut points = (0..=12)
        .map(|x| (x as f64, if x % 2 == 0 { 0.0 } else { 0.2 }))
        .collect::<Vec<_>>();
    points.extend([(12.0, 6.0), (0.0, 6.0), (0.0, 0.0)]);

    let polygon = optimal_potrace_polygon_indices(&points);

    assert!(polygon.len() < points.len() / 2, "{polygon:?}");
}

#[test]
fn vertex_adjustment_moves_corner_toward_fitted_line_intersection() {
    let points = vec![
        (0.0, 0.0),
        (1.0, 0.0),
        (2.0, 0.2),
        (2.0, 1.0),
        (2.0, 2.0),
        (0.0, 2.0),
    ];
    let adjusted = adjust_potrace_vertices(&points, &[0, 2, 4, 5], 1.0);

    assert!(
        adjusted[1].1 < points[2].1,
        "corner did not move toward the fitted intersection: {adjusted:?}"
    );
}

#[test]
fn graph_opticurve_merges_compatible_adjacent_curves() {
    let run = vec![
        CubicSegment {
            start: (0.0, 0.0),
            control1: (0.33, 0.0),
            control2: (0.66, 0.0),
            end: (1.0, 0.0),
        },
        CubicSegment {
            start: (1.0, 0.0),
            control1: (1.33, 0.0),
            control2: (1.66, 0.0),
            end: (2.0, 0.0),
        },
    ];

    let optimized = optimize_potrace_curve_run_graph(&run, 0.2);

    assert_eq!(optimized.len(), 1, "{optimized:?}");
}

#[test]
fn compact_path_data_uses_relative_segments_when_shorter() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (10.0, 10.0),
            end: (15.0, 9.0),
        },
        SvgPathSegment::Cubic(CubicSegment {
            start: (15.0, 9.0),
            control1: (17.0, 9.0),
            control2: (19.0, 12.0),
            end: (21.0, 15.0),
        }),
    ];

    let data = compact_svg_path_data_from_segments((10.0, 10.0), &segments);

    assert_eq!(data, "M10 10l5-1c2 0 4 3 6 6Z");
}

#[test]
fn compact_path_data_keeps_absolute_segments_when_shorter() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (1000.0, 1000.0),
            end: (0.0, 0.0),
        },
        SvgPathSegment::Cubic(CubicSegment {
            start: (0.0, 0.0),
            control1: (0.0, 0.0),
            control2: (0.0, 0.0),
            end: (0.0, 0.0),
        }),
    ];

    let data = compact_svg_path_data_from_segments((1000.0, 1000.0), &segments);

    assert_eq!(data, "M1000 1000L0 0C0 0 0 0 0 0Z");
}

#[test]
fn compact_path_data_limits_fractional_precision() {
    let segments = vec![SvgPathSegment::Line {
        start: (10.12345, 20.98765),
        end: (11.55555, 22.44444),
    }];

    let data = compact_svg_path_data_from_segments((10.12345, 20.98765), &segments);

    assert_eq!(data, "M10.12 20.99l1.43 1.46Z");
}

#[test]
fn compact_path_data_omits_fractional_leading_zeroes() {
    let segments = vec![SvgPathSegment::Line {
        start: (0.25, -0.25),
        end: (0.75, -0.75),
    }];

    let data = compact_svg_path_data_from_segments((0.25, -0.25), &segments);

    assert_eq!(data, "M.25-.25l.5-.5Z");
}

#[test]
fn compact_path_data_omits_separator_before_fraction_after_decimal() {
    let segments = vec![SvgPathSegment::Line {
        start: (1.5, 0.25),
        end: (2.5, 0.75),
    }];

    let data = compact_svg_path_data_from_segments((1.5, 0.25), &segments);

    assert_eq!(data, "M1.5.25l1 .5Z");
}

#[test]
fn compact_path_data_uses_axis_line_shorthand() {
    let segments = vec![SvgPathSegment::Line {
        start: (0.0, 0.0),
        end: (10.0, 0.0),
    }];

    let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

    assert_eq!(data, "M0 0h10Z");
}

#[test]
fn compact_path_data_omits_redundant_closing_line() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (10.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (10.0, 0.0),
            end: (10.0, 10.0),
        },
        SvgPathSegment::Line {
            start: (10.0, 10.0),
            end: (0.0, 10.0),
        },
        SvgPathSegment::Line {
            start: (0.0, 10.0),
            end: (0.0, 0.0),
        },
    ];

    let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

    assert_eq!(data, "M0 0l10 0 0 10-10 0Z");
}

#[test]
fn compact_path_data_omits_collinear_line_before_close() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (10.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (10.0, 0.0),
            end: (10.0, 10.0),
        },
        SvgPathSegment::Line {
            start: (10.0, 10.0),
            end: (0.0, 10.0),
        },
        SvgPathSegment::Line {
            start: (0.0, 10.0),
            end: (0.0, 5.0),
        },
    ];

    let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

    assert_eq!(data, "M0 0l10 0 0 10-10 0Z");
}

#[test]
fn compact_path_data_rotates_closed_segments_to_shorter_start() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (1000.0, 1000.0),
            end: (0.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (1.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (1.0, 0.0),
            end: (1000.0, 1000.0),
        },
    ];

    let data = compact_svg_path_data_from_segments((1000.0, 1000.0), &segments);

    assert!(data.starts_with("M0 0"), "{data}");
}

#[test]
fn compact_path_data_uses_smooth_cubic_shorthand() {
    let segments = vec![
        SvgPathSegment::Cubic(CubicSegment {
            start: (0.0, 0.0),
            control1: (0.0, 10.0),
            control2: (10.0, 10.0),
            end: (10.0, 0.0),
        }),
        SvgPathSegment::Cubic(CubicSegment {
            start: (10.0, 0.0),
            control1: (10.0, -10.0),
            control2: (20.0, -10.0),
            end: (20.0, 0.0),
        }),
    ];

    let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

    assert_eq!(data, "M0 0c0 10 10 10 10 0s10-10 10 0Z");
}

#[test]
fn compact_path_data_uses_quadratic_for_tiny_cubic() {
    let segments = vec![SvgPathSegment::Cubic(CubicSegment {
        start: (0.0, 0.0),
        control1: (0.19, -0.75),
        control2: (1.81, -0.75),
        end: (2.0, 0.0),
    })];

    let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

    assert!(data.contains('q'), "{data}");
    assert!(!data.contains('c'), "{data}");
}

#[test]
fn compact_path_data_uses_arc_for_circle_primitive() {
    let segments = ellipse_arc_segments((128.0, 128.0), 76.0, 76.0, 5);
    let data = compact_svg_path_data_from_segments(segments[0].start(), &segments);

    assert!(data.contains('a'), "{data}");
    assert!(data.contains("75.85 75.9"), "{data}");
    assert!(data.len() <= 61, "{data}");
}

#[test]
fn compact_path_data_can_disable_arc_for_potrace_parity() {
    let segments = potrace_like_ellipse_segments((128.0, 128.0), 76.0, 76.0);
    let data = compact_svg_path_data_from_segments_without_arcs(segments[0].start(), &segments);

    assert!(data.contains('c'), "{data}");
    assert!(!data.contains('a'), "{data}");
}

#[test]
fn scaled_integer_path_data_preserves_numeric_separators() {
    let data = scaled_integer_svg_path_data(
        "M52 128c0-32.93 21.2-62.11 52.51-72.28s65.62.97 84.97 27.61Z",
        10.0,
    )
    .expect("compact path data should parse");

    assert!(data.contains("656 10"), "{data}");
    assert!(!data.contains("65610"), "{data}");
}

#[test]
fn pixel_potrace_path_element_uses_y_flipped_integer_path_when_shorter() {
    let diagonal = "M92.5 183c-11 9.62-21.5 18.18-23.32 19-6.29 2.84-15.93 1.27-19.72-3.2-4.94-5.83-6.24-13.59-3.43-20.53.84-2.07 23.11-22.67 49.5-45.77l67.98-59.5c11-9.62 21.5-18.17 23.32-19 6.29-2.84 15.93-1.27 19.72 3.2 4.94 5.83 6.24 13.59 3.43 20.53-.84 2.07-23.11 22.67-49.5 45.77l-67.98 59.5Z";
    let square = "M72 72l112 0 0 112-112 0 0-56 0-56Z";

    let diagonal_path = svg_path_element(diagonal, true, 256);
    let square_path = svg_path_element(square, true, 256);

    assert!(
        diagonal_path.contains("translate(0 256) scale(.1 -.1)"),
        "{diagonal_path}"
    );
    assert!(diagonal_path.contains("M925 730"), "{diagonal_path}");
    assert!(diagonal_path.contains("-96"), "{diagonal_path}");
    assert!(!square_path.contains("transform="), "{square_path}");
}

#[test]
fn one_decimal_path_element_uses_half_away_rounding_for_quadratics() {
    let triangle = "M84.5 129l42.5-85.25q1-1.12 2 0l42.5 85.25 42.5 84.75-86 .25-86-.25Z";

    let rounded = one_decimal_svg_path_data(triangle).expect("triangle path should round cleanly");
    let snapped = snap_near_integer_one_decimal_svg_path_data(&rounded)
        .expect("rounded triangle path should snap cleanly");

    assert!(rounded.contains("-85.3"), "{rounded}");
    assert!(rounded.contains("q1-1.1 2 0"), "{rounded}");
    assert!(snapped.contains("q1-1 2 0"), "{snapped}");
    assert!(!snapped.contains("q1-1.1"), "{snapped}");
}

#[test]
fn one_decimal_path_element_skips_arc_commands() {
    let circle = "M52.15 128a75.85 75.9 0 1 0 151.7 0a75.85 75.9 0 1 0-151.7 0Z";

    let path = svg_path_element(circle, true, 256);

    assert!(path.contains("75.85"), "{path}");
    assert!(!path.contains("75.9 75.9"), "{path}");
}

#[test]
fn pixel_potrace_candidate_selection_rejects_shorter_mask_regression() {
    let path = TracePath {
        is_hole: false,
        points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
    };
    let best = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (10.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 10.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 0.0),
            },
        ],
    );
    let shorter_wrong = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 0.0),
            },
        ],
    );

    assert!(pixel_potrace_candidate_is_better(
        &path,
        None,
        &shorter_wrong,
        &best
    ));
    assert!(!pixel_potrace_candidate_is_better(
        &path,
        Some((12, 12)),
        &shorter_wrong,
        &best
    ));
}

#[test]
fn fitted_candidate_selection_allows_tiny_mask_slack_only() {
    let path = TracePath {
        is_hole: false,
        points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
    };
    let best = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (10.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 10.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 0.0),
            },
        ],
    );
    let close = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Cubic(line_as_cubic((0.0, 0.0), (10.0, 0.0))),
            SvgPathSegment::Cubic(line_as_cubic((10.0, 0.0), (10.0, 10.0))),
            SvgPathSegment::Cubic(line_as_cubic((10.0, 10.0), (0.0, 10.0))),
            SvgPathSegment::Cubic(line_as_cubic((0.0, 10.0), (0.0, 0.0))),
        ],
    );
    let far = (
        (0.0, 0.0),
        vec![
            SvgPathSegment::Cubic(line_as_cubic((0.0, 0.0), (10.0, 0.0))),
            SvgPathSegment::Cubic(line_as_cubic((10.0, 0.0), (0.0, 10.0))),
            SvgPathSegment::Cubic(line_as_cubic((0.0, 10.0), (0.0, 0.0))),
        ],
    );

    assert!(pixel_potrace_fitted_candidate_is_close_enough(
        &path,
        Some((12, 12)),
        &close,
        &best
    ));
    assert!(!pixel_potrace_fitted_candidate_is_close_enough(
        &path,
        Some((12, 12)),
        &far,
        &best
    ));
}

#[test]
fn pixel_trace_can_preserve_collinear_boundary_points() {
    let bitmap =
        Bitmap::from_rows(3, 1, &[true, true, true]).expect("bitmap dimensions should match");
    let simplified = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 0,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: false,
        },
    );
    let preserved = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 0,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );

    assert_eq!(simplified.paths[0].points.len(), 4);
    assert_eq!(preserved.paths[0].points.len(), 8);
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
fn closed_capsule_potrace_fit_uses_six_cubics() {
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

    assert_eq!(segments.len(), 6);
    assert!(segments
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_))));
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
fn icon_candidate_selection_uses_global_fit_band() {
    let candidates = vec![
        test_icon_candidate(0.0, 10.0, 100, 100),
        test_icon_candidate(0.0015, 8.0, 80, 80),
        test_icon_candidate(0.003, 1.0, 10, 10),
    ];

    let best_index = best_icon_candidate_index(&candidates).expect("candidates should exist");

    assert_eq!(best_index, 1);
}

#[test]
fn potrace_segment_cleanup_removes_tiny_spike_between_long_curves() {
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (10.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((10.0, 0.0), (20.0, 0.0))),
        SvgPathSegment::Cubic(CubicSegment {
            start: (20.0, 0.0),
            control1: (19.9, 0.0),
            control2: (18.6, -0.9),
            end: (18.4, -1.2),
        }),
        SvgPathSegment::Cubic(test_cubic((18.4, -1.2), (30.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((30.0, 0.0), (40.0, 0.0))),
    ];

    let pruned = prune_tiny_potrace_curve_segments(segments);

    assert_eq!(pruned.len(), 4);
}

#[test]
fn potrace_segment_cleanup_removes_tiny_spike_at_closed_start() {
    let segments = vec![
        SvgPathSegment::Cubic(CubicSegment {
            start: (0.0, 0.0),
            control1: (0.0, -0.4),
            control2: (0.0, -1.2),
            end: (0.0, -1.8),
        }),
        SvgPathSegment::Cubic(test_cubic((0.0, -1.8), (12.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((12.0, 0.0), (12.0, 12.0))),
        SvgPathSegment::Cubic(test_cubic((12.0, 12.0), (-12.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((-12.0, 0.0), (0.0, 0.0))),
    ];

    let pruned = prune_tiny_potrace_curve_segments(segments.clone());
    let cleaned = cleanup_potrace_segments(segments, PIXEL_POTRACE_LINEAR_DEVIATION);
    let start = cleanup_potrace_start((0.0, 0.0), &cleaned);

    assert_eq!(pruned.len(), 4);
    assert_eq!(pruned[0].start(), (0.0, -1.8));
    assert_eq!(start, cleaned[0].start());
}

#[test]
fn potrace_segment_cleanup_snaps_near_axis_lines_continuously() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (10.0, 0.4),
        },
        SvgPathSegment::Cubic(line_as_cubic((10.0, 0.4), (10.0, 10.0))),
        SvgPathSegment::Line {
            start: (10.0, 10.0),
            end: (0.0, 9.8),
        },
        SvgPathSegment::Cubic(line_as_cubic((0.0, 9.8), (0.0, 0.0))),
    ];

    let snapped = snap_near_axis_potrace_lines(segments);

    assert_eq!(snapped[0].start(), (0.0, 0.2));
    assert_eq!(snapped[0].end(), (10.0, 0.2));
    assert_eq!(snapped[1].start(), (10.0, 0.2));
    assert_eq!(snapped[1].end(), (10.0, 9.9));
    assert_eq!(snapped[2].start(), (10.0, 9.9));
    assert_eq!(snapped[2].end(), (0.0, 9.9));
    assert_eq!(snapped[3].start(), (0.0, 9.9));
}

#[test]
fn potrace_segment_cleanup_demotes_nearly_linear_cubics() {
    let segments = [
        SvgPathSegment::Cubic(CubicSegment {
            start: (0.0, 0.0),
            control1: (33.0, 0.8),
            control2: (66.0, -0.8),
            end: (100.0, 0.0),
        }),
        SvgPathSegment::Cubic(CubicSegment {
            start: (100.0, 0.0),
            control1: (100.0, 40.0),
            control2: (140.0, 40.0),
            end: (140.0, 0.0),
        }),
    ];

    let strict_cleaned =
        demote_nearly_linear_potrace_cubics(segments.to_vec(), STRICT_POTRACE_LINEAR_DEVIATION);
    let pixel_cleaned =
        demote_nearly_linear_potrace_cubics(segments.to_vec(), PIXEL_POTRACE_LINEAR_DEVIATION);

    assert!(matches!(strict_cleaned[0], SvgPathSegment::Cubic(_)));
    assert!(matches!(pixel_cleaned[0], SvgPathSegment::Line { .. }));
    assert!(matches!(pixel_cleaned[1], SvgPathSegment::Cubic(_)));
}

#[test]
fn potrace_segment_cleanup_merges_adjacent_collinear_lines() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (10.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (10.0, 0.0),
            end: (20.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (20.0, 0.0),
            end: (20.0, 10.0),
        },
    ];

    let merged = merge_collinear_potrace_lines(segments);

    assert_eq!(merged.len(), 2, "{merged:?}");
    assert!(matches!(
        merged[0],
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (20.0, 0.0)
        }
    ));
}

#[test]
fn potrace_segment_cleanup_keeps_reversing_collinear_lines() {
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (10.0, 0.0),
        },
        SvgPathSegment::Line {
            start: (10.0, 0.0),
            end: (0.0, 0.0),
        },
    ];

    let merged = merge_collinear_potrace_lines(segments);

    assert_eq!(merged.len(), 2, "{merged:?}");
}

#[test]
fn potrace_segment_cleanup_reruns_curve_optimization_after_linear_demotion() {
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (1.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((1.0, 0.0), (2.0, 0.0))),
        SvgPathSegment::Cubic(line_as_cubic((2.0, 0.0), (30.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((30.0, 0.0), (31.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((31.0, 0.0), (32.0, 0.0))),
        SvgPathSegment::Cubic(line_as_cubic((32.0, 0.0), (0.0, 0.0))),
    ];

    let (_, optimized) =
        finish_potrace_segments((0.0, 0.0), segments, 0.2, STRICT_POTRACE_LINEAR_DEVIATION);
    let cubic_count = optimized
        .iter()
        .filter(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
        .count();
    let line_count = optimized
        .iter()
        .filter(|segment| matches!(segment, SvgPathSegment::Line { .. }))
        .count();

    assert_eq!(cubic_count, 2, "{optimized:?}");
    assert_eq!(line_count, 2, "{optimized:?}");
}

#[test]
fn bezier_tangent_parameter_handles_linear_degenerate_case() {
    let cubic = CubicSegment {
        start: (0.0, 0.0),
        control1: (1.0, 1.0),
        control2: (2.0, 1.0),
        end: (3.0, 0.0),
    };

    let parameter = bezier_tangent_parameter(cubic, (0.0, 0.0), (1.0, 0.0))
        .expect("linear tangent equation should have an in-range solution");

    assert!((parameter - 0.5).abs() <= 1.0e-9);
}

#[test]
fn regularize_potrace_orthogonal_corner_uses_tangent_controls() {
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (100.0, 0.0))),
        SvgPathSegment::Cubic(CubicSegment {
            start: (100.0, 0.0),
            control1: (104.0, 0.2),
            control2: (109.8, 5.5),
            end: (110.0, 10.0),
        }),
        SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (40.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((40.0, 90.0), (0.0, 0.0))),
    ];

    let regularized = regularize_potrace_orthogonal_corners(segments);
    let SvgPathSegment::Cubic(corner) = regularized[1] else {
        panic!("corner should remain cubic: {regularized:?}");
    };

    assert_eq!(regularized.len(), 5);
    assert!(
        (corner.control1.1 - corner.start.1).abs() <= 1.0e-6,
        "{corner:?}"
    );
    assert!(
        (corner.control2.0 - corner.end.0).abs() <= 1.0e-6,
        "{corner:?}"
    );
}

#[test]
fn regularize_potrace_orthogonal_corner_merges_straight_lead_in() {
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (100.0, 0.0))),
        SvgPathSegment::Cubic(test_cubic((100.0, 0.0), (120.0, 0.0))),
        SvgPathSegment::Cubic(CubicSegment {
            start: (120.0, 0.0),
            control1: (124.0, 0.5),
            control2: (130.0, 6.0),
            end: (130.0, 12.0),
        }),
        SvgPathSegment::Cubic(test_cubic((130.0, 12.0), (130.0, 92.0))),
        SvgPathSegment::Cubic(test_cubic((130.0, 92.0), (0.0, 92.0))),
    ];

    let regularized = regularize_potrace_orthogonal_corners(segments);
    let SvgPathSegment::Cubic(corner) = regularized[1] else {
        panic!("merged corner should be cubic: {regularized:?}");
    };

    assert_eq!(regularized.len(), 4);
    assert_eq!(corner.start, (100.0, 0.0));
    assert_eq!(corner.end, (130.0, 12.0));
}

#[test]
fn regularize_potrace_orthogonal_corner_rejects_beveled_turn() {
    let bevel = test_cubic((100.0, 0.0), (110.0, 10.0));
    let segments = vec![
        SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (100.0, 0.0))),
        SvgPathSegment::Cubic(bevel),
        SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (40.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((40.0, 90.0), (0.0, 0.0))),
    ];

    let regularized = regularize_potrace_orthogonal_corners(segments);
    let SvgPathSegment::Cubic(unchanged) = regularized[1] else {
        panic!("bevel should remain cubic: {regularized:?}");
    };

    assert_eq!(regularized.len(), 5);
    assert_eq!(unchanged.control1, bevel.control1);
    assert_eq!(unchanged.control2, bevel.control2);
}

#[test]
fn regularize_potrace_orthogonal_corner_ignores_mixed_line_boundaries() {
    let corner = CubicSegment {
        start: (100.0, 0.0),
        control1: (104.0, 0.2),
        control2: (109.8, 5.5),
        end: (110.0, 10.0),
    };
    let segments = vec![
        SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (100.0, 0.0),
        },
        SvgPathSegment::Cubic(corner),
        SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (40.0, 90.0))),
        SvgPathSegment::Cubic(test_cubic((40.0, 90.0), (0.0, 0.0))),
    ];

    let regularized = regularize_potrace_orthogonal_corners(segments);
    let SvgPathSegment::Cubic(unchanged) = regularized[1] else {
        panic!("mixed line boundary should keep the corner cubic: {regularized:?}");
    };

    assert_eq!(regularized.len(), 5);
    assert_eq!(unchanged.control1, corner.control1);
    assert_eq!(unchanged.control2, corner.control2);
}

fn test_cubic(start: (f64, f64), end: (f64, f64)) -> CubicSegment {
    CubicSegment {
        start,
        control1: (
            start.0 + (end.0 - start.0) / 3.0,
            start.1 + (end.1 - start.1) / 3.0,
        ),
        control2: (
            start.0 + (end.0 - start.0) * 2.0 / 3.0,
            start.1 + (end.1 - start.1) * 2.0 / 3.0,
        ),
        end,
    }
}

fn test_icon_candidate(
    foreground_error_ratio: f64,
    score: f64,
    point_count: usize,
    svg_command_count: usize,
) -> IconOptimizationCandidate {
    IconOptimizationCandidate {
        trace_options: TraceOptions::default(),
        metrics: IconDiffMetrics {
            total_pixels: 1000,
            target_foreground_pixels: 1000,
            candidate_foreground_pixels: 1000,
            true_positive_pixels: 1000,
            false_positive_pixels: 0,
            false_negative_pixels: 0,
            xor_pixels: 0,
            xor_ratio: foreground_error_ratio,
            foreground_error_ratio,
            false_positive_ratio: 0.0,
            false_negative_ratio: 0.0,
            precision: 1.0,
            recall: 1.0,
            iou: 1.0,
        },
        score,
        path_count: 1,
        point_count,
        svg_command_count,
    }
}
