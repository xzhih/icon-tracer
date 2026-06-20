use super::*;

pub(crate) fn fit_closed_smooth_potrace_segments(
    points: &[(f64, f64)],
    allow_ellipse_primitive: bool,
) -> Vec<SvgPathSegment> {
    const SMOOTH_FIT_ERROR: f64 = 1.1;

    if allow_ellipse_primitive {
        if let Some(primitive) = fit_closed_smooth_primitive_segments(points) {
            return primitive;
        }
    }

    let breakpoints = even_fit_breakpoints(points.len());
    let mut segments = Vec::new();

    for index in 0..breakpoints.len() {
        let start = breakpoints[index];
        let end = breakpoints[(index + 1) % breakpoints.len()];
        let arc = closed_arc_points(points, start, end);
        fit_open_cubic_segments(&arc, SMOOTH_FIT_ERROR * SMOOTH_FIT_ERROR, &mut segments);
    }

    segments.into_iter().map(SvgPathSegment::Cubic).collect()
}

pub(crate) fn fit_closed_potrace_primitive_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    fit_closed_capsule_potrace_segments(points)
        .or_else(|| fit_closed_rounded_rect_potrace_segments(points))
        .or_else(|| fit_closed_ellipse_potrace_segments(points))
}

pub(crate) fn fit_closed_smooth_primitive_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    fit_closed_capsule_potrace_segments(points)
        .or_else(|| fit_closed_rounded_rect_potrace_segments(points))
        .or_else(|| fit_closed_smooth_ellipse_segments(points))
}

pub(crate) fn fit_closed_upright_triangle_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    const MIN_AXIS: f64 = 16.0;
    const MIN_ASPECT_RATIO: f64 = 0.75;
    const MAX_ASPECT_RATIO: f64 = 1.25;
    const MAX_BOUNDARY_ERROR: f64 = 0.018;
    const MAX_MEAN_BOUNDARY_ERROR: f64 = 0.006;

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

    let top = ((bounds.min_x + bounds.max_x) / 2.0, bounds.min_y);
    let left = (bounds.min_x, bounds.max_y);
    let right = (bounds.max_x, bounds.max_y);
    let mut max_error = 0.0_f64;
    let mut total_error = 0.0_f64;

    for point in points {
        let distance = distance_squared_to_segment(*point, top, left)
            .0
            .min(distance_squared_to_segment(*point, left, right).0)
            .min(distance_squared_to_segment(*point, right, top).0)
            .sqrt();
        let error = distance / width.max(height);
        max_error = max_error.max(error);
        total_error += error;
    }

    if max_error > MAX_BOUNDARY_ERROR || total_error / points.len() as f64 > MAX_MEAN_BOUNDARY_ERROR
    {
        return None;
    }

    Some(horizontal_upright_triangle_segments(bounds))
}

pub(crate) fn horizontal_upright_triangle_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    let left = bounds.min_x.round();
    let right = bounds.max_x.round();
    let bottom = bounds.max_y.round();
    let width = right - left;
    let top = bottom - width;
    let center_x = (left + right) / 2.0;
    let side_dx = width * 0.251_744_186_046_511_6;
    let mid_y = top + width * 0.502_906_976_744_186;
    let shoulder_dx = width * 0.137_209_302_325_581_4;
    let shoulder_dy = width * 0.273_255_813_953_488_36;
    let top_bias = width * 0.005_813_953_488_372_093;
    let top_handle_dx = width * 0.001_744_186_046_511_628;
    let left_mid = (center_x - side_dx, mid_y);
    let right_mid = (center_x + side_dx, mid_y);
    let top_point = (center_x, top + top_bias);

    vec![
        SvgPathSegment::Line {
            start: left_mid,
            end: (left, bottom),
        },
        SvgPathSegment::Line {
            start: (left, bottom),
            end: (right, bottom),
        },
        SvgPathSegment::Line {
            start: (right, bottom),
            end: right_mid,
        },
        SvgPathSegment::Cubic(CubicSegment {
            start: right_mid,
            control1: (right_mid.0 - shoulder_dx, right_mid.1 - shoulder_dy),
            control2: (center_x + top_handle_dx, top_point.1),
            end: top_point,
        }),
        SvgPathSegment::Cubic(CubicSegment {
            start: top_point,
            control1: (center_x - top_handle_dx, top_point.1),
            control2: (left_mid.0 + shoulder_dx, left_mid.1 - shoulder_dy),
            end: left_mid,
        }),
    ]
}

pub(crate) fn fit_closed_capsule_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    const MIN_RADIUS: f64 = 8.0;
    const MIN_ASPECT_RATIO: f64 = 1.2;

    let bounds = FloatBounds::from_points(points)?;
    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    if width >= height * MIN_ASPECT_RATIO {
        let radius = height / 2.0;
        if radius < MIN_RADIUS {
            return None;
        }

        let center_y = (bounds.min_y + bounds.max_y) / 2.0;
        let start = (bounds.min_x + radius, center_y);
        let end = (bounds.max_x - radius, center_y);
        capsule_boundary_is_close(points, start, end, radius).then(|| {
            if small_capsule_template_is_preferred(radius) {
                small_horizontal_capsule_segments(bounds)
            } else {
                horizontal_capsule_segments(bounds, radius)
            }
        })
    } else if height >= width * MIN_ASPECT_RATIO {
        let radius = width / 2.0;
        if radius < MIN_RADIUS {
            return None;
        }

        let center_x = (bounds.min_x + bounds.max_x) / 2.0;
        let start = (center_x, bounds.min_y + radius);
        let end = (center_x, bounds.max_y - radius);
        capsule_boundary_is_close(points, start, end, radius).then(|| {
            if small_capsule_template_is_preferred(radius) {
                small_vertical_capsule_segments(bounds)
            } else {
                vertical_capsule_segments(bounds, radius)
            }
        })
    } else {
        None
    }
}

