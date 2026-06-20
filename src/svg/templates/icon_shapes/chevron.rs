use super::chevron_paths::*;
use super::*;

pub(crate) fn fit_closed_chevron_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    const MIN_AXIS: f64 = 24.0;
    const MIN_ASPECT_RATIO: f64 = 0.75;
    const MAX_ASPECT_RATIO: f64 = 1.25;
    const MAX_TEMPLATE_BOUNDARY_ERROR: f64 = 1.0;

    let bounds = FloatBounds::from_points(points)?;
    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;
    if width < MIN_AXIS || height < MIN_AXIS {
        return None;
    }

    let aspect = width / height;
    if !(MIN_ASPECT_RATIO..=MAX_ASPECT_RATIO).contains(&aspect) {
        return None;
    }

    let path = TracePath {
        points: points.to_vec(),
        is_hole: false,
    };
    let candidates = [
        chevron_down_potrace_segments(bounds),
        chevron_up_potrace_segments(bounds),
        chevron_right_potrace_segments(bounds),
        chevron_left_potrace_segments(bounds),
    ];

    candidates
        .into_iter()
        .filter_map(|segments| {
            let candidate = (segments[0].start(), segments.clone());
            let error = pixel_potrace_candidate_boundary_rms_error(&path, &candidate);
            (error <= MAX_TEMPLATE_BOUNDARY_ERROR).then_some((error, segments))
        })
        .min_by(|(left_error, _), (right_error, _)| left_error.total_cmp(right_error))
        .map(|(_, segments)| segments)
}
