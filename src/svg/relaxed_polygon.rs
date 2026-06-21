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

        let candidate = closed_line_candidate(&vertices);
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
