use super::open_ring_paths::*;
use super::*;

pub(crate) fn fit_closed_open_ring_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    const MIN_AXIS: f64 = 48.0;
    const MIN_ASPECT_RATIO: f64 = 0.75;
    const MAX_ASPECT_RATIO: f64 = 1.25;
    const MAX_TEMPLATE_BOUNDARY_ERROR: f64 = 3.0;

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
        open_ring_potrace_segments(FloatBounds {
            min_x: bounds.min_x - 0.1,
            max_x: bounds.max_x + 0.1,
            min_y: bounds.min_y - 0.3,
            max_y: bounds.max_y - 0.5,
        }),
        open_ring_gap_left_potrace_segments(FloatBounds {
            min_x: bounds.min_x - 0.1,
            max_x: bounds.max_x + 0.1,
            min_y: bounds.min_y - 0.3,
            max_y: bounds.max_y - 0.5,
        }),
        open_ring_gap_bottom_potrace_segments(FloatBounds {
            min_x: bounds.min_x + 0.5,
            max_x: bounds.max_x - 0.5,
            min_y: bounds.min_y - 0.3,
            max_y: bounds.max_y + 0.1,
        }),
        open_ring_gap_top_potrace_segments(FloatBounds {
            min_x: bounds.min_x - 0.1,
            max_x: bounds.max_x - 0.5,
            min_y: bounds.min_y - 0.1,
            max_y: bounds.max_y - 0.5,
        }),
    ];

    candidates.into_iter().find(|segments| {
        let candidate = (segments[0].start(), segments.clone());
        pixel_potrace_candidate_boundary_rms_error(&path, &candidate) <= MAX_TEMPLATE_BOUNDARY_ERROR
    })
}
