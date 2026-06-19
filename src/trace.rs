use std::collections::{BTreeMap, HashSet};

use crate::raster::apply_invert;
use crate::{
    Bitmap, BitmapError, ContourMode, RasterOptions, ScalarField, TraceOptions, TracePath,
    TracedBitmap,
};

pub fn trace_bitmap(bitmap: &Bitmap, options: TraceOptions) -> TracedBitmap {
    match options.contour_mode {
        ContourMode::Pixel => trace_pixel_bitmap(bitmap, options),
        ContourMode::Subpixel | ContourMode::Scalar => trace_subpixel_bitmap(bitmap, options),
    }
}

pub fn trace_scalar_field(
    field: &ScalarField,
    raster_options: RasterOptions,
    trace_options: TraceOptions,
) -> Result<TracedBitmap, BitmapError> {
    if trace_options.contour_mode != ContourMode::Scalar {
        let bitmap = field.to_bitmap(raster_options)?;
        return Ok(trace_bitmap(&bitmap, trace_options));
    }

    Ok(trace_scalar_bitmap(field, raster_options, trace_options))
}

pub(crate) fn rasterize_path_evenodd(
    path: &TracePath,
    width: usize,
    height: usize,
    pixels: &mut [bool],
) {
    if path.points.len() < 3 {
        return;
    }

    let mut intersections = Vec::new();

    for y in 0..height {
        let scan_y = y as f64 + 0.5;
        intersections.clear();

        for (start, end) in path.points.iter().zip(path.points.iter().cycle().skip(1)) {
            if (start.1 <= scan_y && scan_y < end.1) || (end.1 <= scan_y && scan_y < start.1) {
                let amount = (scan_y - start.1) / (end.1 - start.1);
                intersections.push(start.0 + (end.0 - start.0) * amount);
            }
        }

        intersections.sort_by(|a, b| a.total_cmp(b));

        for pair in intersections.chunks_exact(2) {
            let left = pair[0].min(pair[1]);
            let right = pair[0].max(pair[1]);
            let start_x = clamp_scanline_x((left - 0.5).ceil(), width);
            let end_x = clamp_scanline_x((right - 0.5).ceil(), width);

            for x in start_x..end_x {
                let index = y * width + x;
                pixels[index] = !pixels[index];
            }
        }
    }
}

pub(crate) fn clamp_scanline_x(value: f64, width: usize) -> usize {
    if value <= 0.0 {
        0
    } else if value >= width as f64 {
        width
    } else {
        value as usize
    }
}

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

