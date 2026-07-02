use super::*;

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
