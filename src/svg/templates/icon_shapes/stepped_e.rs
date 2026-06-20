use super::stepped_e_paths::*;
use super::*;

#[cfg(test)]
pub(crate) fn fit_closed_stepped_e_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    closed_stepped_e_potrace_candidates(points)?
        .into_iter()
        .filter_map(|segments| {
            let path = TracePath {
                points: points.to_vec(),
                is_hole: false,
            };
            let candidate = (segments[0].start(), segments.clone());
            let error = pixel_potrace_candidate_boundary_rms_error(&path, &candidate);
            (error <= MAX_TEMPLATE_BOUNDARY_ERROR).then_some((error, segments))
        })
        .min_by(|(left_error, _), (right_error, _)| left_error.total_cmp(right_error))
        .map(|(_, segments)| segments)
}

pub(crate) fn closed_stepped_e_potrace_candidates(
    points: &[(f64, f64)],
) -> Option<Vec<Vec<SvgPathSegment>>> {
    const MIN_AXIS: f64 = 48.0;
    const MIN_ASPECT_RATIO: f64 = 0.75;
    const MAX_ASPECT_RATIO: f64 = 1.25;

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

    Some(vec![
        stepped_e_potrace_segments(bounds),
        stepped_e_left_potrace_segments(bounds),
        stepped_e_down_potrace_segments(bounds),
        stepped_e_up_potrace_segments(bounds),
    ])
}

#[cfg(test)]
const MAX_TEMPLATE_BOUNDARY_ERROR: f64 = 3.0;
