use super::*;

pub(crate) fn local_bounds(
    points: &[(f64, f64)],
    origin: (f64, f64),
    axis: (f64, f64),
) -> Option<FloatBounds> {
    let normal = left_normal(axis);
    let local_points = points
        .iter()
        .map(|point| point_to_local(*point, origin, axis, normal))
        .collect::<Vec<_>>();

    FloatBounds::from_points(&local_points)
}

pub(crate) fn point_to_local(
    point: (f64, f64),
    origin: (f64, f64),
    axis: (f64, f64),
    normal: (f64, f64),
) -> (f64, f64) {
    let vector = subtract(point, origin);
    (dot(vector, axis), dot(vector, normal))
}

pub(crate) fn left_normal(axis: (f64, f64)) -> (f64, f64) {
    (-axis.1, axis.0)
}

pub(crate) fn rotate_vector(vector: (f64, f64), angle: f64) -> (f64, f64) {
    let cos = angle.cos();
    let sin = angle.sin();
    (
        vector.0 * cos - vector.1 * sin,
        vector.0 * sin + vector.1 * cos,
    )
}

pub(crate) fn positive_x_axis(axis: (f64, f64)) -> (f64, f64) {
    let axis = unit_vector(axis);
    if axis.0 < 0.0 {
        (-axis.0, -axis.1)
    } else {
        axis
    }
}

#[cfg(test)]
pub(crate) fn line_as_cubic(start: (f64, f64), end: (f64, f64)) -> CubicSegment {
    CubicSegment {
        start,
        control1: interpolate(start, end, 1.0 / 3.0),
        control2: interpolate(start, end, 2.0 / 3.0),
        end,
    }
}

pub(crate) fn fit_closed_ellipse_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    let (center, rx, ry) = closed_ellipse_fit(points)?;

    Some(potrace_like_ellipse_segments(center, rx, ry))
}

pub(crate) fn fit_closed_smooth_ellipse_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    const CENTER_BIAS: f64 = -0.5;
    const RADIUS_BIAS: f64 = -0.2;

    let (center, rx, ry) = closed_ellipse_fit(points)?;
    let center = (center.0 + CENTER_BIAS, center.1 + CENTER_BIAS);
    let rx = (rx + RADIUS_BIAS).max(1.0);
    let ry = (ry + RADIUS_BIAS).max(1.0);

    Some(potrace_like_ellipse_segments(center, rx, ry))
}

pub(crate) fn closed_ellipse_fit(points: &[(f64, f64)]) -> Option<((f64, f64), f64, f64)> {
    const MIN_AXIS: f64 = 8.0;
    const MAX_RADIAL_ERROR: f64 = 0.075;
    const MAX_MEAN_RADIAL_ERROR: f64 = 0.03;

    let bounds = FloatBounds::from_points(points)?;
    let rx = (bounds.max_x - bounds.min_x) / 2.0;
    let ry = (bounds.max_y - bounds.min_y) / 2.0;
    if rx < MIN_AXIS || ry < MIN_AXIS {
        return None;
    }

    let center = (
        (bounds.min_x + bounds.max_x) / 2.0,
        (bounds.min_y + bounds.max_y) / 2.0,
    );
    let mut max_error = 0.0_f64;
    let mut total_error = 0.0_f64;

    for point in points {
        let nx = (point.0 - center.0) / rx;
        let ny = (point.1 - center.1) / ry;
        let error = ((nx * nx + ny * ny).sqrt() - 1.0).abs();
        max_error = max_error.max(error);
        total_error += error;
    }

    let mean_error = total_error / points.len() as f64;
    if max_error > MAX_RADIAL_ERROR || mean_error > MAX_MEAN_RADIAL_ERROR {
        return None;
    }

    Some((center, rx, ry))
}