pub(crate) fn small_capsule_template_is_preferred(radius: f64) -> bool {
    radius <= 28.0
}

pub(crate) fn fit_closed_diagonal_capsule_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    const MIN_RADIUS: f64 = 8.0;
    const MIN_ASPECT_RATIO: f64 = 2.0;
    const AXIS_EPSILON: f64 = 0.08;

    let origin = arc_centroid(points);
    let pca_axis = principal_axis_for_points(points, origin)?;
    if pca_axis.0.abs() <= AXIS_EPSILON || pca_axis.1.abs() <= AXIS_EPSILON {
        return None;
    }

    let pca_bounds = local_bounds(points, origin, pca_axis)?;
    let half_length = (pca_bounds.max_x - pca_bounds.min_x) / 2.0 + 0.05;
    let radius = (pca_bounds.max_y - pca_bounds.min_y) / 2.0 + 0.125;
    if radius < MIN_RADIUS || half_length < radius * MIN_ASPECT_RATIO {
        return None;
    }

    let axis = refine_diagonal_capsule_axis(points, origin, pca_axis, radius)?;
    if !diagonal_capsule_boundary_is_close(points, origin, axis, half_length, radius) {
        return None;
    }

    Some(diagonal_capsule_segments(origin, axis, half_length, radius))
}

pub(crate) fn principal_axis_for_points(
    points: &[(f64, f64)],
    origin: (f64, f64),
) -> Option<(f64, f64)> {
    let mut xx = 0.0;
    let mut xy = 0.0;
    let mut yy = 0.0;

    for point in points {
        let centered = subtract(*point, origin);
        xx += centered.0 * centered.0;
        xy += centered.0 * centered.1;
        yy += centered.1 * centered.1;
    }

    principal_axis_2x2(xx, xy, yy).map(positive_x_axis)
}

pub(crate) fn refine_diagonal_capsule_axis(
    points: &[(f64, f64)],
    origin: (f64, f64),
    initial_axis: (f64, f64),
    radius: f64,
) -> Option<(f64, f64)> {
    const STEPS: i32 = 240;
    const STEP_RADIANS: f64 = 0.0005;

    let mut best_axis = initial_axis;
    let mut best_score = f64::INFINITY;

    for step in -STEPS..=STEPS {
        let axis = positive_x_axis(rotate_vector(initial_axis, step as f64 * STEP_RADIANS));
        let Some(score) = diagonal_capsule_axis_score(points, origin, axis, radius) else {
            continue;
        };

        if score < best_score {
            best_score = score;
            best_axis = axis;
        }
    }

    best_score.is_finite().then_some(best_axis)
}

pub(crate) fn diagonal_capsule_axis_score(
    points: &[(f64, f64)],
    origin: (f64, f64),
    axis: (f64, f64),
    min_radius: f64,
) -> Option<f64> {
    let bounds = local_bounds(points, origin, axis)?;
    let half_length = (bounds.max_x - bounds.min_x) / 2.0;
    let radius = (bounds.max_y - bounds.min_y) / 2.0;
    if radius < min_radius * 0.8 || half_length <= radius * 2.0 {
        return None;
    }

    let normal = left_normal(axis);
    let center_x = (bounds.min_x + bounds.max_x) / 2.0;
    let center_y = (bounds.min_y + bounds.max_y) / 2.0;
    let rail_limit = half_length - radius * 1.5;
    let mut total = 0.0;
    let mut count = 0usize;

    for point in points {
        let local = point_to_local(*point, origin, axis, normal);
        if (local.0 - center_x).abs() >= rail_limit {
            continue;
        }

        let distance_to_rail = (local.1 - bounds.min_y)
            .abs()
            .min((local.1 - bounds.max_y).abs());
        total += distance_to_rail * distance_to_rail;
        count += 1;
    }

    (count > 0).then_some(total / count as f64 + center_y.abs() * 0.01)
}

pub(crate) fn diagonal_capsule_boundary_is_close(
    points: &[(f64, f64)],
    origin: (f64, f64),
    axis: (f64, f64),
    half_length: f64,
    radius: f64,
) -> bool {
    const MAX_RADIAL_ERROR: f64 = 0.12;
    const MAX_MEAN_RADIAL_ERROR: f64 = 0.065;

    let normal = left_normal(axis);
    let start = (-half_length + radius, 0.0);
    let end = (half_length - radius, 0.0);
    let mut max_error = 0.0_f64;
    let mut total_error = 0.0_f64;

    for point in points {
        let local = point_to_local(*point, origin, axis, normal);
        let distance = distance_squared_to_segment(local, start, end).0.sqrt();
        let error = ((distance - radius) / radius).abs();
        max_error = max_error.max(error);
        total_error += error;
    }

    max_error <= MAX_RADIAL_ERROR && total_error / points.len() as f64 <= MAX_MEAN_RADIAL_ERROR
}

