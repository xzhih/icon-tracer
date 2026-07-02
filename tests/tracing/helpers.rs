use icon_tracer::{Bitmap, Rgba8};

pub(crate) fn rgba(red: u8, green: u8, blue: u8) -> Rgba8 {
    Rgba8 {
        red,
        green,
        blue,
        alpha: 255,
    }
}

pub(crate) fn circle_bitmap(
    width: usize,
    height: usize,
    center_x: f64,
    center_y: f64,
    radius: f64,
) -> Bitmap {
    let radius_squared = radius * radius;
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let sample_x = x as f64 + 0.5;
                let sample_y = y as f64 + 0.5;
                let dx = sample_x - center_x;
                let dy = sample_y - center_y;
                dx * dx + dy * dy <= radius_squared
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(width, height, &pixels).expect("bitmap dimensions should match")
}

pub(crate) fn open_arc_bitmap(
    width: usize,
    height: usize,
    center_x: f64,
    center_y: f64,
    inner_radius: f64,
    outer_radius: f64,
    gap_angle: f64,
) -> Bitmap {
    let inner_radius_squared = inner_radius * inner_radius;
    let outer_radius_squared = outer_radius * outer_radius;
    let mid_radius = (inner_radius + outer_radius) / 2.0;
    let cap_radius = (outer_radius - inner_radius) / 2.0;
    let cap_upper = (
        center_x + mid_radius * gap_angle.cos(),
        center_y - mid_radius * gap_angle.sin(),
    );
    let cap_lower = (
        center_x + mid_radius * gap_angle.cos(),
        center_y + mid_radius * gap_angle.sin(),
    );
    let cap_radius_squared = cap_radius * cap_radius;
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let sample_x = x as f64 + 0.5;
                let sample_y = y as f64 + 0.5;
                let dx = sample_x - center_x;
                let dy = sample_y - center_y;
                let radius_squared = dx * dx + dy * dy;
                let angle = dy.atan2(dx).abs();
                let in_arc = radius_squared >= inner_radius_squared
                    && radius_squared <= outer_radius_squared
                    && angle >= gap_angle;
                let upper_dx = sample_x - cap_upper.0;
                let upper_dy = sample_y - cap_upper.1;
                let lower_dx = sample_x - cap_lower.0;
                let lower_dy = sample_y - cap_lower.1;
                let in_upper_cap = upper_dx * upper_dx + upper_dy * upper_dy <= cap_radius_squared;
                let in_lower_cap = lower_dx * lower_dx + lower_dy * lower_dy <= cap_radius_squared;

                in_arc || in_upper_cap || in_lower_cap
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(width, height, &pixels).expect("bitmap dimensions should match")
}

pub(crate) fn count_cubic_segments(svg: &str) -> usize {
    svg.matches(" C ").count()
}

type Cubic = ((f64, f64), (f64, f64), (f64, f64), (f64, f64));

pub(crate) fn cubic_segments_from_svg(svg: &str) -> Vec<Cubic> {
    let path_data = svg
        .split_once(r#"d=""#)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(path_data, _)| path_data)
        .expect("SVG should contain path data");
    let tokens = path_data.split_whitespace().collect::<Vec<_>>();
    let mut segments = Vec::new();
    let mut index = 0;
    let mut current = (0.0, 0.0);

    while index < tokens.len() {
        match tokens[index] {
            "M" => {
                current = (
                    parse_svg_number(tokens[index + 1]),
                    parse_svg_number(tokens[index + 2]),
                );
                index += 3;
            }
            "C" => {
                let control1 = (
                    parse_svg_number(tokens[index + 1].trim_end_matches(',')),
                    parse_svg_number(tokens[index + 2].trim_end_matches(',')),
                );
                let control2 = (
                    parse_svg_number(tokens[index + 3].trim_end_matches(',')),
                    parse_svg_number(tokens[index + 4].trim_end_matches(',')),
                );
                let end = (
                    parse_svg_number(tokens[index + 5]),
                    parse_svg_number(tokens[index + 6]),
                );
                segments.push((current, control1, control2, end));
                current = end;
                index += 7;
            }
            "Z" => index += 1,
            token => panic!("unexpected SVG path token: {token}"),
        }
    }

    segments
}

fn parse_svg_number(token: &str) -> f64 {
    token.parse().expect("SVG coordinate should parse")
}

pub(crate) fn max_cubic_sample_distance_to_closed_path(
    segments: &[Cubic],
    path: &[(f64, f64)],
) -> f64 {
    let mut max_distance: f64 = 0.0;

    for cubic in segments {
        for sample in 0..=12 {
            let parameter = sample as f64 / 12.0;
            let point = cubic_point(*cubic, parameter);
            max_distance = max_distance.max(distance_to_closed_path(point, path));
        }
    }

    max_distance
}

fn cubic_point(cubic: Cubic, parameter: f64) -> (f64, f64) {
    let inverse = 1.0 - parameter;
    let b0 = inverse * inverse * inverse;
    let b1 = 3.0 * parameter * inverse * inverse;
    let b2 = 3.0 * parameter * parameter * inverse;
    let b3 = parameter * parameter * parameter;

    (
        cubic.0 .0 * b0 + cubic.1 .0 * b1 + cubic.2 .0 * b2 + cubic.3 .0 * b3,
        cubic.0 .1 * b0 + cubic.1 .1 * b1 + cubic.2 .1 * b2 + cubic.3 .1 * b3,
    )
}

fn distance_to_closed_path(point: (f64, f64), path: &[(f64, f64)]) -> f64 {
    path.iter()
        .zip(path.iter().cycle().skip(1))
        .map(|(start, end)| distance_to_segment(point, *start, *end))
        .fold(f64::INFINITY, f64::min)
}

fn distance_to_segment(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> f64 {
    let segment = (end.0 - start.0, end.1 - start.1);
    let length_squared = segment.0 * segment.0 + segment.1 * segment.1;

    if length_squared <= f64::EPSILON {
        return (point.0 - start.0).hypot(point.1 - start.1);
    }

    let projection =
        ((point.0 - start.0) * segment.0 + (point.1 - start.1) * segment.1) / length_squared;
    let projection = projection.clamp(0.0, 1.0);
    let closest = (
        start.0 + segment.0 * projection,
        start.1 + segment.1 * projection,
    );

    (point.0 - closest.0).hypot(point.1 - closest.1)
}
