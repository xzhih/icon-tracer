use super::*;

pub(crate) fn adjust_potrace_vertices(
    points: &[(f64, f64)],
    polygon: &[usize],
    max_vertex_adjustment: f64,
) -> Vec<(f64, f64)> {
    if polygon.len() < 3 {
        return polygon.iter().map(|index| points[*index]).collect();
    }

    let mut adjusted = Vec::with_capacity(polygon.len());

    for index in 0..polygon.len() {
        let previous = polygon[(index + polygon.len() - 1) % polygon.len()];
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        let incoming = best_fit_line_for_closed_arc(points, previous, current);
        let outgoing = best_fit_line_for_closed_arc(points, current, next);
        let vertex = line_intersection(incoming, outgoing)
            .map(|point| clamp_point_to_box(point, points[current], max_vertex_adjustment))
            .unwrap_or(points[current]);

        adjusted.push(vertex);
    }

    adjusted
}

pub(crate) fn adjust_potrace_vertices_quadratic(
    points: &[(f64, f64)],
    polygon: &[usize],
    max_vertex_adjustment: f64,
) -> Vec<(f64, f64)> {
    if polygon.len() < 3 {
        return polygon.iter().map(|index| points[*index]).collect();
    }

    let mut adjusted = Vec::with_capacity(polygon.len());

    for index in 0..polygon.len() {
        let previous = polygon[(index + polygon.len() - 1) % polygon.len()];
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        let incoming = best_fit_line_for_closed_arc(points, previous, current);
        let outgoing = best_fit_line_for_closed_arc(points, current, next);
        adjusted.push(adjusted_potrace_vertex_quadratic(
            incoming,
            outgoing,
            points[current],
            max_vertex_adjustment,
        ));
    }

    adjusted
}

fn clamp_point_to_box(point: (f64, f64), center: (f64, f64), radius: f64) -> (f64, f64) {
    (
        point.0.clamp(center.0 - radius, center.0 + radius),
        point.1.clamp(center.1 - radius, center.1 + radius),
    )
}

fn adjusted_potrace_vertex_quadratic(
    incoming: FitLine,
    outgoing: FitLine,
    center: (f64, f64),
    radius: f64,
) -> (f64, f64) {
    let mut form = line_distance_quadform(incoming);
    let outgoing_form = line_distance_quadform(outgoing);
    add_quadform(&mut form, outgoing_form);

    let optimum = loop {
        if let Some(point) = quadform_minimum(form) {
            break point;
        }

        let axis_form = orthogonal_axis_quadform(form, center);
        add_quadform(&mut form, axis_form);
    };

    if point_is_in_box(optimum, center, radius) {
        return optimum;
    }

    minimize_quadform_on_box(form, center, radius)
}

type QuadForm = [[f64; 3]; 3];

fn line_distance_quadform(line: FitLine) -> QuadForm {
    let d = vector_length_squared(line.direction);
    if d <= f64::EPSILON {
        return [[0.0; 3]; 3];
    }

    let v = [
        line.direction.1,
        -line.direction.0,
        line.direction.0 * line.point.1 - line.direction.1 * line.point.0,
    ];
    outer_product_quadform(v, d)
}

fn orthogonal_axis_quadform(form: QuadForm, center: (f64, f64)) -> QuadForm {
    let axis = if form[0][0] > form[1][1] {
        (-form[0][1], form[0][0])
    } else if form[1][1] != 0.0 {
        (-form[1][1], form[1][0])
    } else {
        (1.0, 0.0)
    };
    let d = axis.0 * axis.0 + axis.1 * axis.1;
    let v = [axis.0, axis.1, -axis.1 * center.1 - axis.0 * center.0];
    outer_product_quadform(v, d)
}

fn outer_product_quadform(v: [f64; 3], divisor: f64) -> QuadForm {
    let mut form = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            form[row][column] = v[row] * v[column] / divisor;
        }
    }
    form
}

fn add_quadform(left: &mut QuadForm, right: QuadForm) {
    for row in 0..3 {
        for column in 0..3 {
            left[row][column] += right[row][column];
        }
    }
}

fn quadform_minimum(form: QuadForm) -> Option<(f64, f64)> {
    let determinant = form[0][0] * form[1][1] - form[0][1] * form[1][0];
    if determinant.abs() <= f64::EPSILON {
        return None;
    }

    Some((
        (-form[0][2] * form[1][1] + form[1][2] * form[0][1]) / determinant,
        (form[0][2] * form[1][0] - form[1][2] * form[0][0]) / determinant,
    ))
}

fn point_is_in_box(point: (f64, f64), center: (f64, f64), radius: f64) -> bool {
    (point.0 - center.0).abs() <= radius && (point.1 - center.1).abs() <= radius
}

fn minimize_quadform_on_box(form: QuadForm, center: (f64, f64), radius: f64) -> (f64, f64) {
    let mut minimum_point = center;
    let mut minimum = quadform_value(form, center);

    if form[0][0] != 0.0 {
        for y in [center.1 - radius, center.1 + radius] {
            let x = -(form[0][1] * y + form[0][2]) / form[0][0];
            if (x - center.0).abs() <= radius {
                let point = (x, y);
                let value = quadform_value(form, point);
                if value < minimum {
                    minimum = value;
                    minimum_point = point;
                }
            }
        }
    }

    if form[1][1] != 0.0 {
        for x in [center.0 - radius, center.0 + radius] {
            let y = -(form[1][0] * x + form[1][2]) / form[1][1];
            if (y - center.1).abs() <= radius {
                let point = (x, y);
                let value = quadform_value(form, point);
                if value < minimum {
                    minimum = value;
                    minimum_point = point;
                }
            }
        }
    }

    for x in [center.0 - radius, center.0 + radius] {
        for y in [center.1 - radius, center.1 + radius] {
            let point = (x, y);
            let value = quadform_value(form, point);
            if value < minimum {
                minimum = value;
                minimum_point = point;
            }
        }
    }

    minimum_point
}

fn quadform_value(form: QuadForm, point: (f64, f64)) -> f64 {
    let v = [point.0, point.1, 1.0];
    let mut sum = 0.0;
    for row in 0..3 {
        for column in 0..3 {
            sum += v[row] * form[row][column] * v[column];
        }
    }
    sum
}
