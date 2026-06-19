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

pub(crate) fn cleanup_potrace_start(start: (f64, f64), segments: &[SvgPathSegment]) -> (f64, f64) {
    segments.first().map_or(start, |segment| segment.start())
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

pub(crate) fn cleanup_potrace_segments(
    segments: Vec<SvgPathSegment>,
    max_linear_deviation: f64,
) -> Vec<SvgPathSegment> {
    let optimized = prune_tiny_potrace_curve_segments(segments);
    let optimized = regularize_potrace_orthogonal_corners(optimized);
    let optimized = demote_nearly_linear_potrace_cubics(optimized, max_linear_deviation);
    let optimized = snap_near_axis_potrace_lines(optimized);
    merge_collinear_potrace_lines(optimized)
}

pub(crate) fn snap_near_axis_potrace_lines(segments: Vec<SvgPathSegment>) -> Vec<SvgPathSegment> {
    const MAX_AXIS_DRIFT: f64 = 0.75;

    if segments.len() < 2 {
        return segments;
    }

    let mut nodes = Vec::with_capacity(segments.len() + 1);
    nodes.push(segments[0].start());
    nodes.extend(segments.iter().map(|segment| segment.end()));

    let mut x_constraints: Vec<(f64, usize)> = vec![(0.0, 0); nodes.len()];
    let mut y_constraints: Vec<(f64, usize)> = vec![(0.0, 0); nodes.len()];

    for (index, segment) in segments.iter().enumerate() {
        let SvgPathSegment::Line { start, end } = *segment else {
            continue;
        };

        let dx = (end.0 - start.0).abs();
        let dy = (end.1 - start.1).abs();
        if dx <= MAX_AXIS_DRIFT && dy > MAX_AXIS_DRIFT {
            let snapped_x = (start.0 + end.0) / 2.0;
            add_axis_constraint(&mut x_constraints[index], snapped_x);
            add_axis_constraint(&mut x_constraints[index + 1], snapped_x);
        } else if dy <= MAX_AXIS_DRIFT && dx > MAX_AXIS_DRIFT {
            let snapped_y = (start.1 + end.1) / 2.0;
            add_axis_constraint(&mut y_constraints[index], snapped_y);
            add_axis_constraint(&mut y_constraints[index + 1], snapped_y);
        }
    }

    if compact_segments_are_closed(nodes[0], &segments) {
        merge_closed_axis_constraints(&mut x_constraints);
        merge_closed_axis_constraints(&mut y_constraints);
    }

    let mut changed = false;
    for index in 0..nodes.len() {
        if x_constraints[index].1 > 0 {
            let snapped = x_constraints[index].0 / x_constraints[index].1 as f64;
            changed |= (nodes[index].0 - snapped).abs() > 1.0e-9;
            nodes[index].0 = snapped;
        }
        if y_constraints[index].1 > 0 {
            let snapped = y_constraints[index].0 / y_constraints[index].1 as f64;
            changed |= (nodes[index].1 - snapped).abs() > 1.0e-9;
            nodes[index].1 = snapped;
        }
    }

    if !changed {
        return segments;
    }

    segments
        .into_iter()
        .enumerate()
        .map(|(index, segment)| snap_segment_endpoints(segment, nodes[index], nodes[index + 1]))
        .collect()
}

pub(crate) fn add_axis_constraint(constraint: &mut (f64, usize), value: f64) {
    constraint.0 += value;
    constraint.1 += 1;
}

pub(crate) fn merge_closed_axis_constraints(constraints: &mut [(f64, usize)]) {
    if constraints.len() < 2 {
        return;
    }

    let last = constraints.len() - 1;
    let sum = constraints[0].0 + constraints[last].0;
    let count = constraints[0].1 + constraints[last].1;
    constraints[0] = (sum, count);
    constraints[last] = (sum, count);
}

pub(crate) fn snap_segment_endpoints(
    segment: SvgPathSegment,
    snapped_start: (f64, f64),
    snapped_end: (f64, f64),
) -> SvgPathSegment {
    match segment {
        SvgPathSegment::Line { .. } => SvgPathSegment::Line {
            start: snapped_start,
            end: snapped_end,
        },
        SvgPathSegment::Cubic(cubic) => {
            let start_delta = subtract(snapped_start, cubic.start);
            let end_delta = subtract(snapped_end, cubic.end);
            SvgPathSegment::Cubic(CubicSegment {
                start: snapped_start,
                control1: add(cubic.control1, start_delta),
                control2: add(cubic.control2, end_delta),
                end: snapped_end,
            })
        }
    }
}

pub(crate) fn merge_collinear_potrace_lines(segments: Vec<SvgPathSegment>) -> Vec<SvgPathSegment> {
    if segments.len() < 2 {
        return segments;
    }

    let mut merged: Vec<SvgPathSegment> = Vec::with_capacity(segments.len());

    for segment in segments {
        if let Some(previous) = merged.last_mut() {
            if let Some(combined) = merge_collinear_potrace_line_pair(*previous, segment) {
                *previous = combined;
                continue;
            }
        }

        merged.push(segment);
    }

    merged
}

pub(crate) fn merge_collinear_potrace_line_pair(
    previous: SvgPathSegment,
    current: SvgPathSegment,
) -> Option<SvgPathSegment> {
    let (
        SvgPathSegment::Line { start, end: middle },
        SvgPathSegment::Line {
            start: current_start,
            end,
        },
    ) = (previous, current)
    else {
        return None;
    };

    if distance_squared_float(middle, current_start) > 1.0e-12 {
        return None;
    }

    let first = subtract(middle, start);
    let second = subtract(end, middle);
    if vector_length_squared(first) <= f64::EPSILON
        || vector_length_squared(second) <= f64::EPSILON
        || cross(first, second).abs() > 1.0e-9
        || dot(first, second) < 0.0
    {
        return None;
    }

    Some(SvgPathSegment::Line { start, end })
}

pub(crate) fn demote_nearly_linear_potrace_cubics(
    segments: Vec<SvgPathSegment>,
    max_linear_deviation: f64,
) -> Vec<SvgPathSegment> {
    segments
        .into_iter()
        .map(|segment| match segment {
            SvgPathSegment::Cubic(cubic)
                if potrace_cubic_is_nearly_linear(cubic, max_linear_deviation) =>
            {
                SvgPathSegment::Line {
                    start: cubic.start,
                    end: cubic.end,
                }
            }
            segment => segment,
        })
        .collect()
}

pub(crate) const STRICT_POTRACE_LINEAR_DEVIATION: f64 = 0.25;
pub(crate) const PIXEL_POTRACE_LINEAR_DEVIATION: f64 = 1.0;

pub(crate) fn potrace_cubic_is_nearly_linear(
    cubic: CubicSegment,
    max_linear_deviation: f64,
) -> bool {
    const MIN_LINEAR_LENGTH: f64 = 16.0;

    cubic_chord_length(cubic) >= MIN_LINEAR_LENGTH
        && cubic_chord_deviation(cubic) <= max_linear_deviation
}

pub(crate) fn prune_tiny_potrace_curve_segments(
    segments: Vec<SvgPathSegment>,
) -> Vec<SvgPathSegment> {
    if segments.len() < 5 {
        return segments;
    }

    let mut pruned = Vec::with_capacity(segments.len());
    for index in 0..segments.len() {
        if potrace_segment_is_tiny_spike(&segments, index) {
            continue;
        }

        pruned.push(segments[index]);
    }

    if pruned.len() >= 3 && pruned.len() < segments.len() {
        pruned
    } else {
        segments
    }
}

pub(crate) fn potrace_segment_is_tiny_spike(segments: &[SvgPathSegment], index: usize) -> bool {
    const TINY_CHORD_LENGTH: f64 = 2.1;
    const TINY_BOUNDS_DIAGONAL: f64 = 2.1;
    const MIN_NEIGHBOR_CHORD_LENGTH: f64 = 4.0;

    if segments.len() < 3 {
        return false;
    }

    let previous_index = (index + segments.len() - 1) % segments.len();
    let next_index = (index + 1) % segments.len();
    let (
        SvgPathSegment::Cubic(previous),
        SvgPathSegment::Cubic(current),
        SvgPathSegment::Cubic(next),
    ) = (
        segments[previous_index],
        segments[index],
        segments[next_index],
    )
    else {
        return false;
    };

    if distance_squared_float(previous.end, current.start) > 1.0e-9
        || distance_squared_float(current.end, next.start) > 1.0e-9
    {
        return false;
    }

    cubic_chord_length(current) <= TINY_CHORD_LENGTH
        && cubic_bounds_diagonal(current) <= TINY_BOUNDS_DIAGONAL
        && cubic_chord_length(previous) >= MIN_NEIGHBOR_CHORD_LENGTH
        && cubic_chord_length(next) >= MIN_NEIGHBOR_CHORD_LENGTH
        && potrace_segment_has_spike_turn(previous, current, next)
}

pub(crate) fn potrace_segment_has_spike_turn(
    previous: CubicSegment,
    current: CubicSegment,
    next: CubicSegment,
) -> bool {
    const MIN_SPIKE_TURN_RADIANS: f64 = 1.0;
    const MIN_BRIDGED_TURN_RADIANS: f64 = 0.35;

    let previous_vector = cubic_chord_vector(previous);
    let current_vector = cubic_chord_vector(current);
    let next_vector = cubic_chord_vector(next);
    let entry_turn = vector_turn_angle(previous_vector, current_vector);
    let exit_turn = vector_turn_angle(current_vector, next_vector);
    let bridged_turn = vector_turn_angle(previous_vector, next_vector);

    entry_turn.max(exit_turn) >= MIN_SPIKE_TURN_RADIANS
        && (bridged_turn >= MIN_BRIDGED_TURN_RADIANS
            || (entry_turn >= MIN_SPIKE_TURN_RADIANS && exit_turn >= MIN_SPIKE_TURN_RADIANS))
}

pub(crate) fn regularize_potrace_orthogonal_corners(
    segments: Vec<SvgPathSegment>,
) -> Vec<SvgPathSegment> {
    if segments.len() < 5 {
        return segments;
    }

    let mut regularized = Vec::with_capacity(segments.len());
    let mut index = 0usize;
    let mut changed = false;

    while index < segments.len() {
        if let Some(cubic) = regularized_potrace_corner_pair(&segments, index) {
            regularized.push(SvgPathSegment::Cubic(cubic));
            changed = true;
            index += 2;
            continue;
        }

        if let Some(cubic) = regularized_potrace_corner(&segments, index) {
            regularized.push(SvgPathSegment::Cubic(cubic));
            changed = true;
            index += 1;
            continue;
        }

        regularized.push(segments[index]);
        index += 1;
    }

    if changed && regularized.len() >= 3 {
        regularized
    } else {
        segments
    }
}

pub(crate) fn regularized_potrace_corner_pair(
    segments: &[SvgPathSegment],
    index: usize,
) -> Option<CubicSegment> {
    const MAX_LEAD_TURN_RADIANS: f64 = 0.35;

    if index == 0 || index + 2 >= segments.len() {
        return None;
    }

    let (
        SvgPathSegment::Cubic(previous),
        SvgPathSegment::Cubic(lead),
        SvgPathSegment::Cubic(turn),
        SvgPathSegment::Cubic(next),
    ) = (
        segments[index - 1],
        segments[index],
        segments[index + 1],
        segments[index + 2],
    )
    else {
        return None;
    };

    if !potrace_segment_is_straight_edge(previous)
        || !potrace_segment_is_straight_edge(next)
        || !potrace_segment_is_short_straight_lead(lead)
        || !potrace_segment_is_roundable_corner(turn)
    {
        return None;
    }

    let previous_vector = cubic_chord_vector(previous);
    let lead_vector = cubic_chord_vector(lead);
    let next_vector = cubic_chord_vector(next);
    if vector_turn_angle(previous_vector, lead_vector) > MAX_LEAD_TURN_RADIANS
        || !vectors_are_roughly_orthogonal(previous_vector, next_vector)
    {
        return None;
    }

    let candidate = tangent_corner_cubic(lead.start, turn.end, previous_vector, next_vector)?;
    potrace_regularized_corner_is_close(&[lead, turn], candidate, 5.0).then_some(candidate)
}

pub(crate) fn regularized_potrace_corner(
    segments: &[SvgPathSegment],
    index: usize,
) -> Option<CubicSegment> {
    if index == 0 || index + 1 >= segments.len() {
        return None;
    }

    let (
        SvgPathSegment::Cubic(previous),
        SvgPathSegment::Cubic(current),
        SvgPathSegment::Cubic(next),
    ) = (segments[index - 1], segments[index], segments[index + 1])
    else {
        return None;
    };

    if !potrace_segment_is_straight_edge(previous)
        || !potrace_segment_is_straight_edge(next)
        || !potrace_segment_is_roundable_corner(current)
        || !vectors_are_roughly_orthogonal(cubic_chord_vector(previous), cubic_chord_vector(next))
    {
        return None;
    }

    let candidate = tangent_corner_cubic(
        current.start,
        current.end,
        cubic_chord_vector(previous),
        cubic_chord_vector(next),
    )?;
    potrace_regularized_corner_is_close(&[current], candidate, 3.5).then_some(candidate)
}

pub(crate) fn potrace_segment_is_straight_edge(cubic: CubicSegment) -> bool {
    const MIN_STRAIGHT_LENGTH: f64 = 40.0;
    const MAX_STRAIGHT_DEVIATION: f64 = 1.5;

    cubic_chord_length(cubic) >= MIN_STRAIGHT_LENGTH
        && cubic_chord_deviation(cubic) <= MAX_STRAIGHT_DEVIATION
}

pub(crate) fn potrace_segment_is_short_straight_lead(cubic: CubicSegment) -> bool {
    const MIN_LEAD_LENGTH: f64 = 4.0;
    const MAX_LEAD_LENGTH: f64 = 32.0;
    const MAX_LEAD_DEVIATION: f64 = 1.5;

    let length = cubic_chord_length(cubic);
    (MIN_LEAD_LENGTH..=MAX_LEAD_LENGTH).contains(&length)
        && cubic_chord_deviation(cubic) <= MAX_LEAD_DEVIATION
}

pub(crate) fn potrace_segment_is_roundable_corner(cubic: CubicSegment) -> bool {
    const MIN_CORNER_LENGTH: f64 = 6.0;
    const MAX_CORNER_LENGTH: f64 = 36.0;
    const MIN_CORNER_DEVIATION: f64 = 1.5;

    let length = cubic_chord_length(cubic);
    (MIN_CORNER_LENGTH..=MAX_CORNER_LENGTH).contains(&length)
        && cubic_chord_deviation(cubic) >= MIN_CORNER_DEVIATION
}

pub(crate) fn vectors_are_roughly_orthogonal(a: (f64, f64), b: (f64, f64)) -> bool {
    const MIN_ORTHOGONAL_TURN: f64 = 1.0;
    const MAX_ORTHOGONAL_TURN: f64 = 2.15;

    let turn = vector_turn_angle(a, b);
    (MIN_ORTHOGONAL_TURN..=MAX_ORTHOGONAL_TURN).contains(&turn)
}

pub(crate) fn tangent_corner_cubic(
    start: (f64, f64),
    end: (f64, f64),
    incoming: (f64, f64),
    outgoing: (f64, f64),
) -> Option<CubicSegment> {
    const CIRCLE_ARC_KAPPA: f64 = 0.552_284_749_830_793_6;
    const MIN_HANDLE_LENGTH: f64 = 2.0;

    let incoming = unit_vector(incoming);
    let outgoing = unit_vector(outgoing);
    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return None;
    }

    let delta = subtract(end, start);
    let incoming_projection = dot(delta, incoming);
    let outgoing_projection = dot(delta, outgoing);
    if incoming_projection <= 0.0 || outgoing_projection <= 0.0 {
        return None;
    }

    let handle = incoming_projection.min(outgoing_projection) * CIRCLE_ARC_KAPPA;
    if handle < MIN_HANDLE_LENGTH {
        return None;
    }

    Some(CubicSegment {
        start,
        control1: add(start, scale(incoming, handle)),
        control2: subtract(end, scale(outgoing, handle)),
        end,
    })
}

pub(crate) fn potrace_regularized_corner_is_close(
    source: &[CubicSegment],
    candidate: CubicSegment,
    tolerance: f64,
) -> bool {
    let samples = sample_cubic_run(source);
    cubic_runs_are_close(&samples, &[candidate], tolerance)
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
