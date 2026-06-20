use super::*;

const STRICT_POLYGON_MAX_DISTANCE: f64 = 1.0;
const RELAXED_POLYGON_MAX_DISTANCE: f64 = 2.0;
type PotraceAlphaFn = fn((f64, f64), (f64, f64), (f64, f64)) -> f64;

pub(crate) fn optimal_potrace_polygon_indices(points: &[(f64, f64)]) -> Vec<usize> {
    optimal_potrace_polygon_indices_with_max_distance(points, STRICT_POLYGON_MAX_DISTANCE)
}

pub(crate) fn relaxed_optimal_potrace_polygon_indices(points: &[(f64, f64)]) -> Vec<usize> {
    optimal_potrace_polygon_indices_with_max_distance(points, RELAXED_POLYGON_MAX_DISTANCE)
}

pub(crate) fn optimal_potrace_polygon_indices_with_max_distance(
    points: &[(f64, f64)],
    max_distance: f64,
) -> Vec<usize> {
    if points.len() > 3 && distance_squared_float(points[0], points[points.len() - 1]) <= 1.0e-12 {
        return optimal_potrace_polygon_indices_with_max_distance(
            &points[..points.len() - 1],
            max_distance,
        );
    }

    if points.len() <= 8 {
        return (0..points.len()).collect();
    }

    if !points_are_half_pixel_quantized(points) {
        return legacy_potrace_polygon_indices(points);
    }

    let mut best: Option<PolygonCandidate> = None;
    for rotation in polygon_rotation_candidates(points) {
        let rotated = rotate_float_points(points, rotation);
        let Some(candidate) =
            best_polygon_for_rotated_points_with_max_distance(&rotated, max_distance)
        else {
            continue;
        };
        let indices = candidate
            .indices
            .iter()
            .map(|index| (index + rotation) % points.len())
            .collect::<Vec<_>>();
        let candidate = PolygonCandidate {
            indices,
            segments: candidate.segments,
            penalty: candidate.penalty,
        };

        if best
            .as_ref()
            .is_none_or(|current| polygon_candidate_is_better(&candidate, current))
        {
            best = Some(candidate);
        }
    }

    best.map(|candidate| candidate.indices)
        .filter(|indices| indices.len() >= 3)
        .unwrap_or_else(|| (0..points.len()).collect())
}

pub(crate) fn legacy_potrace_polygon_indices(points: &[(f64, f64)]) -> Vec<usize> {
    const POLYGON_TOLERANCE: f64 = 0.75;

    let n = points.len();
    let mut dp: Vec<Option<PolygonDpState>> = vec![None; n + 1];
    dp[0] = Some(PolygonDpState {
        previous: 0,
        segments: 0,
        penalty: 0.0,
    });

    for start in 0..n {
        let Some(state) = dp[start] else {
            continue;
        };

        let mut end = start + 1;
        while end <= n {
            if end == n && state.segments < 2 {
                end += 1;
                continue;
            }

            if !legacy_potrace_arc_is_straight(points, start, end, POLYGON_TOLERANCE) {
                if end == start + 1 {
                    end += 1;
                    continue;
                }
                break;
            }

            let penalty =
                state.penalty + legacy_potrace_polygon_segment_penalty(points, start, end);
            let candidate = PolygonDpState {
                previous: start,
                segments: state.segments + 1,
                penalty,
            };

            if dp[end].is_none_or(|best| polygon_dp_state_is_better(candidate, best)) {
                dp[end] = Some(candidate);
            }

            end += 1;
        }
    }

    let Some(_) = dp[n] else {
        return (0..points.len()).collect();
    };

    let mut indices = Vec::new();
    let mut cursor = n;

    while cursor != 0 {
        let state = dp[cursor].expect("legacy dp cursor should be reachable");
        indices.push(state.previous % n);
        cursor = state.previous;
    }

    indices.reverse();
    indices.dedup();

    if indices.len() < 3 {
        (0..points.len()).collect()
    } else {
        indices
    }
}

