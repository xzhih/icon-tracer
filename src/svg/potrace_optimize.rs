use super::*;

pub(crate) fn optimize_potrace_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
    opt_tolerance: f64,
    max_linear_deviation: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    if segments.len() < 3 {
        return (start, segments.to_vec());
    }

    if segments
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        let optimized = optimize_closed_potrace_curve_run(
            &segments
                .iter()
                .filter_map(|segment| match segment {
                    SvgPathSegment::Cubic(cubic) => Some(*cubic),
                    SvgPathSegment::Line { .. } => None,
                })
                .collect::<Vec<_>>(),
            opt_tolerance,
        );

        return finish_potrace_segments(start, optimized, opt_tolerance, max_linear_deviation);
    }

    let (start, optimized) = optimize_mixed_potrace_curve_runs_once(start, segments, opt_tolerance);
    finish_potrace_segments(start, optimized, opt_tolerance, max_linear_deviation)
}

pub(crate) fn finish_potrace_segments(
    start: (f64, f64),
    segments: Vec<SvgPathSegment>,
    opt_tolerance: f64,
    max_linear_deviation: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let optimized = cleanup_potrace_segments(segments, max_linear_deviation);
    let start = cleanup_potrace_start(start, &optimized);
    if optimized
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        return (start, optimized);
    }

    let (start, optimized) =
        optimize_mixed_potrace_curve_runs_once(start, &optimized, opt_tolerance);
    let optimized = cleanup_potrace_segments(optimized, max_linear_deviation);
    let start = cleanup_potrace_start(start, &optimized);
    (start, optimized)
}

pub(crate) fn optimize_mixed_potrace_curve_runs_once(
    start: (f64, f64),
    segments: &[SvgPathSegment],
    opt_tolerance: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let rotated = rotate_potrace_segments_after_last_line(segments);
    let start = rotated.first().map_or(start, |segment| segment.start());
    let mut optimized = Vec::new();
    let mut curve_run = Vec::new();

    for segment in rotated {
        match segment {
            SvgPathSegment::Cubic(cubic) => curve_run.push(cubic),
            SvgPathSegment::Line { .. } => {
                flush_potrace_curve_run(&mut optimized, &mut curve_run, opt_tolerance);
                optimized.push(segment);
            }
        }
    }

    flush_potrace_curve_run(&mut optimized, &mut curve_run, opt_tolerance);
    (start, optimized)
}

pub(crate) fn rotate_potrace_segments_after_last_line(
    segments: &[SvgPathSegment],
) -> Vec<SvgPathSegment> {
    let Some(line_index) = segments
        .iter()
        .rposition(|segment| matches!(segment, SvgPathSegment::Line { .. }))
    else {
        return segments.to_vec();
    };

    let start = (line_index + 1) % segments.len();
    segments[start..]
        .iter()
        .chain(segments[..start].iter())
        .copied()
        .collect()
}

pub(crate) fn optimize_closed_potrace_curve_run(
    run: &[CubicSegment],
    opt_tolerance: f64,
) -> Vec<SvgPathSegment> {
    const CLOSED_SPLITS: usize = 4;

    if run.len() < CLOSED_SPLITS * 2 {
        return run.iter().copied().map(SvgPathSegment::Cubic).collect();
    }

    let mut optimized = Vec::new();

    for split in 0..CLOSED_SPLITS {
        let start = split * run.len() / CLOSED_SPLITS;
        let end = (split + 1) * run.len() / CLOSED_SPLITS;
        append_optimized_potrace_curve_run(&mut optimized, &run[start..end], opt_tolerance);
    }

    optimized
}

pub(crate) fn flush_potrace_curve_run(
    output: &mut Vec<SvgPathSegment>,
    run: &mut Vec<CubicSegment>,
    opt_tolerance: f64,
) {
    append_optimized_potrace_curve_run(output, run, opt_tolerance);
    run.clear();
}

pub(crate) fn append_optimized_potrace_curve_run(
    output: &mut Vec<SvgPathSegment>,
    run: &[CubicSegment],
    opt_tolerance: f64,
) {
    if run.is_empty() {
        return;
    }

    if run.len() <= 1 {
        output.extend(run.iter().copied().map(SvgPathSegment::Cubic));
        return;
    }

    output.extend(
        optimize_potrace_curve_run_graph(run, opt_tolerance)
            .into_iter()
            .map(SvgPathSegment::Cubic),
    );
}

