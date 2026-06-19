use super::*;

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
