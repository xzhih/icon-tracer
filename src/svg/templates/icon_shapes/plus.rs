use super::*;

pub(crate) fn fit_closed_plus_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    const MIN_AXIS: f64 = 48.0;
    const MIN_ASPECT_RATIO: f64 = 0.75;
    const MAX_ASPECT_RATIO: f64 = 1.25;
    const MAX_TEMPLATE_BOUNDARY_ERROR: f64 = 3.0;

    let bounds = FloatBounds::from_points(points)?;
    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;
    if width < MIN_AXIS || height < MIN_AXIS {
        return None;
    }

    let aspect = width / height;
    if !(MIN_ASPECT_RATIO..=MAX_ASPECT_RATIO).contains(&aspect) {
        return None;
    }

    let segments = plus_potrace_segments(bounds);
    let candidate = (segments[0].start(), segments);
    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(
        &TracePath {
            points: points.to_vec(),
            is_hole: false,
        },
        &candidate,
    );

    (candidate_boundary_error <= MAX_TEMPLATE_BOUNDARY_ERROR).then_some(candidate.1)
}

pub(crate) fn plus_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    vec![
        normalized_rect_cubic(
            bounds,
            (0.428_125, 0.011_875),
            (0.369_375, 0.038_75),
            (0.362_5, 0.060_625),
            (0.362_5, 0.228_125),
        ),
        normalized_rect_line(bounds, (0.362_5, 0.228_125), (0.362_5, 0.362_5)),
        normalized_rect_line(bounds, (0.362_5, 0.362_5), (0.228_125, 0.362_5)),
        normalized_rect_cubic(
            bounds,
            (0.228_125, 0.362_5),
            (0.087_5, 0.362_5),
            (0.065, 0.366_25),
            (0.035, 0.392_5),
        ),
        normalized_rect_cubic(
            bounds,
            (0.035, 0.392_5),
            (-0.011_875, 0.434_375),
            (-0.011_875, 0.565_625),
            (0.035, 0.607_5),
        ),
        normalized_rect_cubic(
            bounds,
            (0.035, 0.607_5),
            (0.065, 0.633_75),
            (0.087_5, 0.637_5),
            (0.228_125, 0.637_5),
        ),
        normalized_rect_line(bounds, (0.228_125, 0.637_5), (0.362_5, 0.637_5)),
        normalized_rect_line(bounds, (0.362_5, 0.637_5), (0.362_5, 0.773_125)),
        normalized_rect_cubic(
            bounds,
            (0.362_5, 0.773_125),
            (0.362_5, 0.891_875),
            (0.364_375, 0.912_5),
            (0.374_375, 0.934_375),
        ),
        normalized_rect_cubic(
            bounds,
            (0.374_375, 0.934_375),
            (0.395_625, 0.981_25),
            (0.431_875, 1.0),
            (0.5, 1.0),
        ),
        normalized_rect_cubic(
            bounds,
            (0.5, 1.0),
            (0.568_125, 1.0),
            (0.604_375, 0.981_25),
            (0.625_625, 0.934_375),
        ),
        normalized_rect_cubic(
            bounds,
            (0.625_625, 0.934_375),
            (0.635_625, 0.912_5),
            (0.637_5, 0.891_875),
            (0.637_5, 0.773_125),
        ),
        normalized_rect_line(bounds, (0.637_5, 0.773_125), (0.637_5, 0.637_5)),
        normalized_rect_line(bounds, (0.637_5, 0.637_5), (0.771_875, 0.637_5)),
        normalized_rect_cubic(
            bounds,
            (0.771_875, 0.637_5),
            (0.912_5, 0.637_5),
            (0.935, 0.633_75),
            (0.965, 0.607_5),
        ),
        normalized_rect_cubic(
            bounds,
            (0.965, 0.607_5),
            (1.011_875, 0.565_625),
            (1.011_875, 0.434_375),
            (0.965, 0.392_5),
        ),
        normalized_rect_cubic(
            bounds,
            (0.965, 0.392_5),
            (0.935, 0.366_25),
            (0.912_5, 0.362_5),
            (0.771_875, 0.362_5),
        ),
        normalized_rect_line(bounds, (0.771_875, 0.362_5), (0.637_5, 0.362_5)),
        normalized_rect_line(bounds, (0.637_5, 0.362_5), (0.637_5, 0.226_25)),
        normalized_rect_cubic(
            bounds,
            (0.637_5, 0.226_25),
            (0.637_5, 0.057_5),
            (0.63, 0.036_25),
            (0.566_875, 0.009_375),
        ),
        normalized_rect_cubic(
            bounds,
            (0.566_875, 0.009_375),
            (0.534_375, -0.003_75),
            (0.46, -0.003_125),
            (0.428_125, 0.011_875),
        ),
    ]
}