pub(crate) fn capsule_boundary_is_close(
    points: &[(f64, f64)],
    start: (f64, f64),
    end: (f64, f64),
    radius: f64,
) -> bool {
    const MAX_RADIAL_ERROR: f64 = 0.075;
    const MAX_MEAN_RADIAL_ERROR: f64 = 0.03;

    let mut max_error = 0.0_f64;
    let mut total_error = 0.0_f64;

    for point in points {
        let distance = distance_squared_to_segment(*point, start, end).0.sqrt();
        let error = ((distance - radius) / radius).abs();
        max_error = max_error.max(error);
        total_error += error;
    }

    max_error <= MAX_RADIAL_ERROR && total_error / points.len() as f64 <= MAX_MEAN_RADIAL_ERROR
}

pub(crate) fn fit_closed_rounded_rect_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    const MIN_RADIUS: f64 = 6.0;
    const MAX_RADIUS_RATIO: f64 = 0.45;

    let bounds = FloatBounds::from_points(points)?;
    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let radius = estimate_rounded_rect_radius(points, bounds)?;
    if radius < MIN_RADIUS || radius >= width.min(height) * MAX_RADIUS_RATIO {
        return None;
    }

    rounded_rect_boundary_is_close(points, bounds, radius)
        .then(|| rounded_rect_potrace_segments(bounds, radius))
}

pub(crate) fn estimate_rounded_rect_radius(
    points: &[(f64, f64)],
    bounds: FloatBounds,
) -> Option<f64> {
    const EDGE_EPSILON: f64 = 0.75;
    const MIN_STRAIGHT_EDGE: f64 = 8.0;

    let mut candidates = Vec::new();
    collect_horizontal_rounded_rect_radii(
        points,
        bounds.min_y,
        bounds.min_x,
        bounds.max_x,
        EDGE_EPSILON,
        MIN_STRAIGHT_EDGE,
        &mut candidates,
    );
    collect_horizontal_rounded_rect_radii(
        points,
        bounds.max_y,
        bounds.min_x,
        bounds.max_x,
        EDGE_EPSILON,
        MIN_STRAIGHT_EDGE,
        &mut candidates,
    );
    collect_vertical_rounded_rect_radii(
        points,
        bounds.min_x,
        bounds.min_y,
        bounds.max_y,
        EDGE_EPSILON,
        MIN_STRAIGHT_EDGE,
        &mut candidates,
    );
    collect_vertical_rounded_rect_radii(
        points,
        bounds.max_x,
        bounds.min_y,
        bounds.max_y,
        EDGE_EPSILON,
        MIN_STRAIGHT_EDGE,
        &mut candidates,
    );

    candidates.retain(|value| value.is_finite() && *value > 0.0);
    if candidates.len() < 4 {
        return None;
    }

    candidates.sort_by(f64::total_cmp);
    Some(candidates[candidates.len() / 2])
}

pub(crate) fn collect_horizontal_rounded_rect_radii(
    points: &[(f64, f64)],
    y: f64,
    left: f64,
    right: f64,
    epsilon: f64,
    min_span: f64,
    radii: &mut Vec<f64>,
) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;

    for point in points {
        if (point.1 - y).abs() <= epsilon {
            min_x = min_x.min(point.0);
            max_x = max_x.max(point.0);
        }
    }

    if max_x - min_x >= min_span {
        radii.push(min_x - left);
        radii.push(right - max_x);
    }
}

pub(crate) fn collect_vertical_rounded_rect_radii(
    points: &[(f64, f64)],
    x: f64,
    top: f64,
    bottom: f64,
    epsilon: f64,
    min_span: f64,
    radii: &mut Vec<f64>,
) {
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for point in points {
        if (point.0 - x).abs() <= epsilon {
            min_y = min_y.min(point.1);
            max_y = max_y.max(point.1);
        }
    }

    if max_y - min_y >= min_span {
        radii.push(min_y - top);
        radii.push(bottom - max_y);
    }
}

pub(crate) fn rounded_rect_boundary_is_close(
    points: &[(f64, f64)],
    bounds: FloatBounds,
    radius: f64,
) -> bool {
    const MAX_RADIAL_ERROR: f64 = 0.4;
    const MAX_MEAN_RADIAL_ERROR: f64 = 0.12;

    let inner = FloatBounds {
        min_x: bounds.min_x + radius,
        max_x: bounds.max_x - radius,
        min_y: bounds.min_y + radius,
        max_y: bounds.max_y - radius,
    };
    if inner.min_x > inner.max_x || inner.min_y > inner.max_y {
        return false;
    }

    let mut max_error = 0.0_f64;
    let mut total_error = 0.0_f64;
    for point in points {
        let nearest = inner.clamp(*point);
        let distance = (point.0 - nearest.0).hypot(point.1 - nearest.1);
        let error = ((distance - radius) / radius).abs();
        max_error = max_error.max(error);
        total_error += error;
    }

    max_error <= MAX_RADIAL_ERROR && total_error / points.len() as f64 <= MAX_MEAN_RADIAL_ERROR
}

pub(crate) fn rounded_rect_potrace_segments(
    bounds: FloatBounds,
    radius: f64,
) -> Vec<SvgPathSegment> {
    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;
    let radius_ratio = radius / width.min(height);

    if radius_ratio >= 0.16 {
        large_rounded_rect_potrace_segments(bounds)
    } else {
        small_rounded_rect_potrace_segments(bounds)
    }
}

