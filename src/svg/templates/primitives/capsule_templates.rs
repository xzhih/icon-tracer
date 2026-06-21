use crate::svg::*;

use super::capsule::diagonal_capsule_point;

pub(super) fn low_angle_diagonal_capsule_template_is_preferred(
    axis: (f64, f64),
    radius: f64,
) -> bool {
    let angle = axis.1.abs().atan2(axis.0.abs()).to_degrees();
    (15.0..=20.0).contains(&radius) && (25.0..=40.0).contains(&angle)
}

pub(super) fn low_angle_diagonal_capsule_segments(
    origin: (f64, f64),
    axis: (f64, f64),
    half_length: f64,
    radius: f64,
) -> Vec<SvgPathSegment> {
    let points = [
        [
            (0.877_373_83, 0.982_907_88),
            (0.859_272_895, 0.995_116_537),
            (0.453_627_857, 0.999_223_085),
            (-0.023_976_07, 1.001_664_817),
        ],
        [
            (-0.023_976_07, 1.001_664_817),
            (-0.710_477_067, 1.002_330_744),
            (-0.898_082_528, 0.980_910_1),
            (-0.920_724_037, 0.895_782_464),
        ],
        [
            (-0.920_724_037, 0.895_782_464),
            (-1.043_150_79, 0.463_152_053),
            (-1.011_750_268, -0.897_558_269),
            (-0.877_373_83, -0.982_907_88),
        ],
        [
            (-0.877_373_83, -0.982_907_88),
            (-0.859_272_895, -0.995_116_537),
            (-0.453_627_857, -0.999_223_085),
            (0.023_976_07, -1.001_664_817),
        ],
        [
            (0.023_976_07, -1.001_664_817),
            (0.710_477_067, -1.002_330_744),
            (0.898_082_528, -0.980_910_1),
            (0.920_724_037, -0.895_782_464),
        ],
        [
            (0.920_724_037, -0.895_782_464),
            (1.043_150_79, -0.463_152_053),
            (1.011_750_268, 0.897_558_269),
            (0.877_373_83, 0.982_907_88),
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
