use std::collections::{BTreeMap, HashSet};

use crate::{Bitmap, TraceOptions, TracePath, TracedBitmap};

pub(crate) fn trace_pixel_bitmap(bitmap: &Bitmap, options: TraceOptions) -> TracedBitmap {
    let edges = boundary_edges(bitmap);
    let mut outgoing: BTreeMap<Point, Vec<Edge>> = BTreeMap::new();

    for edge in &edges {
        outgoing.entry(edge.start).or_default().push(*edge);
    }

    for edges in outgoing.values_mut() {
        edges.sort();
    }

    let mut visited = HashSet::new();
    let mut paths = Vec::new();

    for edge in edges {
        if visited.contains(&edge) {
            continue;
        }

        if let Some(points) = trace_path(edge, &outgoing, &mut visited) {
            let points = if options.preserve_collinear {
                points
            } else {
                simplify_collinear(&points)
            };
            let points = optimize_path(&points, options.opt_tolerance.max(0.0));

            if points.len() >= 3 {
                let area2 = signed_area2(&points);

                if is_below_turd_size(area2, options.turd_size) {
                    continue;
                }

                paths.push(TracePath {
                    is_hole: area2 < 0,
                    points: points
                        .into_iter()
                        .map(|point| (f64::from(point.x), f64::from(point.y)))
                        .collect(),
                });
            }
        }
    }

    TracedBitmap {
        width: bitmap.width(),
        height: bitmap.height(),
        paths,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: usize, y: usize) -> Self {
        Self {
            x: x as i32,
            y: y as i32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Edge {
    start: Point,
    end: Point,
}

impl Edge {
    fn direction(self) -> (i32, i32) {
        (
            (self.end.x - self.start.x).signum(),
            (self.end.y - self.start.y).signum(),
        )
    }
}

pub(crate) fn boundary_edges(bitmap: &Bitmap) -> Vec<Edge> {
    let mut edges = Vec::new();

    for y in 0..bitmap.height() {
        for x in 0..bitmap.width() {
            if !bitmap.is_black(x, y) {
                continue;
            }

            if y == 0 || !bitmap.is_black(x, y - 1) {
                edges.push(Edge {
                    start: Point::new(x, y),
                    end: Point::new(x + 1, y),
                });
            }

            if x + 1 == bitmap.width() || !bitmap.is_black(x + 1, y) {
                edges.push(Edge {
                    start: Point::new(x + 1, y),
                    end: Point::new(x + 1, y + 1),
                });
            }

            if y + 1 == bitmap.height() || !bitmap.is_black(x, y + 1) {
                edges.push(Edge {
                    start: Point::new(x + 1, y + 1),
                    end: Point::new(x, y + 1),
                });
            }

            if x == 0 || !bitmap.is_black(x - 1, y) {
                edges.push(Edge {
                    start: Point::new(x, y + 1),
                    end: Point::new(x, y),
                });
            }
        }
    }

    edges.sort();
    edges
}

pub(crate) fn trace_path(
    start_edge: Edge,
    outgoing: &BTreeMap<Point, Vec<Edge>>,
    visited: &mut HashSet<Edge>,
) -> Option<Vec<Point>> {
    let mut current = start_edge;
    let mut points = vec![start_edge.start];

    loop {
        if !visited.insert(current) {
            return None;
        }

        points.push(current.end);

        if current.end == start_edge.start {
            points.pop();
            return Some(points);
        }

        current = choose_next_edge(current, outgoing.get(&current.end)?, visited)?;
    }
}

pub(crate) fn choose_next_edge(
    current: Edge,
    candidates: &[Edge],
    visited: &HashSet<Edge>,
) -> Option<Edge> {
    let current_direction = current.direction();
    let preferred = [
        right_turn(current_direction),
        current_direction,
        left_turn(current_direction),
        (-current_direction.0, -current_direction.1),
    ];

    preferred.into_iter().find_map(|direction| {
        candidates
            .iter()
            .copied()
            .find(|edge| !visited.contains(edge) && edge.direction() == direction)
    })
}

pub(crate) fn right_turn((dx, dy): (i32, i32)) -> (i32, i32) {
    (-dy, dx)
}

pub(crate) fn left_turn((dx, dy): (i32, i32)) -> (i32, i32) {
    (dy, -dx)
}

pub(crate) fn simplify_collinear(points: &[Point]) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut simplified = Vec::new();

    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];

        let ab = (current.x - previous.x, current.y - previous.y);
        let bc = (next.x - current.x, next.y - current.y);

        if ab.0 * bc.1 - ab.1 * bc.0 != 0 {
            simplified.push(current);
        }
    }

    simplified
}