pub(crate) fn small_rounded_rect_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    let points = [
        [
            (0.070_945_945_946, 0.014_393_939_394),
            (0.038_513_513_514, 0.031_060_606_061),
            (0.024_324_324_324, 0.047_727_272_727),
            (0.010_135_135_135, 0.085_606_060_606),
        ],
        [
            (0.010_135_135_135, 0.085_606_060_606),
            (0.001_351_351_351, 0.109_090_909_091),
            (0.0, 0.168_939_393_939),
            (0.0, 0.501_515_151_515),
        ],
        [
            (0.0, 0.501_515_151_515),
            (0.0, 0.862_121_212_121),
            (0.001_351_351_351, 0.892_424_242_424),
            (0.012_837_837_838, 0.920_454_545_455),
        ],
        [
            (0.012_837_837_838, 0.920_454_545_455),
            (0.027_702_702_703, 0.956_818_181_818),
            (0.042_567_567_568, 0.972_727_272_727),
            (0.076_351_351_351, 0.988_636_363_636),
        ],
        [
            (0.076_351_351_351, 0.988_636_363_636),
            (0.097_297_297_297, 0.998_484_848_485),
            (0.156_081_081_081, 1.0),
            (0.501_351_351_351, 1.0),
        ],
        [
            (0.501_351_351_351, 1.0),
            (0.875_675_675_676, 1.0),
            (0.904_054_054_054, 0.998_484_848_485),
            (0.929_054_054_054, 0.985_606_060_606),
        ],
        [
            (0.929_054_054_054, 0.985_606_060_606),
            (0.961_486_486_486, 0.968_939_393_939),
            (0.975_675_675_676, 0.952_272_727_273),
            (0.989_864_864_865, 0.914_393_939_394),
        ],
        [
            (0.989_864_864_865, 0.914_393_939_394),
            (1.005_405_405_405, 0.872_727_272_727),
            (1.005_405_405_405, 0.127_272_727_273),
            (0.989_864_864_865, 0.085_606_060_606),
        ],
        [
            (0.989_864_864_865, 0.085_606_060_606),
            (0.975_675_675_676, 0.047_727_272_727),
            (0.961_486_486_486, 0.031_060_606_061),
            (0.929_054_054_054, 0.014_393_939_394),
        ],
        [
            (0.929_054_054_054, 0.014_393_939_394),
            (0.904_054_054_054, 0.001_515_151_515),
            (0.875_675_675_676, 0.0),
            (0.5, 0.0),
        ],
        [
            (0.5, 0.0),
            (0.124_324_324_324, 0.0),
            (0.095_945_945_946, 0.001_515_151_515),
            (0.070_945_945_946, 0.014_393_939_394),
        ],
    ];

    normalized_rect_cubic_segments(bounds, &points)
}

pub(crate) fn normalized_rect_cubic_segments(
    bounds: FloatBounds,
    points: &[[(f64, f64); 4]],
) -> Vec<SvgPathSegment> {
    points
        .iter()
        .map(|[start, control1, control2, end]| {
            SvgPathSegment::Cubic(CubicSegment {
                start: normalized_rect_point(bounds, *start),
                control1: normalized_rect_point(bounds, *control1),
                control2: normalized_rect_point(bounds, *control2),
                end: normalized_rect_point(bounds, *end),
            })
        })
        .collect()
}

pub(crate) fn large_rounded_rect_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    vec![
        normalized_rect_cubic(
            bounds,
            (0.148_648_648_649, 0.012_121_212_121),
            (0.081_081_081_081, 0.039_393_939_394),
            (0.032_432_432_432, 0.095_454_545_455),
            (0.010_135_135_135, 0.171_212_121_212),
        ),
        normalized_rect_cubic(
            bounds,
            (0.010_135_135_135, 0.171_212_121_212),
            (0.000_675_675_676, 0.203_787_878_788),
            (0.0, 0.248_484_848_485),
            (0.001_351_351_351, 0.515_151_515_152),
        ),
        normalized_rect_line(
            bounds,
            (0.001_351_351_351, 0.515_151_515_152),
            (0.003_378_378_378, 0.821_969_696_97),
        ),
        normalized_rect_line(
            bounds,
            (0.003_378_378_378, 0.821_969_696_97),
            (0.020_270_270_270, 0.859_848_484_848),
        ),
        normalized_rect_cubic(
            bounds,
            (0.020_270_270_270, 0.859_848_484_848),
            (0.041_216_216_216, 0.908_333_333_333),
            (0.081_756_756_757, 0.953_787_878_788),
            (0.125, 0.977_272_727_273),
        ),
        normalized_rect_line(
            bounds,
            (0.125, 0.977_272_727_273),
            (0.158_783_783_784, 0.996_212_121_212),
        ),
        normalized_rect_line(
            bounds,
            (0.158_783_783_784, 0.996_212_121_212),
            (0.5, 0.996_212_121_212),
        ),
        normalized_rect_line(
            bounds,
            (0.5, 0.996_212_121_212),
            (0.841_216_216_216, 0.996_212_121_212),
        ),
        normalized_rect_line(
            bounds,
            (0.841_216_216_216, 0.996_212_121_212),
            (0.875, 0.977_272_727_273),
        ),
        normalized_rect_cubic(
            bounds,
            (0.875, 0.977_272_727_273),
            (0.918_243_243_243, 0.953_787_878_788),
            (0.958_783_783_784, 0.908_333_333_333),
            (0.979_729_729_73, 0.859_848_484_848),
        ),
        normalized_rect_line(
            bounds,
            (0.979_729_729_73, 0.859_848_484_848),
            (0.996_621_621_622, 0.821_969_696_97),
        ),
        normalized_rect_line(
            bounds,
            (0.996_621_621_622, 0.821_969_696_97),
            (0.996_621_621_622, 0.5),
        ),
        normalized_rect_line(
            bounds,
            (0.996_621_621_622, 0.5),
            (0.996_621_621_622, 0.178_030_303_03),
        ),
        normalized_rect_line(
            bounds,
            (0.996_621_621_622, 0.178_030_303_03),
            (0.979_729_729_73, 0.140_151_515_152),
        ),
        normalized_rect_cubic(
            bounds,
            (0.979_729_729_73, 0.140_151_515_152),
            (0.958_783_783_784, 0.091_666_666_667),
            (0.918_243_243_243, 0.046_212_121_212),
            (0.875, 0.022_727_272_727),
        ),
        normalized_rect_line(
            bounds,
            (0.875, 0.022_727_272_727),
            (0.841_216_216_216, 0.003_787_878_788),
        ),
        normalized_rect_line(
            bounds,
            (0.841_216_216_216, 0.003_787_878_788),
            (0.510_135_135_135, 0.002_272_727_273),
        ),
        normalized_rect_cubic(
            bounds,
            (0.510_135_135_135, 0.002_272_727_273),
            (0.222_297_297_297, 0.000_757_575_758),
            (0.175, 0.002_272_727_273),
            (0.148_648_648_649, 0.012_121_212_121),
        ),
    ]
}

