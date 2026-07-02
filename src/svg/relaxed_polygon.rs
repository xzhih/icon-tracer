use super::*;
use crate::TracePath;

type PathCandidate = ((f64, f64), Vec<SvgPathSegment>);

pub(crate) fn relaxed_quadrilateral_line_candidate(path: &TracePath) -> Option<PathCandidate> {
    const RELAXED_VERTEX_COUNT: usize = 5;
    const SIMPLIFIED_VERTEX_COUNT: usize = 4;
    const MAX_BOUNDARY_RMS_ERROR: f64 = 0.5;

    let relaxed = relaxed_optimal_potrace_polygon_indices(&path.points);
    if relaxed.len() != RELAXED_VERTEX_COUNT {
        return None;
    }

    let mut best: Option<(f64, PathCandidate)> = None;
    for skip in 0..relaxed.len() {
        let vertices = relaxed
            .iter()
            .enumerate()
            .filter_map(|(index, point_index)| (index != skip).then_some(path.points[*point_index]))
            .collect::<Vec<_>>();

        if vertices.len() != SIMPLIFIED_VERTEX_COUNT || !polygon_is_strictly_convex(&vertices) {
            continue;
        }

        let candidate = relaxed_quadrilateral_line_candidate_from_vertices(&vertices);
        let boundary_error = pixel_potrace_candidate_boundary_rms_error(path, &candidate);
        if boundary_error > MAX_BOUNDARY_RMS_ERROR {
            continue;
        }

        if best
            .as_ref()
            .is_none_or(|(best_error, _)| boundary_error < *best_error)
        {
            best = Some((boundary_error, candidate));
        }
    }

    best.map(|(_, candidate)| candidate)
}

pub(crate) fn relaxed_quadrilateral_curve_candidate(path: &TracePath) -> Option<PathCandidate> {
    const RELAXED_VERTEX_COUNT: usize = 5;
    const SIMPLIFIED_VERTEX_COUNT: usize = 4;
    const MAX_BOUNDARY_RMS_ERROR: f64 = 0.5;

    let relaxed = relaxed_optimal_potrace_polygon_indices(&path.points);
    if relaxed.len() != RELAXED_VERTEX_COUNT {
        return None;
    }

    let mut best: Option<(f64, PathCandidate)> = None;
    for skip in 0..relaxed.len() {
        let vertices = relaxed
            .iter()
            .enumerate()
            .filter_map(|(index, point_index)| (index != skip).then_some(path.points[*point_index]))
            .collect::<Vec<_>>();

        if vertices.len() != SIMPLIFIED_VERTEX_COUNT || !polygon_is_strictly_convex(&vertices) {
            continue;
        }

        let candidate = relaxed_quadrilateral_curve_candidate_from_vertices(&vertices);
        let boundary_error = pixel_potrace_candidate_boundary_rms_error(path, &candidate);
        if boundary_error > MAX_BOUNDARY_RMS_ERROR {
            continue;
        }

        if best
            .as_ref()
            .is_none_or(|(best_error, _)| boundary_error < *best_error)
        {
            best = Some((boundary_error, candidate));
        }
    }

    best.map(|(_, candidate)| candidate)
}

pub(crate) fn relaxed_quadrilateral_curve_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &PathCandidate,
    best: &PathCandidate,
) -> bool {
    const MIN_SPLIT_LINE_SEGMENTS: usize = 8;
    const MAX_EXTRA_MASK_PIXELS: usize = 4;
    const MAX_EXTRA_BOUNDARY_ERROR: f64 = 0.02;

    let Some((width, height)) = canvas_size else {
        return false;
    };
    if best.1.len() < MIN_SPLIT_LINE_SEGMENTS || candidate.1.len() != best.1.len() {
        return false;
    }
    if !best
        .1
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Line { .. }))
        || !candidate
            .1
            .iter()
            .any(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        return false;
    }

    let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
    let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);
    if candidate_error > best_error.saturating_add(MAX_EXTRA_MASK_PIXELS) {
        return false;
    }

    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, candidate);
    let best_boundary_error = pixel_potrace_candidate_boundary_rms_error(path, best);
    candidate_boundary_error <= best_boundary_error + MAX_EXTRA_BOUNDARY_ERROR
}

fn polygon_is_strictly_convex(vertices: &[(f64, f64)]) -> bool {
    const MIN_AREA: f64 = 16.0;
    const MIN_TURN_AREA: f64 = 1.0;

    if vertices.len() < 3 || polygon_area(vertices).abs() < MIN_AREA {
        return false;
    }

    let mut turn_sign = 0.0;
    for index in 0..vertices.len() {
        let previous = vertices[(index + vertices.len() - 1) % vertices.len()];
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];
        let turn = cross(subtract(current, previous), subtract(next, current));
        if turn.abs() < MIN_TURN_AREA {
            return false;
        }

        if turn_sign == 0.0 {
            turn_sign = turn.signum();
        } else if turn.signum() != turn_sign {
            return false;
        }
    }

    true
}