pub(crate) fn potrace_like_ellipse_segments(
    center: (f64, f64),
    rx: f64,
    ry: f64,
) -> Vec<SvgPathSegment> {
    // Normalized from Potrace 1.16's 256px circle fixture. Potrace's five-cubic
    // fit is asymmetric at pixel scale, and matches the raster oracle better
    // than a mathematically even five-arc ellipse.
    let points = [
        [
            (-0.197_368_421_052_631_58, -0.980_263_157_894_736_8),
            (-0.525, -0.910_526_315_789_473_7),
            (-0.794_736_842_105_263_2, -0.689_473_684_210_526_3),
            (-0.918_421_052_631_578_9, -0.390_789_473_684_210_5),
        ],
        [
            (-0.918_421_052_631_578_9, -0.390_789_473_684_210_5),
            (-1.163_157_894_736_842_2, 0.198_684_210_526_315_8),
            (-0.818_421_052_631_578_9, 0.851_315_789_473_684_2),
            (-0.194_736_842_105_263_15, 0.978_947_368_421_052_7),
        ],
        [
            (-0.194_736_842_105_263_15, 0.978_947_368_421_052_7),
            (0.264_473_684_210_526_3, 1.073_684_210_526_315_8),
            (0.739_473_684_210_526_3, 0.822_368_421_052_631_5),
            (0.919_736_842_105_263_2, 0.390_789_473_684_210_5),
        ],
        [
            (0.919_736_842_105_263_2, 0.390_789_473_684_210_5),
            (1.163_157_894_736_842_2, -0.198_684_210_526_315_8),
            (0.818_421_052_631_578_9, -0.851_315_789_473_684_2),
            (0.194_736_842_105_263_15, -0.978_947_368_421_052_7),
        ],
        [
            (0.194_736_842_105_263_15, -0.978_947_368_421_052_7),
            (0.072_368_421_052_631_58, -1.003_947_368_421_052_7),
            (-0.081_578_947_368_421_06, -1.003_947_368_421_052_7),
            (-0.197_368_421_052_631_58, -0.980_263_157_894_736_8),
        ],
    ];

    points
        .into_iter()
        .map(|[start, control1, control2, end]| {
            SvgPathSegment::Cubic(CubicSegment {
                start: ellipse_normalized_point(center, rx, ry, start),
                control1: ellipse_normalized_point(center, rx, ry, control1),
                control2: ellipse_normalized_point(center, rx, ry, control2),
                end: ellipse_normalized_point(center, rx, ry, end),
            })
        })
        .collect()
}

pub(crate) fn ellipse_normalized_point(
    center: (f64, f64),
    rx: f64,
    ry: f64,
    point: (f64, f64),
) -> (f64, f64) {
    (center.0 + rx * point.0, center.1 + ry * point.1)
}