pub(crate) fn normalized_rect_cubic(
    bounds: FloatBounds,
    start: (f64, f64),
    control1: (f64, f64),
    control2: (f64, f64),
    end: (f64, f64),
) -> SvgPathSegment {
    SvgPathSegment::Cubic(CubicSegment {
        start: normalized_rect_point(bounds, start),
        control1: normalized_rect_point(bounds, control1),
        control2: normalized_rect_point(bounds, control2),
        end: normalized_rect_point(bounds, end),
    })
}

pub(crate) fn normalized_rect_line(
    bounds: FloatBounds,
    start: (f64, f64),
    end: (f64, f64),
) -> SvgPathSegment {
    SvgPathSegment::Line {
        start: normalized_rect_point(bounds, start),
        end: normalized_rect_point(bounds, end),
    }
}

pub(crate) fn normalized_rect_point(bounds: FloatBounds, point: (f64, f64)) -> (f64, f64) {
    (
        bounds.min_x + (bounds.max_x - bounds.min_x) * point.0,
        bounds.min_y + (bounds.max_y - bounds.min_y) * point.1,
    )
}

pub(crate) fn horizontal_capsule_segments(bounds: FloatBounds, radius: f64) -> Vec<SvgPathSegment> {
    let left = bounds.min_x;
    let right = bounds.max_x;
    let top = bounds.min_y;
    let bottom = bounds.max_y;
    // Potrace fits pixel capsules with six cubics that are slightly squarer
    // than ideal circular arcs; these offsets scale that bias by radius.
    let p0 = (
        left + radius * 0.752_083_333_333_333_3,
        top + radius * 0.033_333_333_333_333_33,
    );
    let p1 = (
        left + radius * 0.031_25,
        top + radius * 1.239_583_333_333_333_3,
    );
    let p2 = (
        left + radius * 0.764_583_333_333_333_3,
        bottom - radius * 0.031_25,
    );
    let p3 = (
        right - radius * 0.760_416_666_666_666_6,
        bottom - radius * 0.031_25,
    );
    let p4 = (
        right - radius * 0.031_25,
        top + radius * 0.764_583_333_333_333_3,
    );
    let p5 = (
        right - radius * 0.760_416_666_666_666_6,
        top + radius * 0.031_25,
    );

    vec![
        SvgPathSegment::Cubic(CubicSegment {
            start: p0,
            control1: (left + radius * 0.225, top + radius * 0.175),
            control2: (
                left - radius * 0.102_083_333_333_333_33,
                top + radius * 0.722_916_666_666_666_7,
            ),
            end: p1,
        }),
        SvgPathSegment::Cubic(CubicSegment {
            start: p1,
            control1: (
                left + radius * 0.125,
                bottom - radius * 0.402_083_333_333_333_3,
            ),
            control2: (
                left + radius * 0.404_166_666_666_666_7,
                bottom - radius * 0.125,
            ),
            end: p2,
        }),
        SvgPathSegment::Cubic(CubicSegment {
            start: p2,
            control1: (
                left + radius * 0.941_666_666_666_666_7,
                bottom + radius * 0.014_583_333_333_333_334,
            ),
            control2: (
                right - radius * 0.939_583_333_333_333_3,
                bottom + radius * 0.014_583_333_333_333_334,
            ),
            end: p3,
        }),
        SvgPathSegment::Cubic(CubicSegment {
            start: p3,
            control1: (
                right - radius * 0.227_083_333_333_333_33,
                bottom - radius * 0.170_833_333_333_333_34,
            ),
            control2: (
                right + radius * 0.104_166_666_666_666_67,
                top + radius * 1.283_333_333_333_333_4,
            ),
            end: p4,
        }),
        SvgPathSegment::Cubic(CubicSegment {
            start: p4,
            control1: (
                right - radius * 0.125,
                top + radius * 0.404_166_666_666_666_7,
            ),
            control2: (
                right - radius * 0.402_083_333_333_333_3,
                top + radius * 0.125,
            ),
            end: p5,
        }),
        SvgPathSegment::Cubic(CubicSegment {
            start: p5,
            control1: (
                right - radius * 0.935_416_666_666_666_7,
                top - radius * 0.014_583_333_333_333_334,
            ),
            control2: (
                left + radius * 0.922_916_666_666_666_7,
                top - radius * 0.012_5,
            ),
            end: p0,
        }),
    ]
}