fn polygon_area(vertices: &[(f64, f64)]) -> f64 {
    vertices
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = vertices[(index + 1) % vertices.len()];
            point.0 * next.1 - next.0 * point.1
        })
        .sum::<f64>()
        / 2.0
}

fn relaxed_quadrilateral_line_candidate_from_vertices(vertices: &[(f64, f64)]) -> PathCandidate {
    let vertices = vertices_with_potrace_edge_splits(vertices);
    closed_line_candidate(&vertices)
}

fn relaxed_quadrilateral_curve_candidate_from_vertices(vertices: &[(f64, f64)]) -> PathCandidate {
    const CORNER_TRIM: f64 = 0.03;
    const CONTROL_PULL: f64 = 1.0;

    closed_rounded_corner_candidate(vertices, CORNER_TRIM, CONTROL_PULL)
}

fn vertices_with_potrace_edge_splits(vertices: &[(f64, f64)]) -> Vec<(f64, f64)> {
    const FIRST_SPLIT: f64 = 0.5;
    const SECOND_SPLIT: f64 = 0.985;
    const MIN_VERTICAL_DOMINANCE: f64 = 1.0;

    if vertices.len() != 4 {
        return vertices.to_vec();
    }

    let pair_02_vertical = edge_vertical_span(vertices, 0) + edge_vertical_span(vertices, 2);
    let pair_13_vertical = edge_vertical_span(vertices, 1) + edge_vertical_span(vertices, 3);
    let split_edges = if pair_13_vertical > pair_02_vertical + MIN_VERTICAL_DOMINANCE {
        [1, 3]
    } else if pair_02_vertical > pair_13_vertical + MIN_VERTICAL_DOMINANCE {
        [0, 2]
    } else {
        return vertices.to_vec();
    };

    let mut split_vertices = Vec::with_capacity(vertices.len() + 4);
    for index in 0..vertices.len() {
        let start = vertices[index];
        split_vertices.push(start);

        if split_edges.contains(&index) {
            let end = vertices[(index + 1) % vertices.len()];
            split_vertices.push(quantized_lerp_point(start, end, FIRST_SPLIT));
            split_vertices.push(quantized_lerp_point(start, end, SECOND_SPLIT));
        }
    }

    split_vertices
}

fn edge_vertical_span(vertices: &[(f64, f64)], index: usize) -> f64 {
    let next = vertices[(index + 1) % vertices.len()];
    (next.1 - vertices[index].1).abs()
}

fn quantized_lerp_point(start: (f64, f64), end: (f64, f64), t: f64) -> (f64, f64) {
    (
        quantize_to_tenth(start.0 + (end.0 - start.0) * t),
        quantize_to_tenth(start.1 + (end.1 - start.1) * t),
    )
}

fn quantize_to_tenth(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn closed_line_candidate(vertices: &[(f64, f64)]) -> PathCandidate {
    let start = vertices[0];
    let segments = (0..vertices.len())
        .map(|index| SvgPathSegment::Line {
            start: vertices[index],
            end: vertices[(index + 1) % vertices.len()],
        })
        .collect();
    (start, segments)
}

fn closed_rounded_corner_candidate(
    vertices: &[(f64, f64)],
    corner_trim: f64,
    control_pull: f64,
) -> PathCandidate {
    let pre_corners = (0..vertices.len())
        .map(|index| {
            interpolate(
                vertices[index],
                vertices[(index + vertices.len() - 1) % vertices.len()],
                corner_trim,
            )
        })
        .collect::<Vec<_>>();
    let post_corners = (0..vertices.len())
        .map(|index| {
            interpolate(
                vertices[index],
                vertices[(index + 1) % vertices.len()],
                corner_trim,
            )
        })
        .collect::<Vec<_>>();
    let start = post_corners[0];
    let mut segments = Vec::with_capacity(vertices.len() * 2);

    for index in 1..=vertices.len() {
        let current = index % vertices.len();
        let line_start = post_corners[(index - 1) % vertices.len()];
        let line_end = pre_corners[current];
        segments.push(SvgPathSegment::Line {
            start: line_start,
            end: line_end,
        });

        let vertex = vertices[current];
        let curve_end = post_corners[current];
        segments.push(SvgPathSegment::Cubic(CubicSegment {
            start: line_end,
            control1: interpolate(line_end, vertex, control_pull),
            control2: interpolate(curve_end, vertex, control_pull),
            end: curve_end,
        }));
    }

    (start, segments)
}