#[cfg(test)]
pub(crate) fn ellipse_arc_segments(
    center: (f64, f64),
    rx: f64,
    ry: f64,
    segment_count: usize,
) -> Vec<SvgPathSegment> {
    let step = 2.0 * std::f64::consts::PI / segment_count as f64;
    let handle = (4.0 / 3.0) * (step / 4.0).tan();

    (0..segment_count)
        .map(|index| {
            let start_angle = std::f64::consts::PI + step * index as f64;
            let end_angle = start_angle + step;
            let start = ellipse_point(center, rx, ry, start_angle);
            let end = ellipse_point(center, rx, ry, end_angle);
            let start_tangent = ellipse_tangent(rx, ry, start_angle);
            let end_tangent = ellipse_tangent(rx, ry, end_angle);

            SvgPathSegment::Cubic(CubicSegment {
                start,
                control1: add(start, scale(start_tangent, handle)),
                control2: subtract(end, scale(end_tangent, handle)),
                end,
            })
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn ellipse_point(center: (f64, f64), rx: f64, ry: f64, angle: f64) -> (f64, f64) {
    (center.0 + rx * angle.cos(), center.1 + ry * angle.sin())
}

#[cfg(test)]
pub(crate) fn ellipse_tangent(rx: f64, ry: f64, angle: f64) -> (f64, f64) {
    (-rx * angle.sin(), ry * angle.cos())
}

pub(crate) fn points_are_half_pixel_quantized(points: &[(f64, f64)]) -> bool {
    points
        .iter()
        .all(|point| is_half_pixel_quantized(point.0) && is_half_pixel_quantized(point.1))
}

pub(crate) fn is_half_pixel_quantized(value: f64) -> bool {
    let doubled = value * 2.0;
    (doubled - doubled.round()).abs() <= 1.0e-6
}

pub(crate) fn sample_cubic_run(run: &[CubicSegment]) -> Vec<(f64, f64)> {
    const SAMPLES_PER_SEGMENT: usize = 4;

    let mut samples = Vec::with_capacity(run.len() * SAMPLES_PER_SEGMENT + 1);
    samples.push(run[0].start);

    for cubic in run {
        for step in 1..=SAMPLES_PER_SEGMENT {
            let parameter = step as f64 / SAMPLES_PER_SEGMENT as f64;
            samples.push(cubic_point(*cubic, parameter));
        }
    }

    dedup_nearby_points(samples)
}

pub(crate) fn cubic_runs_are_close(
    source_samples: &[(f64, f64)],
    fitted: &[CubicSegment],
    tolerance: f64,
) -> bool {
    let tolerance_squared = tolerance * tolerance;

    source_samples
        .iter()
        .all(|sample| distance_squared_to_cubic_segments(*sample, fitted) <= tolerance_squared)
        && fitted.iter().all(|cubic| {
            (1..16).all(|step| {
                let point = cubic_point(*cubic, step as f64 / 16.0);
                distance_squared_to_polyline(point, source_samples).0 <= tolerance_squared
            })
        })
}

pub(crate) fn svg_segments_are_all_cubic(segments: &[SvgPathSegment]) -> bool {
    segments
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
}

pub(crate) fn distance_squared_to_cubic_segments(
    point: (f64, f64),
    segments: &[CubicSegment],
) -> f64 {
    let mut best = f64::INFINITY;

    for segment in segments {
        for step in 0..=32 {
            let candidate = cubic_point(*segment, step as f64 / 32.0);
            best = best.min(distance_squared_float(point, candidate));
        }
    }

    best
}

pub(crate) fn dedup_nearby_points(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut deduped = Vec::with_capacity(points.len());

    for point in points {
        if deduped
            .last()
            .is_none_or(|previous| distance_squared_float(*previous, point) > 1.0e-12)
        {
            deduped.push(point);
        }
    }

    deduped
}

pub(crate) fn fit_open_cubic_segments_raw(
    points: &[(f64, f64)],
    max_error_squared: f64,
    segments: &mut Vec<CubicSegment>,
) {
    if points.len() < 2 {
        return;
    }

    let left_tangent = unit_vector(subtract(points[1], points[0]));
    let right_tangent = unit_vector(subtract(points[points.len() - 2], points[points.len() - 1]));

    fit_cubic_recursive(
        points,
        left_tangent,
        right_tangent,
        max_error_squared,
        segments,
    );
}

pub(crate) fn edge_midpoint(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

pub(crate) fn potrace_curve_alpha(
    previous: (f64, f64),
    vertex: (f64, f64),
    next: (f64, f64),
) -> f64 {
    let incoming_segment = subtract(vertex, previous);
    let outgoing_segment = subtract(next, vertex);
    let incoming_length = distance(vertex, previous);
    let outgoing_length = distance(next, vertex);
    let incoming = unit_vector(incoming_segment);
    let outgoing = unit_vector(outgoing_segment);

    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return 0.0;
    }

    let turn = dot(incoming, outgoing).clamp(-1.0, 1.0).acos();
    let base_alpha = (4.0 / 3.0) * (turn / 4.0).tan();
    base_alpha * incoming_length.min(outgoing_length).sqrt()
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CubicSegment {
    pub(crate) start: (f64, f64),
    pub(crate) control1: (f64, f64),
    pub(crate) control2: (f64, f64),
    pub(crate) end: (f64, f64),
}

pub(crate) fn cubic_chord_length(cubic: CubicSegment) -> f64 {
    (cubic.end.0 - cubic.start.0).hypot(cubic.end.1 - cubic.start.1)
}

pub(crate) fn cubic_chord_vector(cubic: CubicSegment) -> (f64, f64) {
    (cubic.end.0 - cubic.start.0, cubic.end.1 - cubic.start.1)
}

pub(crate) fn cubic_bounds_diagonal(cubic: CubicSegment) -> f64 {
    let min_x = cubic
        .start
        .0
        .min(cubic.control1.0)
        .min(cubic.control2.0)
        .min(cubic.end.0);
    let max_x = cubic
        .start
        .0
        .max(cubic.control1.0)
        .max(cubic.control2.0)
        .max(cubic.end.0);
    let min_y = cubic
        .start
        .1
        .min(cubic.control1.1)
        .min(cubic.control2.1)
        .min(cubic.end.1);
    let max_y = cubic
        .start
        .1
        .max(cubic.control1.1)
        .max(cubic.control2.1)
        .max(cubic.end.1);

    (max_x - min_x).hypot(max_y - min_y)
}

pub(crate) fn cubic_chord_deviation(cubic: CubicSegment) -> f64 {
    distance_squared_to_segment(cubic.control1, cubic.start, cubic.end)
        .0
        .max(distance_squared_to_segment(cubic.control2, cubic.start, cubic.end).0)
        .sqrt()
}

pub(crate) fn vector_turn_angle(a: (f64, f64), b: (f64, f64)) -> f64 {
    let a = unit_vector(a);
    let b = unit_vector(b);

    if vector_length_squared(a) <= f64::EPSILON || vector_length_squared(b) <= f64::EPSILON {
        0.0
    } else {
        dot(a, b).clamp(-1.0, 1.0).acos()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FloatBounds {
    pub(crate) min_x: f64,
    pub(crate) max_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_y: f64,
}

impl FloatBounds {
    pub(crate) fn from_points(points: &[(f64, f64)]) -> Option<Self> {
        let (first, rest) = points.split_first()?;
        let mut bounds = Self {
            min_x: first.0,
            max_x: first.0,
            min_y: first.1,
            max_y: first.1,
        };

        for point in rest {
            bounds.min_x = bounds.min_x.min(point.0);
            bounds.max_x = bounds.max_x.max(point.0);
            bounds.min_y = bounds.min_y.min(point.1);
            bounds.max_y = bounds.max_y.max(point.1);
        }

        Some(bounds)
    }

    pub(crate) fn clamp(self, point: (f64, f64)) -> (f64, f64) {
        (
            point.0.clamp(self.min_x, self.max_x),
            point.1.clamp(self.min_y, self.max_y),
        )
    }
}

pub(crate) fn fit_closed_cubic_segments(points: &[(f64, f64)], error: f64) -> Vec<CubicSegment> {
    if points.len() < 2 {
        return Vec::new();
    }

    let breakpoints = fit_breakpoints(points);
    let mut segments = Vec::new();

    for index in 0..breakpoints.len() {
        let start = breakpoints[index];
        let end = breakpoints[(index + 1) % breakpoints.len()];
        let arc = closed_arc_points(points, start, end);
        fit_open_cubic_segments(&arc, error * error, &mut segments);
    }

    segments
}

pub(crate) fn fit_breakpoints(points: &[(f64, f64)]) -> Vec<usize> {
    let mut breakpoints = vec![0];

    for index in 1..points.len() {
        if is_sharp_fit_corner(points, index) {
            breakpoints.push(index);
        }
    }

    if breakpoints.len() < 2 {
        return even_fit_breakpoints(points.len());
    }

    breakpoints
}

pub(crate) fn even_fit_breakpoints(point_count: usize) -> Vec<usize> {
    let breakpoint_count = point_count.min(4);
    let mut breakpoints = Vec::with_capacity(breakpoint_count);

    for index in 0..breakpoint_count {
        let breakpoint = index * point_count / breakpoint_count;
        if breakpoints.last() != Some(&breakpoint) {
            breakpoints.push(breakpoint);
        }
    }

    breakpoints
}

pub(crate) fn is_sharp_fit_corner(points: &[(f64, f64)], index: usize) -> bool {
    // Smooth contours often contain small alternating turns from raster sampling.
    // Average a few neighboring segments before deciding whether a point is a
    // structural corner that should split a cubic fitting run.
    const CORNER_COSINE_THRESHOLD: f64 = 0.0;

    let steps = fit_corner_tangent_steps(points.len());
    let incoming = averaged_fit_tangent(points, index, FitTangentDirection::Incoming, steps);
    let outgoing = averaged_fit_tangent(points, index, FitTangentDirection::Outgoing, steps);

    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return false;
    }

    let incoming = unit_vector(incoming);
    let outgoing = unit_vector(outgoing);

    dot(incoming, outgoing) <= CORNER_COSINE_THRESHOLD
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum FitTangentDirection {
    Incoming,
    Outgoing,
}

pub(crate) fn fit_corner_tangent_steps(point_count: usize) -> usize {
    match point_count {
        0..=7 => 1,
        8..=17 => 2,
        _ => 3,
    }
}

pub(crate) fn averaged_fit_tangent(
    points: &[(f64, f64)],
    index: usize,
    direction: FitTangentDirection,
    steps: usize,
) -> (f64, f64) {
    let mut vector = (0.0, 0.0);
    let mut current = index;

    for _ in 0..steps {
        match direction {
            FitTangentDirection::Incoming => {
                let previous = (current + points.len() - 1) % points.len();
                vector = add(vector, subtract(points[current], points[previous]));
                current = previous;
            }
            FitTangentDirection::Outgoing => {
                let next = (current + 1) % points.len();
                vector = add(vector, subtract(points[next], points[current]));
                current = next;
            }
        }
    }

    vector
}

pub(crate) fn closed_arc_points(
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

pub(crate) fn fit_open_cubic_segments(
    points: &[(f64, f64)],
    max_error_squared: f64,
    segments: &mut Vec<CubicSegment>,
) {
    if points.len() < 2 {
        return;
    }

    let points = smooth_open_fit_points(points);
    let left_tangent = unit_vector(subtract(points[1], points[0]));
    let right_tangent = unit_vector(subtract(points[points.len() - 2], points[points.len() - 1]));

    fit_cubic_recursive(
        &points,
        left_tangent,
        right_tangent,
        max_error_squared,
        segments,
    );
}

pub(crate) fn smooth_open_fit_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() < 9 {
        return points.to_vec();
    }

    let mut smoothed = points.to_vec();
    smooth_open_laplacian(&mut smoothed, 0.25);
    smooth_open_laplacian(&mut smoothed, -0.265);
    smoothed
}

pub(crate) fn smooth_open_laplacian(points: &mut [(f64, f64)], amount: f64) {
    let original = points.to_vec();

    for index in 1..points.len() - 1 {
        let midpoint = interpolate(original[index - 1], original[index + 1], 0.5);
        points[index] = add(
            original[index],
            scale(subtract(midpoint, original[index]), amount),
        );
    }
}

pub(crate) fn fit_cubic_recursive(
    points: &[(f64, f64)],
    left_tangent: (f64, f64),
    right_tangent: (f64, f64),
    max_error_squared: f64,
    segments: &mut Vec<CubicSegment>,
) {
    let bounds = FloatBounds::from_points(points).expect("fit segment should contain points");

    if points.len() == 2 {
        segments.push(clamp_cubic(
            linear_cubic(points[0], points[1], left_tangent, right_tangent),
            bounds,
        ));
        return;
    }

    let parameters = chord_length_parameters(points);
    let mut cubic = clamp_cubic(
        generate_cubic(points, &parameters, left_tangent, right_tangent),
        bounds,
    );
    let (mut error_squared, mut split_index) = max_cubic_error(points, &parameters, cubic);

    if error_squared <= max_error_squared {
        segments.push(cubic);
        return;
    }

    let mut refined_parameters = parameters;

    for _ in 0..4 {
        let Some(next_parameters) = reparameterize(points, &refined_parameters, cubic) else {
            break;
        };

        refined_parameters = next_parameters;
        let refined_cubic = clamp_cubic(
            generate_cubic(points, &refined_parameters, left_tangent, right_tangent),
            bounds,
        );
        let (refined_error, refined_split_index) =
            max_cubic_error(points, &refined_parameters, refined_cubic);

        if refined_error < error_squared {
            cubic = refined_cubic;
            error_squared = refined_error;
            split_index = refined_split_index;
        }

        if refined_error <= max_error_squared {
            segments.push(refined_cubic);
            return;
        }
    }

    if split_index == 0 || split_index + 1 >= points.len() {
        segments.push(cubic);
        return;
    }

    let center_tangent = center_tangent(points, split_index);
    fit_cubic_recursive(
        &points[..=split_index],
        left_tangent,
        center_tangent,
        max_error_squared,
        segments,
    );
    fit_cubic_recursive(
        &points[split_index..],
        scale(center_tangent, -1.0),
        right_tangent,
        max_error_squared,
        segments,
    );
}

pub(crate) fn clamp_cubic(cubic: CubicSegment, bounds: FloatBounds) -> CubicSegment {
    CubicSegment {
        start: cubic.start,
        control1: bounds.clamp(cubic.control1),
        control2: bounds.clamp(cubic.control2),
        end: cubic.end,
    }
}

pub(crate) fn linear_cubic(
    start: (f64, f64),
    end: (f64, f64),
    left_tangent: (f64, f64),
    right_tangent: (f64, f64),
) -> CubicSegment {
    let distance = distance(start, end) / 3.0;

    CubicSegment {
        start,
        control1: add(start, scale(left_tangent, distance)),
        control2: add(end, scale(right_tangent, distance)),
        end,
    }
}

pub(crate) fn chord_length_parameters(points: &[(f64, f64)]) -> Vec<f64> {
    let mut parameters = Vec::with_capacity(points.len());
    parameters.push(0.0);

    for index in 1..points.len() {
        parameters.push(parameters[index - 1] + distance(points[index - 1], points[index]));
    }

    let total = *parameters.last().unwrap_or(&0.0);
    if total <= f64::EPSILON {
        return (0..points.len())
            .map(|index| index as f64 / (points.len().saturating_sub(1)).max(1) as f64)
            .collect();
    }

    parameters
        .iter()
        .map(|parameter| parameter / total)
        .collect()
}

pub(crate) fn generate_cubic(
    points: &[(f64, f64)],
    parameters: &[f64],
    left_tangent: (f64, f64),
    right_tangent: (f64, f64),
) -> CubicSegment {
    let start = points[0];
    let end = points[points.len() - 1];
    let mut c00: f64 = 0.0;
    let mut c01: f64 = 0.0;
    let mut c11: f64 = 0.0;
    let mut x0: f64 = 0.0;
    let mut x1: f64 = 0.0;

    for (point, parameter) in points.iter().zip(parameters) {
        let (b0, b1, b2, b3) = bernstein3(*parameter);
        let a1 = scale(left_tangent, b1);
        let a2 = scale(right_tangent, b2);
        let endpoint_blend = add(scale(start, b0 + b1), scale(end, b2 + b3));
        let target = subtract(*point, endpoint_blend);

        c00 += dot(a1, a1);
        c01 += dot(a1, a2);
        c11 += dot(a2, a2);
        x0 += dot(a1, target);
        x1 += dot(a2, target);
    }

    let determinant = c00 * c11 - c01 * c01;
    let segment_length = distance(start, end);
    let epsilon = 1.0e-6 * segment_length;

    let (alpha1, alpha2) = if determinant.abs() > f64::EPSILON {
        (
            (x0 * c11 - x1 * c01) / determinant,
            (c00 * x1 - c01 * x0) / determinant,
        )
    } else {
        (segment_length / 3.0, segment_length / 3.0)
    };

    if alpha1 <= epsilon || alpha2 <= epsilon {
        return linear_cubic(start, end, left_tangent, right_tangent);
    }

    CubicSegment {
        start,
        control1: add(start, scale(left_tangent, alpha1)),
        control2: add(end, scale(right_tangent, alpha2)),
        end,
    }
}

pub(crate) fn max_cubic_error(
    points: &[(f64, f64)],
    parameters: &[f64],
    cubic: CubicSegment,
) -> (f64, usize) {
    let mut max_error = 0.0;
    let mut split_index = points.len() / 2;

    for index in 1..points.len() - 1 {
        let point = cubic_point(cubic, parameters[index]);
        let error = distance_squared_float(point, points[index]);

        if error > max_error {
            max_error = error;
            split_index = index;
        }
    }

    let sample_count = ((points.len() - 1) * 4).clamp(8, 32);
    for sample in 1..sample_count {
        let parameter = sample as f64 / sample_count as f64;
        let point = cubic_point(cubic, parameter);
        let (error, candidate_split_index) = distance_squared_to_polyline(point, points);

        if error > max_error {
            max_error = error;
            split_index = candidate_split_index;
        }
    }

    (max_error, split_index)
}

pub(crate) fn distance_squared_to_polyline(
    point: (f64, f64),
    points: &[(f64, f64)],
) -> (f64, usize) {
    let mut min_error = f64::INFINITY;
    let mut split_index = points.len() / 2;

    for index in 0..points.len() - 1 {
        let (error, projection) =
            distance_squared_to_segment(point, points[index], points[index + 1]);

        if error < min_error {
            min_error = error;
            split_index = if projection < 0.5 { index } else { index + 1 };
        }
    }

    (min_error, split_index.clamp(1, points.len() - 2))
}

pub(crate) fn distance_squared_to_segment(
    point: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
) -> (f64, f64) {
    let segment = subtract(end, start);
    let length_squared = dot(segment, segment);

    if length_squared <= f64::EPSILON {
        return (distance_squared_float(point, start), 0.0);
    }

    let projection = (dot(subtract(point, start), segment) / length_squared).clamp(0.0, 1.0);
    let closest = add(start, scale(segment, projection));

    (distance_squared_float(point, closest), projection)
}

pub(crate) fn reparameterize(
    points: &[(f64, f64)],
    parameters: &[f64],
    cubic: CubicSegment,
) -> Option<Vec<f64>> {
    let mut refined_parameters = Vec::with_capacity(parameters.len());

    for (point, parameter) in points.iter().zip(parameters) {
        refined_parameters.push(newton_raphson_root_find(cubic, *point, *parameter));
    }

    if refined_parameters
        .windows(2)
        .all(|window| window[0] < window[1])
    {
        Some(refined_parameters)
    } else {
        None
    }
}

pub(crate) fn newton_raphson_root_find(
    cubic: CubicSegment,
    point: (f64, f64),
    parameter: f64,
) -> f64 {
    let curve_point = cubic_point(cubic, parameter);
    let first_derivative = cubic_derivative_point(cubic, parameter);
    let second_derivative = cubic_second_derivative_point(cubic, parameter);
    let difference = subtract(curve_point, point);
    let numerator = dot(difference, first_derivative);
    let denominator = dot(first_derivative, first_derivative) + dot(difference, second_derivative);

    if denominator.abs() <= f64::EPSILON {
        return parameter;
    }

    (parameter - numerator / denominator).clamp(0.0, 1.0)
}

pub(crate) fn center_tangent(points: &[(f64, f64)], index: usize) -> (f64, f64) {
    let previous = points[index - 1];
    let next = points[index + 1];
    unit_vector(subtract(previous, next))
}

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