pub(crate) fn legacy_potrace_arc_is_straight(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
    tolerance: f64,
) -> bool {
    if end <= start + 1 {
        return true;
    }

    let start_point = closed_point(points, start);
    let end_point = closed_point(points, end);
    let tolerance_squared = tolerance * tolerance;

    for index in start + 1..end {
        let point = closed_point(points, index);
        if distance_squared_to_segment(point, start_point, end_point).0 > tolerance_squared {
            return false;
        }
    }

    true
}

pub(crate) fn legacy_potrace_polygon_segment_penalty(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
) -> f64 {
    if end <= start + 1 {
        return 0.0;
    }

    let start_point = closed_point(points, start);
    let end_point = closed_point(points, end);

    (start + 1..end)
        .map(|index| {
            distance_squared_to_segment(closed_point(points, index), start_point, end_point).0
        })
        .sum()
}

#[derive(Debug, Clone)]
pub(crate) struct PolygonCandidate {
    indices: Vec<usize>,
    segments: usize,
    penalty: f64,
}

pub(crate) fn polygon_candidate_is_better(
    candidate: &PolygonCandidate,
    best: &PolygonCandidate,
) -> bool {
    candidate.segments < best.segments
        || (candidate.segments == best.segments && candidate.penalty < best.penalty)
}