pub(crate) fn optimize_potrace_curve_run_graph(
    run: &[CubicSegment],
    opt_tolerance: f64,
) -> Vec<CubicSegment> {
    let mut dp: Vec<Option<OpticurveState>> = vec![None; run.len() + 1];
    let mut edges: Vec<Vec<OpticurveEdge>> = vec![Vec::new(); run.len()];
    dp[0] = Some(OpticurveState {
        previous: 0,
        edge_index: 0,
        segments: 0,
        penalty: 0.0,
    });

    for start in 0..run.len() {
        let Some(state) = dp[start] else {
            continue;
        };

        let mut end = start + 1;
        while end <= run.len() {
            let Some(edge) = opticurve_edge(run, start, end, opt_tolerance) else {
                end += 1;
                continue;
            };
            let edge_index = edges[start].len();
            edges[start].push(edge);
            let candidate = OpticurveState {
                previous: start,
                edge_index,
                segments: state.segments + 1,
                penalty: state.penalty + edge.penalty,
            };

            if dp[end].is_none_or(|best| opticurve_state_is_better(candidate, best)) {
                dp[end] = Some(candidate);
            }

            end += 1;
        }
    }

    let Some(_) = dp[run.len()] else {
        return run.to_vec();
    };

    let mut merged = Vec::new();
    let mut cursor = run.len();

    while cursor != 0 {
        let state = dp[cursor].expect("opticurve cursor should be reachable");
        let edge = edges[state.previous][state.edge_index];
        merged.push(edge.cubic);
        cursor = state.previous;
    }

    merged.reverse();

    if merged.len() <= run.len() {
        merged
    } else {
        run.to_vec()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OpticurveState {
    previous: usize,
    edge_index: usize,
    segments: usize,
    penalty: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OpticurveEdge {
    cubic: CubicSegment,
    penalty: f64,
}

pub(crate) fn opticurve_state_is_better(candidate: OpticurveState, best: OpticurveState) -> bool {
    candidate.segments < best.segments
        || (candidate.segments == best.segments && candidate.penalty < best.penalty)
}

pub(crate) fn opticurve_edge(
    run: &[CubicSegment],
    start: usize,
    end: usize,
    opt_tolerance: f64,
) -> Option<OpticurveEdge> {
    let opt_tolerance = opt_tolerance.max(0.0);
    if end <= start {
        return None;
    }

    if end == start + 1 {
        return Some(OpticurveEdge {
            cubic: run[start],
            penalty: 0.0,
        });
    }

    if !cubic_run_has_consistent_convexity(&run[start..end]) {
        return None;
    }

    if let Some(edge) = potrace_area_opticurve_edge(run, start, end, opt_tolerance) {
        return Some(edge);
    }

    let samples = sample_cubic_run(&run[start..end]);
    let mut fitted = Vec::new();
    fit_open_cubic_segments_raw(&samples, opt_tolerance * opt_tolerance, &mut fitted);

    if fitted.len() != 1 || !cubic_runs_are_close(&samples, &fitted, opt_tolerance) {
        return None;
    }

    Some(OpticurveEdge {
        cubic: fitted[0],
        penalty: cubic_run_fit_penalty(&samples, fitted[0]),
    })
}

pub(crate) fn cubic_run_has_consistent_convexity(run: &[CubicSegment]) -> bool {
    let mut sign = 0.0_f64;

    for cubic in run {
        let start_tangent = subtract(cubic.control1, cubic.start);
        let end_tangent = subtract(cubic.end, cubic.control2);
        let turn = cross(start_tangent, end_tangent);

        if turn.abs() <= 1.0e-9 {
            continue;
        }

        if sign == 0.0 {
            sign = turn.signum();
        } else if turn.signum() != sign {
            return false;
        }
    }

    true
}

pub(crate) fn cubic_run_fit_penalty(samples: &[(f64, f64)], cubic: CubicSegment) -> f64 {
    samples
        .iter()
        .map(|sample| distance_squared_to_cubic_segments(*sample, &[cubic]))
        .sum()
}

pub(crate) struct ReconstructedPotraceRun {
    vertices: Vec<(f64, f64)>,
    alphas: Vec<f64>,
}

impl ReconstructedPotraceRun {
    fn from_cubics(run: &[CubicSegment]) -> Option<Self> {
        let mut vertices = Vec::with_capacity(run.len());
        let mut alphas = Vec::with_capacity(run.len());

        for cubic in run {
            let vertex = potrace_cubic_vertex(*cubic)?;
            let alpha = potrace_cubic_alpha(*cubic, vertex)?;
            vertices.push(vertex);
            alphas.push(alpha);
        }

        Some(Self { vertices, alphas })
    }
}

pub(crate) fn potrace_area_opticurve_edge(
    run: &[CubicSegment],
    start: usize,
    end: usize,
    opt_tolerance: f64,
) -> Option<OpticurveEdge> {
    if end <= start + 1 {
        return None;
    }

    let reconstructed = ReconstructedPotraceRun::from_cubics(run)?;
    let p0 = run[start].start;
    let p1 = reconstructed.vertices[start];
    let p2 = reconstructed.vertices[end - 1];
    let p3 = run[end - 1].end;
    let area = reconstructed_potrace_curve_area(&reconstructed, run, start, end);
    let a1 = signed_area_twice(p0, p1, p2);
    let a2 = signed_area_twice(p0, p1, p3);
    let a3 = signed_area_twice(p0, p2, p3);
    let a4 = a1 + a3 - a2;
    let t_denominator = a3 - a4;
    let s_denominator = a2 - a1;
    if t_denominator.abs() <= f64::EPSILON || s_denominator.abs() <= f64::EPSILON {
        return None;
    }

    let t = a3 / t_denominator;
    let s = a2 / s_denominator;
    let triangle_area = a2 * t / 2.0;
    if triangle_area.abs() <= f64::EPSILON {
        return None;
    }

    let radicand = 4.0 - area / triangle_area / 0.3;
    if radicand < 0.0 {
        return None;
    }

    let alpha = 2.0 - radicand.sqrt();
    if !alpha.is_finite() {
        return None;
    }

    let candidate = CubicSegment {
        start: p0,
        control1: interpolate(p0, p1, t * alpha),
        control2: interpolate(p3, p2, s * alpha),
        end: p3,
    };
    let penalty =
        potrace_area_opticurve_penalty(&reconstructed, run, start, end, candidate, opt_tolerance)?;

    Some(OpticurveEdge {
        cubic: candidate,
        penalty,
    })
}

pub(crate) fn reconstructed_potrace_curve_area(
    reconstructed: &ReconstructedPotraceRun,
    run: &[CubicSegment],
    start: usize,
    end: usize,
) -> f64 {
    let reference = reconstructed.vertices[0];
    let edge_start = run[start].start;
    let edge_end = run[end - 1].end;
    let mut area = 0.0;

    for index in start..end {
        let previous_end = if index == start {
            edge_start
        } else {
            run[index - 1].end
        };
        let end_point = run[index].end;
        let vertex = reconstructed.vertices[index];
        let alpha = reconstructed.alphas[index];
        area +=
            0.3 * alpha * (4.0 - alpha) * signed_area_twice(previous_end, vertex, end_point) / 2.0;
        area += signed_area_twice(reference, previous_end, end_point) / 2.0;
    }

    area - signed_area_twice(reference, edge_start, edge_end) / 2.0
}

pub(crate) fn potrace_area_opticurve_penalty(
    reconstructed: &ReconstructedPotraceRun,
    run: &[CubicSegment],
    start: usize,
    end: usize,
    candidate: CubicSegment,
    opt_tolerance: f64,
) -> Option<f64> {
    let mut penalty = 0.0;

    for index in start..end - 1 {
        let from = reconstructed.vertices[index];
        let to = reconstructed.vertices[index + 1];
        let parameter = bezier_tangent_parameter(candidate, from, to)?;
        let point = cubic_point(candidate, parameter);
        let length = distance(from, to);
        if length <= f64::EPSILON {
            return None;
        }

        let signed_distance = signed_area_twice(from, to, point) / length;
        if signed_distance.abs() > opt_tolerance {
            return None;
        }
        if dot(subtract(to, from), subtract(point, from)) < 0.0
            || dot(subtract(from, to), subtract(point, to)) < 0.0
        {
            return None;
        }

        penalty += signed_distance * signed_distance;
    }

    let edge_start = run[start].start;
    for index in start..end {
        let previous_end = if index == start {
            edge_start
        } else {
            run[index - 1].end
        };
        let end_point = run[index].end;
        let parameter = bezier_tangent_parameter(candidate, previous_end, end_point)?;
        let point = cubic_point(candidate, parameter);
        let length = distance(previous_end, end_point);
        if length <= f64::EPSILON {
            return None;
        }

        let mut signed_distance = signed_area_twice(previous_end, end_point, point) / length;
        let mut corner_distance =
            signed_area_twice(previous_end, end_point, reconstructed.vertices[index]) / length;
        corner_distance *= 0.75 * reconstructed.alphas[index];
        if corner_distance < 0.0 {
            signed_distance = -signed_distance;
            corner_distance = -corner_distance;
        }

        if signed_distance < corner_distance - opt_tolerance {
            return None;
        }
        if signed_distance < corner_distance {
            let delta = signed_distance - corner_distance;
            penalty += delta * delta;
        }
    }

    Some(penalty)
}

pub(crate) fn potrace_cubic_vertex(cubic: CubicSegment) -> Option<(f64, f64)> {
    let incoming = subtract(cubic.control1, cubic.start);
    let outgoing = subtract(cubic.control2, cubic.end);
    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return None;
    }

    line_intersection(
        FitLine {
            point: cubic.start,
            direction: incoming,
        },
        FitLine {
            point: cubic.end,
            direction: outgoing,
        },
    )
}

pub(crate) fn potrace_cubic_alpha(cubic: CubicSegment, vertex: (f64, f64)) -> Option<f64> {
    let entry_alpha = projected_fraction(cubic.start, vertex, cubic.control1)?;
    let exit_alpha = projected_fraction(cubic.end, vertex, cubic.control2)?;
    let alpha = (entry_alpha + exit_alpha) / 2.0;

    (alpha.is_finite() && alpha > 0.0 && alpha <= 2.0).then_some(alpha)
}

pub(crate) fn projected_fraction(
    start: (f64, f64),
    end: (f64, f64),
    point: (f64, f64),
) -> Option<f64> {
    let vector = subtract(end, start);
    let length_squared = vector_length_squared(vector);
    if length_squared <= f64::EPSILON {
        return None;
    }

    Some(dot(subtract(point, start), vector) / length_squared)
}

pub(crate) fn bezier_tangent_parameter(
    cubic: CubicSegment,
    line_start: (f64, f64),
    line_end: (f64, f64),
) -> Option<f64> {
    let a = cross_lines(cubic.start, cubic.control1, line_start, line_end);
    let b = cross_lines(cubic.control1, cubic.control2, line_start, line_end);
    let c = cross_lines(cubic.control2, cubic.end, line_start, line_end);
    let quadratic_a = a - 2.0 * b + c;
    let quadratic_b = -2.0 * a + 2.0 * b;
    let quadratic_c = a;
    let discriminant = quadratic_b * quadratic_b - 4.0 * quadratic_a * quadratic_c;

    if quadratic_a.abs() <= f64::EPSILON {
        if quadratic_b.abs() <= f64::EPSILON {
            return None;
        }

        let linear = -quadratic_c / quadratic_b;
        return (0.0..=1.0).contains(&linear).then_some(linear);
    }

    if discriminant < 0.0 {
        return None;
    }

    let root = discriminant.sqrt();
    let first = (-quadratic_b + root) / (2.0 * quadratic_a);
    let second = (-quadratic_b - root) / (2.0 * quadratic_a);

    if (0.0..=1.0).contains(&first) {
        Some(first)
    } else if (0.0..=1.0).contains(&second) {
        Some(second)
    } else {
        None
    }
}

pub(crate) fn cross_lines(
    first_start: (f64, f64),
    first_end: (f64, f64),
    second_start: (f64, f64),
    second_end: (f64, f64),
) -> f64 {
    cross(
        subtract(first_end, first_start),
        subtract(second_end, second_start),
    )
}

pub(crate) fn signed_area_twice(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    cross(subtract(b, a), subtract(c, a))
}