pub(crate) fn optimize_path(points: &[Point], tolerance: f64) -> Vec<Point> {
    if tolerance <= 0.0 || points.len() <= 3 {
        return points.to_vec();
    }

    let start_index = farthest_pair_start_index(points);
    let mut open_points = rotate_closed_points(points, start_index);
    open_points.push(open_points[0]);

    let mut keep = vec![false; open_points.len()];
    keep[0] = true;
    keep[open_points.len() - 1] = true;

    mark_rdp_points(&open_points, 0, open_points.len() - 1, tolerance, &mut keep);

    let mut optimized = open_points
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, point)| keep[index].then_some(point))
        .collect::<Vec<_>>();

    if optimized.len() > 1 && optimized.first() == optimized.last() {
        optimized.pop();
    }

    if optimized.len() < 3 {
        points.to_vec()
    } else {
        optimized
    }
}

pub(crate) fn farthest_pair_start_index(points: &[Point]) -> usize {
    let mut best = (0usize, 0i64);

    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let distance = distance_squared(points[i], points[j]);

            if distance > best.1 {
                best = (i, distance);
            }
        }
    }

    best.0
}

pub(crate) fn rotate_closed_points(points: &[Point], start_index: usize) -> Vec<Point> {
    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

pub(crate) fn mark_rdp_points(
    points: &[Point],
    start_index: usize,
    end_index: usize,
    tolerance: f64,
    keep: &mut [bool],
) {
    if end_index <= start_index + 1 {
        return;
    }

    let mut farthest_index = start_index;
    let mut farthest_distance = 0.0;

    for index in (start_index + 1)..end_index {
        let distance =
            perpendicular_distance(points[index], points[start_index], points[end_index]);

        if distance > farthest_distance {
            farthest_distance = distance;
            farthest_index = index;
        }
    }

    if farthest_distance > tolerance {
        keep[farthest_index] = true;
        mark_rdp_points(points, start_index, farthest_index, tolerance, keep);
        mark_rdp_points(points, farthest_index, end_index, tolerance, keep);
    }
}

pub(crate) fn perpendicular_distance(point: Point, line_start: Point, line_end: Point) -> f64 {
    let dx = f64::from(line_end.x - line_start.x);
    let dy = f64::from(line_end.y - line_start.y);

    if dx == 0.0 && dy == 0.0 {
        return f64::from(point.x - line_start.x).hypot(f64::from(point.y - line_start.y));
    }

    let numerator =
        (dy * f64::from(point.x - line_start.x) - dx * f64::from(point.y - line_start.y)).abs();
    let denominator = dx.hypot(dy);

    numerator / denominator
}

pub(crate) fn distance_squared(a: Point, b: Point) -> i64 {
    let dx = i64::from(a.x - b.x);
    let dy = i64::from(a.y - b.y);

    dx * dx + dy * dy
}

pub(crate) fn signed_area2(points: &[Point]) -> i64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| i64::from(a.x) * i64::from(b.y) - i64::from(b.x) * i64::from(a.y))
        .sum()
}

pub(crate) fn is_below_turd_size(area2: i64, turd_size: usize) -> bool {
    if turd_size == 0 {
        return false;
    }

    area2.unsigned_abs() <= (turd_size as u64).saturating_mul(2)
}
