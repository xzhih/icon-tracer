use super::*;

pub(crate) fn cubic_point(cubic: CubicSegment, parameter: f64) -> (f64, f64) {
    let (b0, b1, b2, b3) = bernstein3(parameter);

    add(
        add(scale(cubic.start, b0), scale(cubic.control1, b1)),
        add(scale(cubic.control2, b2), scale(cubic.end, b3)),
    )
}

pub(crate) fn cubic_derivative_point(cubic: CubicSegment, parameter: f64) -> (f64, f64) {
    let inverse = 1.0 - parameter;
    let b0 = 3.0 * inverse * inverse;
    let b1 = 6.0 * inverse * parameter;
    let b2 = 3.0 * parameter * parameter;

    add(
        add(
            scale(subtract(cubic.control1, cubic.start), b0),
            scale(subtract(cubic.control2, cubic.control1), b1),
        ),
        scale(subtract(cubic.end, cubic.control2), b2),
    )
}

pub(crate) fn cubic_second_derivative_point(cubic: CubicSegment, parameter: f64) -> (f64, f64) {
    let inverse = 1.0 - parameter;
    let b0 = 6.0 * inverse;
    let b1 = 6.0 * parameter;

    add(
        scale(
            add(
                subtract(cubic.control2, scale(cubic.control1, 2.0)),
                cubic.start,
            ),
            b0,
        ),
        scale(
            add(
                subtract(cubic.end, scale(cubic.control2, 2.0)),
                cubic.control1,
            ),
            b1,
        ),
    )
}

pub(crate) fn bernstein3(parameter: f64) -> (f64, f64, f64, f64) {
    let inverse = 1.0 - parameter;

    (
        inverse * inverse * inverse,
        3.0 * parameter * inverse * inverse,
        3.0 * parameter * parameter * inverse,
        parameter * parameter * parameter,
    )
}

pub(crate) fn add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

pub(crate) fn subtract(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 - b.0, a.1 - b.1)
}

pub(crate) fn scale(vector: (f64, f64), scalar: f64) -> (f64, f64) {
    (vector.0 * scalar, vector.1 * scalar)
}

pub(crate) fn dot(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

pub(crate) fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

pub(crate) fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    distance_squared_float(a, b).sqrt()
}

pub(crate) fn distance_squared_float(a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;

    dx * dx + dy * dy
}

pub(crate) fn vector_length_squared(vector: (f64, f64)) -> f64 {
    vector.0 * vector.0 + vector.1 * vector.1
}

pub(crate) fn unit_vector(vector: (f64, f64)) -> (f64, f64) {
    let length = vector.0.hypot(vector.1);

    if length <= f64::EPSILON {
        (0.0, 0.0)
    } else {
        (vector.0 / length, vector.1 / length)
    }
}

pub(crate) fn catmull_rom_segment(
    path: &TracePath,
    index: usize,
) -> ((f64, f64), (f64, f64), (f64, f64)) {
    let previous = path.points[(index + path.points.len() - 1) % path.points.len()];
    let current = path.points[index];
    let next = path.points[(index + 1) % path.points.len()];
    let after_next = path.points[(index + 2) % path.points.len()];

    let control1 = (
        current.0 + (next.0 - previous.0) / 6.0,
        current.1 + (next.1 - previous.1) / 6.0,
    );
    let control2 = (
        next.0 - (after_next.0 - current.0) / 6.0,
        next.1 - (after_next.1 - current.1) / 6.0,
    );

    (control1, control2, next)
}

pub(crate) fn corner_entry(points: &[(f64, f64)], index: usize, amount: f64) -> (f64, f64) {
    let previous = points[(index + points.len() - 1) % points.len()];
    interpolate(points[index], previous, amount)
}

pub(crate) fn corner_exit(points: &[(f64, f64)], index: usize, amount: f64) -> (f64, f64) {
    let next = points[(index + 1) % points.len()];
    interpolate(points[index], next, amount)
}

pub(crate) fn interpolate(from: (f64, f64), to: (f64, f64), amount: f64) -> (f64, f64) {
    (
        from.0 + (to.0 - from.0) * amount,
        from.1 + (to.1 - from.1) * amount,
    )
}

pub(crate) fn cubic_control_point(endpoint: (f64, f64), corner: (f64, f64)) -> (f64, f64) {
    (
        endpoint.0 + (corner.0 - endpoint.0) * 2.0 / 3.0,
        endpoint.1 + (corner.1 - endpoint.1) * 2.0 / 3.0,
    )
}

pub(crate) fn format_float(value: f64) -> String {
    format_float_with_precision(value, 6)
}

pub(crate) fn format_compact_float(value: f64) -> String {
    format_compact_float_with_precision(value, 2)
}

pub(crate) fn format_compact_float_with_precision(value: f64, precision: usize) -> String {
    let mut formatted = format_float_with_precision(value, precision);
    if formatted.starts_with("0.") {
        formatted.remove(0);
    } else if formatted.starts_with("-0.") {
        formatted.remove(1);
    }
    formatted
}

pub(crate) fn format_float_with_precision(value: f64, precision: usize) -> String {
    let epsilon = 0.5 * 10.0_f64.powi(-(precision as i32));
    let value = if value.abs() < epsilon { 0.0 } else { value };
    let mut formatted = format!("{value:.precision$}");

    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }

    if formatted.ends_with('.') {
        formatted.pop();
    }

    formatted
}
