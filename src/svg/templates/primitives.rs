use super::*;

mod capsule;
mod capsule_templates;
mod ellipse;
mod rounded_rect_templates;

pub(crate) use capsule::*;
pub(crate) use ellipse::*;

use rounded_rect_templates::{
    vertical_rounded_rect_potrace_segments, vertical_rounded_rect_template_is_preferred,
};

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

pub(crate) fn fit_closed_rounded_rect_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    fit_closed_rounded_rect_potrace_segments_with(points, |_, _, _| true)
}

pub(crate) fn fit_closed_vertical_rounded_rect_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    fit_closed_rounded_rect_potrace_segments_with(points, |width, height, radius_ratio| {
        vertical_rounded_rect_template_is_preferred(width, height, radius_ratio)
    })
}

fn fit_closed_rounded_rect_potrace_segments_with(
    points: &[(f64, f64)],
    accepts_template: impl FnOnce(f64, f64, f64) -> bool,
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

    let radius_ratio = radius / width.min(height);
    if !accepts_template(width, height, radius_ratio) {
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

    if vertical_rounded_rect_template_is_preferred(width, height, radius_ratio) {
        return vertical_rounded_rect_potrace_segments(bounds);
    }

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
