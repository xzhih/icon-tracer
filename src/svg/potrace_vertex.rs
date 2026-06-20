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

fn clamp_point_to_box(point: (f64, f64), center: (f64, f64), radius: f64) -> (f64, f64) {
    (
        point.0.clamp(center.0 - radius, center.0 + radius),
        point.1.clamp(center.1 - radius, center.1 + radius),
    )
}