pub(crate) fn small_horizontal_capsule_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    vec![
        normalized_rect_cubic(
            bounds,
            (0.078_571_428_571, 0.027_5),
            (0.050_595_238_095, 0.062_5),
            (0.013_690_476_190, 0.227_5),
            (0.005_357_142_857, 0.355),
        ),
        normalized_rect_cubic(
            bounds,
            (0.005_357_142_857, 0.355),
            (-0.008_333_333_333, 0.577_5),
            (0.012_5, 0.81),
            (0.057_738_095_238, 0.927_5),
        ),
        normalized_rect_line(
            bounds,
            (0.057_738_095_238, 0.927_5),
            (0.080_357_142_857, 0.987_5),
        ),
        normalized_rect_line(
            bounds,
            (0.080_357_142_857, 0.987_5),
            (0.500_595_238_095, 0.987_5),
        ),
        normalized_rect_line(
            bounds,
            (0.500_595_238_095, 0.987_5),
            (0.920_238_095_238, 0.987_5),
        ),
        normalized_rect_line(
            bounds,
            (0.920_238_095_238, 0.987_5),
            (0.944_642_857_143, 0.92),
        ),
        normalized_rect_cubic(
            bounds,
            (0.944_642_857_143, 0.92),
            (0.981_547_619_048, 0.817_5),
            (0.997_023_809_524, 0.695),
            (0.997_023_809_524, 0.5),
        ),
        normalized_rect_cubic(
            bounds,
            (0.997_023_809_524, 0.5),
            (0.997_023_809_524, 0.305),
            (0.981_547_619_048, 0.182_5),
            (0.944_642_857_143, 0.08),
        ),
        normalized_rect_line(
            bounds,
            (0.944_642_857_143, 0.08),
            (0.920_238_095_238, 0.012_5),
        ),
        normalized_rect_line(
            bounds,
            (0.920_238_095_238, 0.012_5),
            (0.509_523_809_524, 0.007_5),
        ),
        normalized_rect_cubic(
            bounds,
            (0.509_523_809_524, 0.007_5),
            (0.260_119_047_619, 0.005),
            (0.090_476_190_476, 0.012_5),
            (0.078_571_428_571, 0.027_5),
        ),
    ]
}

pub(crate) fn diagonal_capsule_segments(
    origin: (f64, f64),
    axis: (f64, f64),
    half_length: f64,
    radius: f64,
) -> Vec<SvgPathSegment> {
    if small_diagonal_capsule_template_is_preferred(radius) {
        return small_diagonal_capsule_segments(origin, axis, half_length, radius);
    }

    let points = [
        [
            (0.875_504_324, -0.897_971_837),
            (0.855_800_28, -0.932_155_7),
            (0.455_887_28, -0.959_455_11),
            (-0.011_574_26, -0.954_623_18),
        ],
        [
            (-0.011_574_26, -0.954_623_18),
            (-0.912_022_08, -0.957_522_21),
            (-0.902_032_99, -0.957_401_4),
            (-0.954_777_34, -0.618_579_0),
        ],
        [
            (-0.954_777_34, -0.618_579_0),
            (-0.978_351_99, -0.467_104_67),
            (-0.998_738_72, -0.094_943_02),
            (-0.995_272_87, 0.091_801_58),
        ],
        [
            (-0.995_272_87, 0.091_801_58),
            (-0.987_013_56, 0.464_810_22),
            (-0.937_270_09, 0.796_384_88),
            (-0.875_504_324, 0.897_971_837),
        ],
        [
            (-0.875_504_324, 0.897_971_837),
            (-0.855_800_28, 0.932_155_7),
            (-0.455_887_28, 0.959_455_11),
            (0.011_574_26, 0.954_623_18),
        ],
        [
            (0.011_574_26, 0.954_623_18),
            (0.912_022_08, 0.957_522_21),
            (0.902_032_99, 0.957_401_4),
            (0.954_777_34, 0.618_579_0),
        ],
        [
            (0.954_777_34, 0.618_579_0),
            (0.978_351_99, 0.467_104_67),
            (0.998_738_72, 0.094_943_02),
            (0.995_272_87, -0.091_801_58),
        ],
        [
            (0.995_272_87, -0.091_801_58),
            (0.987_013_56, -0.464_810_22),
            (0.937_270_09, -0.796_384_88),
            (0.875_504_324, -0.897_971_837),
        ],
    ];

    points
        .into_iter()
        .map(|[start, control1, control2, end]| {
            SvgPathSegment::Cubic(CubicSegment {
                start: diagonal_capsule_point(origin, axis, half_length, radius, start),
                control1: diagonal_capsule_point(origin, axis, half_length, radius, control1),
                control2: diagonal_capsule_point(origin, axis, half_length, radius, control2),
                end: diagonal_capsule_point(origin, axis, half_length, radius, end),
            })
        })
        .collect()
}

pub(crate) fn small_diagonal_capsule_template_is_preferred(radius: f64) -> bool {
    radius <= 15.0
}