pub(crate) fn polygon_rotation_candidates(points: &[(f64, f64)]) -> Vec<usize> {
    const MAX_ROTATIONS: usize = 24;

    if points.len() <= MAX_ROTATIONS {
        return (0..points.len()).collect();
    }

    let mut scored = (0..points.len())
        .map(|index| {
            let previous = points[(index + points.len() - 1) % points.len()];
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            let turn = vector_turn_angle(subtract(current, previous), subtract(next, current));
            (index, turn)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut candidates = Vec::new();
    candidates.push(0);
    for (index, turn) in scored {
        if turn <= 1.0e-6 {
            continue;
        }

        if !candidates.contains(&index) {
            candidates.push(index);
        }

        if candidates.len() >= MAX_ROTATIONS {
            break;
        }
    }

    let stride = (points.len() / MAX_ROTATIONS).max(1);
    for index in (0..points.len()).step_by(stride) {
        if candidates.len() >= MAX_ROTATIONS {
            break;
        }
        if !candidates.contains(&index) {
            candidates.push(index);
        }
    }

    candidates
}

pub(crate) fn rotate_float_points(points: &[(f64, f64)], start_index: usize) -> Vec<(f64, f64)> {
    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

pub(crate) fn best_polygon_for_rotated_points_with_max_distance(
    points: &[(f64, f64)],
    max_distance: f64,
) -> Option<PolygonCandidate> {
    let n = points.len();
    let sums = PathSums::for_closed_points(points);
    let mut dp: Vec<Option<PolygonDpState>> = vec![None; n + 1];
    dp[0] = Some(PolygonDpState {
        previous: 0,
        segments: 0,
        penalty: 0.0,
    });

    for start in 0..n {
        let Some(state) = dp[start] else {
            continue;
        };

        let mut end = start + 1;
        while end <= n {
            if end - start > n.saturating_sub(3) {
                break;
            }

            if !potrace_possible_segment_is_straight(points, start, end, max_distance) {
                if end == start + 1 {
                    end += 1;
                    continue;
                }
                break;
            }

            let penalty =
                state.penalty + potrace_polygon_segment_penalty(points, &sums, start, end);
            let candidate = PolygonDpState {
                previous: start,
                segments: state.segments + 1,
                penalty,
            };

            if dp[end].is_none_or(|best| polygon_dp_state_is_better(candidate, best)) {
                dp[end] = Some(candidate);
            }

            end += 1;
        }
    }

    let final_state = dp[n]?;

    let mut indices = Vec::new();
    let mut cursor = n;

    while cursor != 0 {
        let state = dp[cursor].expect("dp cursor should be reachable");
        indices.push(state.previous % n);
        cursor = state.previous;
    }

    indices.reverse();
    indices.dedup();

    if indices.len() < 3 {
        None
    } else {
        Some(PolygonCandidate {
            indices,
            segments: final_state.segments,
            penalty: final_state.penalty,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PolygonDpState {
    previous: usize,
    segments: usize,
    penalty: f64,
}

pub(crate) fn polygon_dp_state_is_better(candidate: PolygonDpState, best: PolygonDpState) -> bool {
    candidate.segments < best.segments
        || (candidate.segments == best.segments && candidate.penalty < best.penalty)
}

#[derive(Debug, Clone)]
pub(crate) struct PathSums {
    x: Vec<f64>,
    y: Vec<f64>,
    x2: Vec<f64>,
    xy: Vec<f64>,
    y2: Vec<f64>,
}

impl PathSums {
    fn for_closed_points(points: &[(f64, f64)]) -> Self {
        let count = points.len() * 2 + 1;
        let mut sums = Self {
            x: Vec::with_capacity(count + 1),
            y: Vec::with_capacity(count + 1),
            x2: Vec::with_capacity(count + 1),
            xy: Vec::with_capacity(count + 1),
            y2: Vec::with_capacity(count + 1),
        };
        sums.x.push(0.0);
        sums.y.push(0.0);
        sums.x2.push(0.0);
        sums.xy.push(0.0);
        sums.y2.push(0.0);

        for index in 0..count {
            let point = points[index % points.len()];
            sums.x.push(sums.x[index] + point.0);
            sums.y.push(sums.y[index] + point.1);
            sums.x2.push(sums.x2[index] + point.0 * point.0);
            sums.xy.push(sums.xy[index] + point.0 * point.1);
            sums.y2.push(sums.y2[index] + point.1 * point.1);
        }

        sums
    }

    fn range(&self, start: usize, end: usize) -> PathSumRange {
        let end = end + 1;
        PathSumRange {
            count: (end - start) as f64,
            x: self.x[end] - self.x[start],
            y: self.y[end] - self.y[start],
            x2: self.x2[end] - self.x2[start],
            xy: self.xy[end] - self.xy[start],
            y2: self.y2[end] - self.y2[start],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PathSumRange {
    count: f64,
    x: f64,
    y: f64,
    x2: f64,
    xy: f64,
    y2: f64,
}

pub(crate) fn potrace_possible_segment_is_straight(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
    max_distance: f64,
) -> bool {
    if end <= start + 1 {
        return true;
    }

    if end - start > points.len().saturating_sub(3) {
        return false;
    }

    potrace_subpath_is_straight(points, start as isize - 1, end as isize + 1, max_distance)
}

pub(crate) fn potrace_subpath_is_straight(
    points: &[(f64, f64)],
    start: isize,
    end: isize,
    max_distance: f64,
) -> bool {
    if end <= start + 2 {
        return true;
    }

    if potrace_subpath_uses_all_four_directions(points, start, end) {
        return false;
    }

    let start_point = cyclic_point(points, start);
    let end_point = cyclic_point(points, end);
    if distance_squared_float(start_point, end_point) <= f64::EPSILON {
        return false;
    }

    for index in (start + 1)..end {
        let point = cyclic_point(points, index);
        if max_distance_to_infinite_line(point, start_point, end_point) > max_distance {
            return false;
        }
    }

    true
}

pub(crate) fn potrace_subpath_uses_all_four_directions(
    points: &[(f64, f64)],
    start: isize,
    end: isize,
) -> bool {
    let mut mask = 0u8;

    for index in start..end {
        let from = cyclic_point(points, index);
        let to = cyclic_point(points, index + 1);
        mask |= cardinal_direction_mask(subtract(to, from));
        if mask == 0b1111 {
            return true;
        }
    }

    false
}

pub(crate) fn cardinal_direction_mask(vector: (f64, f64)) -> u8 {
    if vector.0.abs() <= f64::EPSILON && vector.1.abs() <= f64::EPSILON {
        return 0;
    }

    if vector.0.abs() >= vector.1.abs() {
        if vector.0 >= 0.0 {
            0b0001
        } else {
            0b0010
        }
    } else if vector.1 >= 0.0 {
        0b0100
    } else {
        0b1000
    }
}

pub(crate) fn max_distance_to_infinite_line(
    point: (f64, f64),
    line_start: (f64, f64),
    line_end: (f64, f64),
) -> f64 {
    let line = subtract(line_end, line_start);
    let length_squared = vector_length_squared(line);

    if length_squared <= f64::EPSILON {
        return (point.0 - line_start.0)
            .abs()
            .max((point.1 - line_start.1).abs());
    }

    let amount = dot(subtract(point, line_start), line) / length_squared;
    let projection = add(line_start, scale(line, amount));
    (point.0 - projection.0)
        .abs()
        .max((point.1 - projection.1).abs())
}

pub(crate) fn cyclic_point(points: &[(f64, f64)], index: isize) -> (f64, f64) {
    let len = points.len() as isize;
    let index = index.rem_euclid(len) as usize;
    points[index]
}

pub(crate) fn potrace_polygon_segment_penalty(
    points: &[(f64, f64)],
    sums: &PathSums,
    start: usize,
    end: usize,
) -> f64 {
    if end <= start + 1 {
        return 0.0;
    }

    let start_point = closed_point(points, start);
    let end_point = closed_point(points, end);
    let chord = subtract(end_point, start_point);
    let range = sums.range(start, end);
    let a = -chord.1;
    let b = chord.0;
    let c = chord.1 * start_point.0 - chord.0 * start_point.1;
    let squared_error = a * a * range.x2
        + 2.0 * a * b * range.xy
        + b * b * range.y2
        + 2.0 * a * c * range.x
        + 2.0 * b * c * range.y
        + range.count * c * c;

    (squared_error.max(0.0) / range.count).sqrt()
}

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct FitLine {
    pub(crate) point: (f64, f64),
    pub(crate) direction: (f64, f64),
}

pub(crate) fn best_fit_line_for_closed_arc(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
) -> FitLine {
    let arc = closed_arc_points_by_index(points, start, end);

    if arc.len() <= 2 {
        return FitLine {
            point: arc[0],
            direction: unit_vector(subtract(*arc.last().unwrap_or(&arc[0]), arc[0])),
        };
    }

    let centroid = arc_centroid(&arc);
    let mut xx = 0.0;
    let mut xy = 0.0;
    let mut yy = 0.0;

    for point in &arc {
        let centered = subtract(*point, centroid);
        xx += centered.0 * centered.0;
        xy += centered.0 * centered.1;
        yy += centered.1 * centered.1;
    }

    let fallback = unit_vector(subtract(*arc.last().unwrap_or(&arc[0]), arc[0]));
    let direction = principal_axis_2x2(xx, xy, yy).unwrap_or(fallback);

    FitLine {
        point: centroid,
        direction,
    }
}

pub(crate) fn closed_arc_points_by_index(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
) -> Vec<(f64, f64)> {
    let mut arc = Vec::new();
    let mut index = start;

    loop {
        arc.push(points[index]);

        if index == end {
            break;
        }

        index = (index + 1) % points.len();
    }

    arc
}

pub(crate) fn arc_centroid(points: &[(f64, f64)]) -> (f64, f64) {
    let sum = points.iter().copied().fold((0.0, 0.0), add);

    scale(sum, 1.0 / points.len() as f64)
}

pub(crate) fn largest_eigenvalue_2x2(xx: f64, xy: f64, yy: f64) -> f64 {
    let trace = xx + yy;
    let determinant = xx * yy - xy * xy;
    let discriminant = (trace * trace - 4.0 * determinant).max(0.0).sqrt();

    (trace + discriminant) / 2.0
}

pub(crate) fn principal_axis_2x2(xx: f64, xy: f64, yy: f64) -> Option<(f64, f64)> {
    if xx.abs() <= f64::EPSILON && xy.abs() <= f64::EPSILON && yy.abs() <= f64::EPSILON {
        return None;
    }

    let lambda = largest_eigenvalue_2x2(xx, xy, yy);
    let candidates = [(xy, lambda - xx), (lambda - yy, xy)];

    candidates
        .into_iter()
        .find(|candidate| vector_length_squared(*candidate) > f64::EPSILON)
        .map(unit_vector)
        .or({
            if xx >= yy {
                Some((1.0, 0.0))
            } else {
                Some((0.0, 1.0))
            }
        })
}

pub(crate) fn line_intersection(a: FitLine, b: FitLine) -> Option<(f64, f64)> {
    let denominator = cross(a.direction, b.direction);

    if denominator.abs() <= 1.0e-9 {
        return None;
    }

    let amount = cross(subtract(b.point, a.point), b.direction) / denominator;
    Some(add(a.point, scale(a.direction, amount)))
}

pub(crate) fn clamp_point_to_box(point: (f64, f64), center: (f64, f64), radius: f64) -> (f64, f64) {
    (
        point.0.clamp(center.0 - radius, center.0 + radius),
        point.1.clamp(center.1 - radius, center.1 + radius),
    )
}

pub(crate) fn smooth_potrace_vertices(
    points: &[(f64, f64)],
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    smooth_potrace_vertices_with_alpha(points, potrace_curve_alpha)
}

pub(crate) fn smooth_area_alpha_potrace_vertices(
    points: &[(f64, f64)],
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    smooth_potrace_vertices_with_alpha(points, potrace_area_curve_alpha)
}

pub(crate) fn smooth_potrace_vertices_with_alpha(
    points: &[(f64, f64)],
    alpha_for_vertex: PotraceAlphaFn,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    const ALPHA_MIN: f64 = 0.55;
    const ALPHA_MAX: f64 = 1.0;

    if points.len() < 3 {
        return None;
    }

    let first = edge_midpoint(points[points.len() - 1], points[0]);
    let mut segments = Vec::new();
    let mut start = first;

    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let vertex = points[index];
        let next = points[(index + 1) % points.len()];
        let entry = edge_midpoint(previous, vertex);
        let exit = edge_midpoint(vertex, next);
        let alpha = alpha_for_vertex(previous, vertex, next);

        if alpha > ALPHA_MAX {
            segments.push(SvgPathSegment::Line { start, end: vertex });
            segments.push(SvgPathSegment::Line {
                start: vertex,
                end: exit,
            });
        } else {
            let alpha = alpha.clamp(ALPHA_MIN, ALPHA_MAX);
            segments.push(SvgPathSegment::Cubic(CubicSegment {
                start: entry,
                control1: interpolate(entry, vertex, alpha),
                control2: interpolate(exit, vertex, alpha),
                end: exit,
            }));
        }

        start = exit;
    }

    Some((first, segments))
}

pub(crate) fn closed_point(points: &[(f64, f64)], index: usize) -> (f64, f64) {
    points[index % points.len()]
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SvgPathSegment {
    Line { start: (f64, f64), end: (f64, f64) },
    Cubic(CubicSegment),
}

impl SvgPathSegment {
    pub(crate) fn start(self) -> (f64, f64) {
        match self {
            SvgPathSegment::Line { start, .. } => start,
            SvgPathSegment::Cubic(cubic) => cubic.start,
        }
    }

    pub(crate) fn end(self) -> (f64, f64) {
        match self {
            SvgPathSegment::Line { end, .. } => end,
            SvgPathSegment::Cubic(cubic) => cubic.end,
        }
    }
}
