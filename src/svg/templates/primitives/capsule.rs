use crate::svg::*;

use super::capsule_templates::{
    gentle_angle_diagonal_capsule_segments, gentle_angle_diagonal_capsule_template_is_preferred,
    long_shallow_angle_diagonal_capsule_segments,
    long_shallow_angle_diagonal_capsule_template_is_preferred, low_angle_diagonal_capsule_segments,
    low_angle_diagonal_capsule_template_is_preferred, medium_angle_diagonal_capsule_segments,
    medium_angle_diagonal_capsule_template_is_preferred,
    medium_low_angle_diagonal_capsule_segments,
    medium_low_angle_diagonal_capsule_template_is_preferred,
    shallow_angle_diagonal_capsule_segments, shallow_angle_diagonal_capsule_template_is_preferred,
    thick_low_angle_diagonal_capsule_segments,
    thick_low_angle_diagonal_capsule_template_is_preferred,
};
use super::{normalized_rect_cubic, normalized_rect_line};

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
            choose_capsule_template_by_boundary(
                points,
                small_vertical_capsule_segments(bounds),
                vertical_capsule_segments(bounds, radius),
            )
        })
    } else {
        None
    }
}

pub(crate) fn small_capsule_template_is_preferred(radius: f64) -> bool {
    radius <= 28.0
}

pub(crate) fn choose_capsule_template_by_boundary(
    points: &[(f64, f64)],
    small: Vec<SvgPathSegment>,
    regular: Vec<SvgPathSegment>,
) -> Vec<SvgPathSegment> {
    let small_error = capsule_template_boundary_rms_error(points, &small);
    let regular_error = capsule_template_boundary_rms_error(points, &regular);

    if regular_error < small_error {
        regular
    } else {
        small
    }
}

pub(crate) fn capsule_template_boundary_rms_error(
    points: &[(f64, f64)],
    segments: &[SvgPathSegment],
) -> f64 {
    let Some(first) = segments.first() else {
        return f64::INFINITY;
    };

    let reference = closed_polyline_points(points);
    let candidate = closed_polyline_points(&flattened_potrace_segments(first.start(), segments));
    if reference.len() < 2 || candidate.len() < 2 {
        return f64::INFINITY;
    }

    let reference_to_candidate = mean_squared_distance_to_polyline(&reference, &candidate);
    let candidate_to_reference = mean_squared_distance_to_polyline(&candidate, &reference);
    (reference_to_candidate.max(candidate_to_reference)).sqrt()
}

pub(crate) fn fit_closed_diagonal_capsule_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    let fit = fit_diagonal_capsule(points)?;

    Some(diagonal_capsule_segments(
        fit.origin,
        fit.axis,
        fit.half_length,
        fit.radius,
    ))
}

#[derive(Clone, Copy)]
struct DiagonalCapsuleFit {
    origin: (f64, f64),
    axis: (f64, f64),
    half_length: f64,
    radius: f64,
}

fn fit_diagonal_capsule(points: &[(f64, f64)]) -> Option<DiagonalCapsuleFit> {
    const MIN_RADIUS: f64 = 8.0;
    const MIN_ASPECT_RATIO: f64 = 2.0;
    const AXIS_EPSILON: f64 = 0.06;

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

    Some(DiagonalCapsuleFit {
        origin,
        axis,
        half_length,
        radius,
    })
}

pub(crate) fn diagonal_capsule_allows_compact_replacement(points: &[(f64, f64)]) -> bool {
    const MIN_LOW_ANGLE_RADIUS: f64 = 18.0;
    const MIN_SMALL_RADIUS_ANGLE_DEGREES: f64 = 35.0;

    if diagonal_capsule_prefers_medium_low_template(points) {
        return false;
    }
    if diagonal_capsule_prefers_thick_low_angle_template(points) {
        return false;
    }

    let origin = arc_centroid(points);
    let Some(pca_axis) = principal_axis_for_points(points, origin) else {
        return false;
    };
    let Some(bounds) = local_bounds(points, origin, pca_axis) else {
        return false;
    };

    let radius = (bounds.max_y - bounds.min_y) / 2.0 + 0.125;
    let angle = pca_axis.1.abs().atan2(pca_axis.0.abs()).to_degrees();

    radius >= MIN_LOW_ANGLE_RADIUS || angle >= MIN_SMALL_RADIUS_ANGLE_DEGREES
}

pub(crate) fn diagonal_capsule_prefers_medium_low_template(points: &[(f64, f64)]) -> bool {
    fit_diagonal_capsule(points).is_some_and(|fit| {
        medium_low_angle_diagonal_capsule_template_is_preferred(
            fit.axis,
            fit.half_length,
            fit.radius,
        )
    })
}

pub(crate) fn diagonal_capsule_prefers_thick_low_angle_template(points: &[(f64, f64)]) -> bool {
    fit_diagonal_capsule(points).is_some_and(|fit| {
        thick_low_angle_diagonal_capsule_template_is_preferred(
            fit.axis,
            fit.half_length,
            fit.radius,
        )
    })
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
    if long_shallow_angle_diagonal_capsule_template_is_preferred(axis, half_length, radius) {
        return long_shallow_angle_diagonal_capsule_segments(origin, axis, half_length, radius);
    }

    if shallow_angle_diagonal_capsule_template_is_preferred(axis, radius) {
        return shallow_angle_diagonal_capsule_segments(origin, axis, half_length, radius);
    }

    if gentle_angle_diagonal_capsule_template_is_preferred(axis, radius) {
        return gentle_angle_diagonal_capsule_segments(origin, axis, half_length, radius);
    }

    if medium_low_angle_diagonal_capsule_template_is_preferred(axis, half_length, radius) {
        return medium_low_angle_diagonal_capsule_segments(origin, axis, half_length, radius);
    }

    if thick_low_angle_diagonal_capsule_template_is_preferred(axis, half_length, radius) {
        return thick_low_angle_diagonal_capsule_segments(origin, axis, half_length, radius);
    }

    if low_angle_diagonal_capsule_template_is_preferred(axis, radius) {
        return low_angle_diagonal_capsule_segments(origin, axis, half_length, radius);
    }

    if medium_angle_diagonal_capsule_template_is_preferred(axis, radius) {
        return medium_angle_diagonal_capsule_segments(origin, axis, half_length, radius);
    }

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