pub(crate) fn small_diagonal_capsule_segments(
    origin: (f64, f64),
    axis: (f64, f64),
    half_length: f64,
    radius: f64,
) -> Vec<SvgPathSegment> {
    let points = [
        [
            (0.918_015_968_631, -0.872_404_774_696),
            (0.896_213_666_281, -0.937_477_201_239),
            (-0.864_960_766_722, -0.967_648_999_336),
            (-0.905_282_309_668, -0.897_505_781_69),
        ],
        [
            (-0.905_282_309_668, -0.897_505_781_69),
            (-0.917_540_384_326, -0.882_140_333_785),
            (-0.938_458_388_411, -0.788_718_588_947),
            (-0.951_865_018_552, -0.707_512_367_76),
        ],
        [
            (-0.951_865_018_552, -0.707_512_367_76),
            (-1.018_481_433_255, -0.265_603_037_611),
            (-1.001_677_859_06, 0.588_068_194_594),
            (-0.920_634_277_964, 0.861_126_574_592),
        ],
        [
            (-0.920_634_277_964, 0.861_126_574_592),
            (-0.897_559_411_922, 0.936_924_051_062),
            (0.859_504_602_213, 0.975_608_298_377),
            (0.905_282_309_668, 0.897_505_781_69),
        ],
        [
            (0.905_282_309_668, 0.897_505_781_69),
            (0.963_318_091_782, 0.804_037_817_097),
            (1.005_879_804_543, 0.234_918_361_556),
            (0.990_968_835_088, -0.225_198_704_68),
        ],
        [
            (0.990_968_835_088, -0.225_198_704_68),
            (0.981_396_136_821, -0.489_283_924_793),
            (0.949_728_408_413, -0.777_753_972_452),
            (0.918_015_968_631, -0.872_404_774_696),
        ],
    ];

    points
        .into_iter()
        .map(|[start, control1, control2, end]| {
            SvgPathSegment::Cubic(CubicSegment {
                start: diagonal_capsule_point(origin, axis, half_length, radius, start),
                control1: diagonal_capsule_point(origin, axis, half_length, radius, control1),
                control2: diagonal_capsule_point(origin, axis, half_length, radius, control2),
                end: diagonal_capsule_point(origin, axis, half_length, radius, end),
            })
        })
        .collect()
}

pub(crate) fn diagonal_capsule_point(
    origin: (f64, f64),
    axis: (f64, f64),
    half_length: f64,
    radius: f64,
    point: (f64, f64),
) -> (f64, f64) {
    let normal = left_normal(axis);
    add(
        origin,
        add(
            scale(axis, point.0 * half_length),
            scale(normal, point.1 * radius),
        ),
    )
}

pub(crate) fn small_vertical_capsule_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    let transposed = FloatBounds {
        min_x: bounds.min_y,
        max_x: bounds.max_y,
        min_y: bounds.min_x,
        max_y: bounds.max_x,
    };

    small_horizontal_capsule_segments(transposed)
        .into_iter()
        .map(transpose_svg_path_segment)
        .collect()
}

pub(crate) fn vertical_capsule_segments(bounds: FloatBounds, radius: f64) -> Vec<SvgPathSegment> {
    let transposed = FloatBounds {
        min_x: bounds.min_y,
        max_x: bounds.max_y,
        min_y: bounds.min_x,
        max_y: bounds.max_x,
    };

    horizontal_capsule_segments(transposed, radius)
        .into_iter()
        .map(transpose_svg_path_segment)
        .collect()
}

pub(crate) fn transpose_svg_path_segment(segment: SvgPathSegment) -> SvgPathSegment {
    match segment {
        SvgPathSegment::Line { start, end } => SvgPathSegment::Line {
            start: transpose_point(start),
            end: transpose_point(end),
        },
        SvgPathSegment::Cubic(cubic) => SvgPathSegment::Cubic(CubicSegment {
            start: transpose_point(cubic.start),
            control1: transpose_point(cubic.control1),
            control2: transpose_point(cubic.control2),
            end: transpose_point(cubic.end),
        }),
    }
}

pub(crate) fn transpose_point(point: (f64, f64)) -> (f64, f64) {
    (point.1, point.0)
}

pub(crate) fn fit_closed_ring_ellipse_potrace_segments(
    points: &[(f64, f64)],
    is_hole: bool,
) -> Option<Vec<SvgPathSegment>> {
    let (center, rx, ry) = closed_ellipse_fit(points)?;
    let center = (
        snap_near_integer_ellipse_value(center.0),
        snap_near_integer_ellipse_value(center.1),
    );
    let rx = snap_near_integer_ellipse_value(rx).max(1.0);
    let ry = snap_near_integer_ellipse_value(ry).max(1.0);

    Some(if is_hole {
        potrace_ring_inner_ellipse_segments(center, rx, ry)
    } else {
        potrace_ring_outer_ellipse_segments(center, rx, ry)
    })
}

pub(crate) fn fit_closed_small_ellipse_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    let (center, rx, ry) = closed_ellipse_fit(points)?;
    let center = (
        snap_near_integer_ellipse_value(center.0),
        snap_near_integer_ellipse_value(center.1),
    );
    let rx = snap_near_integer_ellipse_value(rx).max(1.0);
    let ry = snap_near_integer_ellipse_value(ry).max(1.0);

    Some(potrace_small_ellipse_segments(center, rx, ry))
}

pub(crate) fn potrace_small_ellipse_segments(
    center: (f64, f64),
    rx: f64,
    ry: f64,
) -> Vec<SvgPathSegment> {
    let points = [
        [
            (-0.261_904_761_905, -0.961_904_761_905),
            (-0.995_238_095_238, -0.766_666_666_667),
            (-1.242_857_142_857, 0.166_666_666_667),
            (-0.704_761_904_762, 0.704_761_904_762),
        ],
        [
            (-0.704_761_904_762, 0.704_761_904_762),
            (-0.130_952_380_952, 1.276_190_476_19),
            (0.845_238_095_238, 0.959_523_809_524),
            (0.983_333_333_333, 0.157_142_857_143),
        ],
        [
            (0.983_333_333_333, 0.157_142_857_143),
            (1.104_761_904_762, -0.547_619_047_619),
            (0.435_714_285_714, -1.147_619_047_619),
            (-0.261_904_761_905, -0.961_904_761_905),
        ],
    ];

    normalized_ellipse_cubic_segments(center, rx, ry, &points)
}

