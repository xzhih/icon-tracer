use super::*;

pub(crate) fn cleanup_potrace_start(start: (f64, f64), segments: &[SvgPathSegment]) -> (f64, f64) {
    segments.first().map_or(start, |segment| segment.start())
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

    let (regularized, changed) = regularize_potrace_orthogonal_corners_linear(&segments);
    if changed {
        return regularized;
    }

    regularize_wrapped_potrace_orthogonal_corner(&segments).unwrap_or(segments)
}

fn regularize_potrace_orthogonal_corners_linear(
    segments: &[SvgPathSegment],
) -> (Vec<SvgPathSegment>, bool) {
    let mut regularized = Vec::with_capacity(segments.len());
    let mut index = 0usize;
    let mut changed = false;

    while index < segments.len() {
        if let Some(cubic) = regularized_potrace_corner_pair(segments, index) {
            regularized.push(SvgPathSegment::Cubic(cubic));
            changed = true;
            index += 2;
            continue;
        }

        if let Some(cubic) = regularized_potrace_corner(segments, index) {
            regularized.push(SvgPathSegment::Cubic(cubic));
            changed = true;
            index += 1;
            continue;
        }

        regularized.push(segments[index]);
        index += 1;
    }

    if changed && regularized.len() >= 3 {
        (regularized, true)
    } else {
        (segments.to_vec(), false)
    }
}

fn regularize_wrapped_potrace_orthogonal_corner(
    segments: &[SvgPathSegment],
) -> Option<Vec<SvgPathSegment>> {
    if !compact_segments_are_closed(segments[0].start(), segments)
        || !segments
            .iter()
            .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        return None;
    }

    for offset in 1..segments.len() {
        let rotated = rotate_segments_at(segments, offset);
        let (regularized, changed) = regularize_potrace_orthogonal_corners_linear(&rotated);
        if changed {
            return Some(regularized);
        }
    }

    None
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