pub(crate) fn trace_subpixel_bitmap(bitmap: &Bitmap, options: TraceOptions) -> TracedBitmap {
    let segments = subpixel_segments(bitmap);
    let loops = trace_subpixel_loops(&segments);
    let mut paths = Vec::new();

    for points in loops {
        let points = simplify_subpixel_collinear(&points);
        let points = optimize_subpixel_path(&points, options.opt_tolerance.max(0.0));
        let points = rotate_subpixel_loop_to_top(points);
        if points.len() < 3 {
            continue;
        }

        let area = signed_subpixel_area(&points);
        if is_below_turd_size_float(area, options.turd_size) {
            continue;
        }

        paths.push(TracePath {
            is_hole: area < 0.0,
            points: points
                .into_iter()
                .map(|point| (point.to_float().0, point.to_float().1))
                .collect(),
        });
    }

    TracedBitmap {
        width: bitmap.width(),
        height: bitmap.height(),
        paths,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SubPoint {
    x2: i32,
    y2: i32,
}

impl SubPoint {
    fn to_float(self) -> (f64, f64) {
        (f64::from(self.x2) / 2.0, f64::from(self.y2) / 2.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SubSegment {
    start: SubPoint,
    end: SubPoint,
}

impl SubSegment {
    fn new(start: SubPoint, end: SubPoint) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct IsoPoint {
    x: i64,
    y: i64,
}

impl IsoPoint {
    const SCALE: f64 = 1_000_000.0;

    fn new(x: f64, y: f64) -> Self {
        Self {
            x: (x * Self::SCALE).round() as i64,
            y: (y * Self::SCALE).round() as i64,
        }
    }

    fn to_float(self) -> (f64, f64) {
        (self.x as f64 / Self::SCALE, self.y as f64 / Self::SCALE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct IsoSegment {
    start: IsoPoint,
    end: IsoPoint,
}

impl IsoSegment {
    fn new(start: IsoPoint, end: IsoPoint) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }
}

pub(crate) fn trace_scalar_bitmap(
    field: &ScalarField,
    raster_options: RasterOptions,
    trace_options: TraceOptions,
) -> TracedBitmap {
    let threshold = f64::from(raster_options.threshold.resolve(field.samples()));
    let segments = scalar_segments(field, raster_options, threshold);
    let loops = trace_iso_loops(&segments);
    let mut paths = Vec::new();

    for points in loops {
        let points = optimize_iso_path(&points, trace_options.opt_tolerance.max(0.0));
        let points = rotate_iso_loop_to_top(points);
        if points.len() < 3 {
            continue;
        }

        let area = signed_iso_area(&points);
        if is_below_turd_size_float(area, trace_options.turd_size) {
            continue;
        }

        paths.push(TracePath {
            is_hole: area < 0.0,
            points: points.into_iter().map(IsoPoint::to_float).collect(),
        });
    }

    TracedBitmap {
        width: field.width(),
        height: field.height(),
        paths,
    }
}

pub(crate) fn scalar_segments(
    field: &ScalarField,
    raster_options: RasterOptions,
    threshold: f64,
) -> Vec<IsoSegment> {
    let mut segments = Vec::new();

    for y in 0..=field.height() {
        for x in 0..=field.width() {
            let top_left = padded_scalar_sample(field, raster_options, x, y);
            let top_right = padded_scalar_sample(field, raster_options, x + 1, y);
            let bottom_right = padded_scalar_sample(field, raster_options, x + 1, y + 1);
            let bottom_left = padded_scalar_sample(field, raster_options, x, y + 1);

            let top_left_inside = scalar_is_foreground(top_left, threshold, raster_options);
            let top_right_inside = scalar_is_foreground(top_right, threshold, raster_options);
            let bottom_right_inside = scalar_is_foreground(bottom_right, threshold, raster_options);
            let bottom_left_inside = scalar_is_foreground(bottom_left, threshold, raster_options);
            let cell = (top_left_inside as u8)
                | ((top_right_inside as u8) << 1)
                | ((bottom_right_inside as u8) << 2)
                | ((bottom_left_inside as u8) << 3);

            let top = scalar_edge_point(
                x as f64 - 0.5,
                y as f64 - 0.5,
                x as f64 + 0.5,
                y as f64 - 0.5,
                top_left,
                top_right,
                threshold,
            );
            let right = scalar_edge_point(
                x as f64 + 0.5,
                y as f64 - 0.5,
                x as f64 + 0.5,
                y as f64 + 0.5,
                top_right,
                bottom_right,
                threshold,
            );
            let bottom = scalar_edge_point(
                x as f64 - 0.5,
                y as f64 + 0.5,
                x as f64 + 0.5,
                y as f64 + 0.5,
                bottom_left,
                bottom_right,
                threshold,
            );
            let left = scalar_edge_point(
                x as f64 - 0.5,
                y as f64 - 0.5,
                x as f64 - 0.5,
                y as f64 + 0.5,
                top_left,
                bottom_left,
                threshold,
            );

            match cell {
                0 | 15 => {}
                1 => segments.push(IsoSegment::new(top, left)),
                2 => segments.push(IsoSegment::new(right, top)),
                3 => segments.push(IsoSegment::new(right, left)),
                4 => segments.push(IsoSegment::new(bottom, right)),
                5 => {
                    segments.push(IsoSegment::new(top, left));
                    segments.push(IsoSegment::new(bottom, right));
                }
                6 => segments.push(IsoSegment::new(bottom, top)),
                7 => segments.push(IsoSegment::new(bottom, left)),
                8 => segments.push(IsoSegment::new(left, bottom)),
                9 => segments.push(IsoSegment::new(top, bottom)),
                10 => {
                    segments.push(IsoSegment::new(right, top));
                    segments.push(IsoSegment::new(left, bottom));
                }
                11 => segments.push(IsoSegment::new(right, bottom)),
                12 => segments.push(IsoSegment::new(left, right)),
                13 => segments.push(IsoSegment::new(top, right)),
                14 => segments.push(IsoSegment::new(left, top)),
                _ => unreachable!("marching-squares cell index is four bits"),
            }
        }
    }

    segments.sort();
    segments.dedup();
    segments
}

pub(crate) fn padded_scalar_sample(
    field: &ScalarField,
    raster_options: RasterOptions,
    x: usize,
    y: usize,
) -> f64 {
    if x == 0 || y == 0 || x > field.width() || y > field.height() {
        if raster_options.invert {
            0.0
        } else {
            255.0
        }
    } else {
        f64::from(field.sample(x - 1, y - 1))
    }
}

pub(crate) fn scalar_is_foreground(
    sample: f64,
    threshold: f64,
    raster_options: RasterOptions,
) -> bool {
    apply_invert(sample < threshold, raster_options)
}

pub(crate) fn scalar_edge_point(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    sample0: f64,
    sample1: f64,
    threshold: f64,
) -> IsoPoint {
    let delta = sample1 - sample0;
    let amount = if delta.abs() <= f64::EPSILON {
        0.5
    } else {
        ((threshold - sample0) / delta).clamp(0.0, 1.0)
    };

    IsoPoint::new(x0 + (x1 - x0) * amount, y0 + (y1 - y0) * amount)
}

pub(crate) fn subpixel_segments(bitmap: &Bitmap) -> Vec<SubSegment> {
    let mut segments = Vec::new();

    for y in 0..=bitmap.height() {
        for x in 0..=bitmap.width() {
            let top_left = padded_black_sample(bitmap, x, y);
            let top_right = padded_black_sample(bitmap, x + 1, y);
            let bottom_right = padded_black_sample(bitmap, x + 1, y + 1);
            let bottom_left = padded_black_sample(bitmap, x, y + 1);
            let cell = (top_left as u8)
                | ((top_right as u8) << 1)
                | ((bottom_right as u8) << 2)
                | ((bottom_left as u8) << 3);

            let top = SubPoint {
                x2: (x as i32) * 2,
                y2: (y as i32) * 2 - 1,
            };
            let right = SubPoint {
                x2: (x as i32) * 2 + 1,
                y2: (y as i32) * 2,
            };
            let bottom = SubPoint {
                x2: (x as i32) * 2,
                y2: (y as i32) * 2 + 1,
            };
            let left = SubPoint {
                x2: (x as i32) * 2 - 1,
                y2: (y as i32) * 2,
            };

            match cell {
                0 | 15 => {}
                1 => segments.push(SubSegment::new(top, left)),
                2 => segments.push(SubSegment::new(right, top)),
                3 => segments.push(SubSegment::new(right, left)),
                4 => segments.push(SubSegment::new(bottom, right)),
                5 => {
                    segments.push(SubSegment::new(top, left));
                    segments.push(SubSegment::new(bottom, right));
                }
                6 => segments.push(SubSegment::new(bottom, top)),
                7 => segments.push(SubSegment::new(bottom, left)),
                8 => segments.push(SubSegment::new(left, bottom)),
                9 => segments.push(SubSegment::new(top, bottom)),
                10 => {
                    segments.push(SubSegment::new(right, top));
                    segments.push(SubSegment::new(left, bottom));
                }
                11 => segments.push(SubSegment::new(right, bottom)),
                12 => segments.push(SubSegment::new(left, right)),
                13 => segments.push(SubSegment::new(top, right)),
                14 => segments.push(SubSegment::new(left, top)),
                _ => unreachable!("marching-squares cell index is four bits"),
            }
        }
    }

    segments.sort();
    segments.dedup();
    segments
}

pub(crate) fn padded_black_sample(bitmap: &Bitmap, x: usize, y: usize) -> bool {
    if x == 0 || y == 0 || x > bitmap.width() || y > bitmap.height() {
        false
    } else {
        bitmap.is_black(x - 1, y - 1)
    }
}

pub(crate) fn trace_subpixel_loops(segments: &[SubSegment]) -> Vec<Vec<SubPoint>> {
    let mut outgoing: BTreeMap<SubPoint, Vec<SubPoint>> = BTreeMap::new();

    for segment in segments {
        outgoing.entry(segment.start).or_default().push(segment.end);
        outgoing.entry(segment.end).or_default().push(segment.start);
    }

    for neighbors in outgoing.values_mut() {
        neighbors.sort();
    }

    let mut visited = HashSet::new();
    let mut loops = Vec::new();

    for segment in segments {
        if visited.contains(segment) {
            continue;
        }

        if let Some(points) = trace_subpixel_loop(*segment, &outgoing, &mut visited) {
            loops.push(points);
        }
    }

    loops
}

pub(crate) fn trace_subpixel_loop(
    start_segment: SubSegment,
    outgoing: &BTreeMap<SubPoint, Vec<SubPoint>>,
    visited: &mut HashSet<SubSegment>,
) -> Option<Vec<SubPoint>> {
    let start = start_segment.start;
    let mut previous = start_segment.start;
    let mut current = start_segment.end;
    let mut points = vec![start, current];
    visited.insert(start_segment);

    while current != start {
        let next = outgoing.get(&current)?.iter().copied().find(|candidate| {
            *candidate != previous && !visited.contains(&SubSegment::new(current, *candidate))
        })?;

        visited.insert(SubSegment::new(current, next));
        previous = current;
        current = next;

        if current != start {
            points.push(current);
        }
    }

    Some(points)
}

pub(crate) fn trace_iso_loops(segments: &[IsoSegment]) -> Vec<Vec<IsoPoint>> {
    let mut outgoing: BTreeMap<IsoPoint, Vec<IsoPoint>> = BTreeMap::new();

    for segment in segments {
        outgoing.entry(segment.start).or_default().push(segment.end);
        outgoing.entry(segment.end).or_default().push(segment.start);
    }

    for neighbors in outgoing.values_mut() {
        neighbors.sort();
    }

    let mut visited = HashSet::new();
    let mut loops = Vec::new();

    for segment in segments {
        if visited.contains(segment) {
            continue;
        }

        if let Some(points) = trace_iso_loop(*segment, &outgoing, &mut visited) {
            loops.push(points);
        }
    }

    loops
}

pub(crate) fn trace_iso_loop(
    start_segment: IsoSegment,
    outgoing: &BTreeMap<IsoPoint, Vec<IsoPoint>>,
    visited: &mut HashSet<IsoSegment>,
) -> Option<Vec<IsoPoint>> {
    let start = start_segment.start;
    let mut previous = start_segment.start;
    let mut current = start_segment.end;
    let mut points = vec![start, current];
    visited.insert(start_segment);

    while current != start {
        let next = outgoing.get(&current)?.iter().copied().find(|candidate| {
            *candidate != previous && !visited.contains(&IsoSegment::new(current, *candidate))
        })?;

        visited.insert(IsoSegment::new(current, next));
        previous = current;
        current = next;

        if current != start {
            points.push(current);
        }
    }

    Some(points)
}

pub(crate) fn simplify_subpixel_collinear(points: &[SubPoint]) -> Vec<SubPoint> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut simplified = Vec::new();

    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];

        let ab = (current.x2 - previous.x2, current.y2 - previous.y2);
        let bc = (next.x2 - current.x2, next.y2 - current.y2);

        if ab.0 * bc.1 - ab.1 * bc.0 != 0 {
            simplified.push(current);
        }
    }

    simplified
}

pub(crate) fn rotate_subpixel_loop_to_top(points: Vec<SubPoint>) -> Vec<SubPoint> {
    let Some((start_index, _)) = points
        .iter()
        .enumerate()
        .min_by_key(|(_, point)| (point.y2, point.x2))
    else {
        return points;
    };

    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

pub(crate) fn rotate_iso_loop_to_top(points: Vec<IsoPoint>) -> Vec<IsoPoint> {
    let Some((start_index, _)) = points
        .iter()
        .enumerate()
        .min_by_key(|(_, point)| (point.y, point.x))
    else {
        return points;
    };

    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

pub(crate) fn optimize_subpixel_path(points: &[SubPoint], tolerance: f64) -> Vec<SubPoint> {
    if tolerance <= 0.0 || points.len() <= 3 {
        return points.to_vec();
    }

    let start_index = farthest_subpixel_pair_start_index(points);
    let mut open_points = rotate_subpixel_points(points, start_index);
    open_points.push(open_points[0]);

    let mut keep = vec![false; open_points.len()];
    keep[0] = true;
    keep[open_points.len() - 1] = true;

    mark_subpixel_rdp_points(&open_points, 0, open_points.len() - 1, tolerance, &mut keep);

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

pub(crate) fn optimize_iso_path(points: &[IsoPoint], tolerance: f64) -> Vec<IsoPoint> {
    if tolerance <= 0.0 || points.len() <= 3 {
        return points.to_vec();
    }

    let start_index = farthest_iso_pair_start_index(points);
    let mut open_points = rotate_iso_points(points, start_index);
    open_points.push(open_points[0]);

    let mut keep = vec![false; open_points.len()];
    keep[0] = true;
    keep[open_points.len() - 1] = true;

    mark_iso_rdp_points(&open_points, 0, open_points.len() - 1, tolerance, &mut keep);

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

pub(crate) fn farthest_subpixel_pair_start_index(points: &[SubPoint]) -> usize {
    let mut best = (0usize, 0i64);

    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let distance = subpixel_distance_squared(points[i], points[j]);

            if distance > best.1 {
                best = (i, distance);
            }
        }
    }

    best.0
}

pub(crate) fn farthest_iso_pair_start_index(points: &[IsoPoint]) -> usize {
    let mut best = (0usize, 0i128);

    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let distance = iso_distance_squared(points[i], points[j]);

            if distance > best.1 {
                best = (i, distance);
            }
        }
    }

    best.0
}

pub(crate) fn rotate_subpixel_points(points: &[SubPoint], start_index: usize) -> Vec<SubPoint> {
    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

pub(crate) fn rotate_iso_points(points: &[IsoPoint], start_index: usize) -> Vec<IsoPoint> {
    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

pub(crate) fn mark_subpixel_rdp_points(
    points: &[SubPoint],
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
            subpixel_perpendicular_distance(points[index], points[start_index], points[end_index]);

        if distance > farthest_distance {
            farthest_distance = distance;
            farthest_index = index;
        }
    }

    if farthest_distance > tolerance {
        keep[farthest_index] = true;
        mark_subpixel_rdp_points(points, start_index, farthest_index, tolerance, keep);
        mark_subpixel_rdp_points(points, farthest_index, end_index, tolerance, keep);
    }
}

pub(crate) fn mark_iso_rdp_points(
    points: &[IsoPoint],
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
            iso_perpendicular_distance(points[index], points[start_index], points[end_index]);

        if distance > farthest_distance {
            farthest_distance = distance;
            farthest_index = index;
        }
    }

    if farthest_distance > tolerance {
        keep[farthest_index] = true;
        mark_iso_rdp_points(points, start_index, farthest_index, tolerance, keep);
        mark_iso_rdp_points(points, farthest_index, end_index, tolerance, keep);
    }
}

pub(crate) fn subpixel_perpendicular_distance(
    point: SubPoint,
    line_start: SubPoint,
    line_end: SubPoint,
) -> f64 {
    let dx = f64::from(line_end.x2 - line_start.x2);
    let dy = f64::from(line_end.y2 - line_start.y2);

    if dx == 0.0 && dy == 0.0 {
        return f64::from(point.x2 - line_start.x2).hypot(f64::from(point.y2 - line_start.y2))
            / 2.0;
    }

    let numerator =
        (dy * f64::from(point.x2 - line_start.x2) - dx * f64::from(point.y2 - line_start.y2)).abs();
    let denominator = dx.hypot(dy);

    numerator / denominator / 2.0
}

pub(crate) fn iso_perpendicular_distance(
    point: IsoPoint,
    line_start: IsoPoint,
    line_end: IsoPoint,
) -> f64 {
    let point = point.to_float();
    let line_start = line_start.to_float();
    let line_end = line_end.to_float();

    let dx = line_end.0 - line_start.0;
    let dy = line_end.1 - line_start.1;

    if dx == 0.0 && dy == 0.0 {
        return (point.0 - line_start.0).hypot(point.1 - line_start.1);
    }

    let numerator = (dy * (point.0 - line_start.0) - dx * (point.1 - line_start.1)).abs();
    let denominator = dx.hypot(dy);

    numerator / denominator
}

pub(crate) fn subpixel_distance_squared(a: SubPoint, b: SubPoint) -> i64 {
    let dx = i64::from(a.x2 - b.x2);
    let dy = i64::from(a.y2 - b.y2);

    dx * dx + dy * dy
}

pub(crate) fn iso_distance_squared(a: IsoPoint, b: IsoPoint) -> i128 {
    let dx = i128::from(a.x - b.x);
    let dy = i128::from(a.y - b.y);

    dx * dx + dy * dy
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

pub(crate) fn signed_subpixel_area(points: &[SubPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| {
            let a = a.to_float();
            let b = b.to_float();
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

pub(crate) fn signed_iso_area(points: &[IsoPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| {
            let a = a.to_float();
            let b = b.to_float();
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

pub(crate) fn is_below_turd_size(area2: i64, turd_size: usize) -> bool {
    if turd_size == 0 {
        return false;
    }

    area2.unsigned_abs() <= (turd_size as u64).saturating_mul(2)
}

pub(crate) fn is_below_turd_size_float(area: f64, turd_size: usize) -> bool {
    turd_size != 0 && area.abs() <= turd_size as f64
}