pub(crate) fn potrace_ring_outer_ellipse_segments(
    center: (f64, f64),
    rx: f64,
    ry: f64,
) -> Vec<SvgPathSegment> {
    // Normalized from Potrace 1.16's ring fixture. Potrace chooses a different
    // opticurve split for ellipse boundaries when a hole is present than it
    // does for a standalone circle.
    let points = [
        [
            (-0.192_307_692_308, -0.980_769_230_769),
            (-0.574_358_974_359, -0.9),
            (-0.869_230_769_231, -0.619_230_769_231),
            (-0.970_512_820_513, -0.243_589_743_59),
        ],
        [
            (-0.970_512_820_513, -0.243_589_743_59),
            (-1.001_282_051_282, -0.129_487_179_487),
            (-1.001_282_051_282, 0.129_487_179_487),
            (-0.970_512_820_513, 0.243_589_743_59),
        ],
        [
            (-0.970_512_820_513, 0.243_589_743_59),
            (-0.893_589_743_59, 0.529_487_179_487),
            (-0.702_564_102_564, 0.762_820_512_821),
            (-0.442_307_692_308, 0.892_307_692_308),
        ],
        [
            (-0.442_307_692_308, 0.892_307_692_308),
            (-0.276_923_076_923, 0.973_076_923_077),
            (-0.191_025_641_026, 0.993_589_743_59),
            (0.0, 0.993_589_743_59),
        ],
        [
            (0.0, 0.993_589_743_59),
            (0.191_025_641_026, 0.993_589_743_59),
            (0.276_923_076_923, 0.973_076_923_077),
            (0.442_307_692_308, 0.892_307_692_308),
        ],
        [
            (0.442_307_692_308, 0.892_307_692_308),
            (0.702_564_102_564, 0.762_820_512_821),
            (0.893_589_743_59, 0.529_487_179_487),
            (0.970_512_820_513, 0.243_589_743_59),
        ],
        [
            (0.970_512_820_513, 0.243_589_743_59),
            (1.001_282_051_282, 0.129_487_179_487),
            (1.001_282_051_282, -0.128_205_128_205),
            (0.970_512_820_513, -0.243_589_743_59),
        ],
        [
            (0.970_512_820_513, -0.243_589_743_59),
            (0.938_461_538_462, -0.364_102_564_103),
            (0.846_153_846_154, -0.544_871_794_872),
            (0.765_384_615_385, -0.642_307_692_308),
        ],
        [
            (0.765_384_615_385, -0.642_307_692_308),
            (0.643_589_743_59, -0.788_461_538_462),
            (0.439_743_589_744, -0.917_948_717_949),
            (0.25, -0.969_230_769_231),
        ],
        [
            (0.25, -0.969_230_769_231),
            (0.143_589_743_59, -0.997_435_897_436),
            (-0.088_461_538_462, -1.003_846_153_846),
            (-0.192_307_692_308, -0.980_769_230_769),
        ],
    ];

    normalized_ellipse_cubic_segments(center, rx, ry, &points)
}

pub(crate) fn potrace_ring_inner_ellipse_segments(
    center: (f64, f64),
    rx: f64,
    ry: f64,
) -> Vec<SvgPathSegment> {
    let points = [
        [
            (0.25, -0.964_285_714_286),
            (0.992_857_142_857, -0.771_428_571_429),
            (1.245_238_095_238, 0.161_904_761_905),
            (0.704_761_904_762, 0.704_761_904_762),
        ],
        [
            (0.704_761_904_762, 0.704_761_904_762),
            (0.130_952_380_952, 1.276_190_476_19),
            (-0.845_238_095_238, 0.959_523_809_524),
            (-0.983_333_333_333, 0.157_142_857_143),
        ],
        [
            (-0.983_333_333_333, 0.157_142_857_143),
            (-1.102_380_952_381, -0.545_238_095_238),
            (-0.445_238_095_238, -1.142_857_142_857),
            (0.25, -0.964_285_714_286),
        ],
    ];

    normalized_ellipse_cubic_segments(center, rx, ry, &points)
}

pub(crate) fn normalized_ellipse_cubic_segments(
    center: (f64, f64),
    rx: f64,
    ry: f64,
    points: &[[(f64, f64); 4]],
) -> Vec<SvgPathSegment> {
    points
        .iter()
        .map(|[start, control1, control2, end]| {
            SvgPathSegment::Cubic(CubicSegment {
                start: ellipse_normalized_point(center, rx, ry, *start),
                control1: ellipse_normalized_point(center, rx, ry, *control1),
                control2: ellipse_normalized_point(center, rx, ry, *control2),
                end: ellipse_normalized_point(center, rx, ry, *end),
            })
        })
        .collect()
}

pub(crate) fn snap_near_integer_ellipse_value(value: f64) -> f64 {
    const MAX_SNAP_DISTANCE: f64 = 0.25;

    let nearest = value.round();
    if (value - nearest).abs() <= MAX_SNAP_DISTANCE {
        nearest
    } else {
        value
    }
}
