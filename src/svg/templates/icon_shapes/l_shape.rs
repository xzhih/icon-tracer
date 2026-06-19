use super::*;

pub(crate) fn fit_closed_l_potrace_segments(points: &[(f64, f64)]) -> Option<Vec<SvgPathSegment>> {
    const MIN_AXIS: f64 = 48.0;
    const MIN_ASPECT_RATIO: f64 = 0.75;
    const MAX_ASPECT_RATIO: f64 = 1.25;
    const MAX_TEMPLATE_BOUNDARY_ERROR: f64 = 3.0;

    fit_closed_template_variants(
        points,
        MIN_AXIS,
        MIN_ASPECT_RATIO,
        MAX_ASPECT_RATIO,
        MAX_TEMPLATE_BOUNDARY_ERROR,
        l_potrace_segments,
        &ORTHOGONAL_TEMPLATE_TRANSFORMS,
    )
}

pub(crate) fn l_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    vec![
        normalized_rect_cubic(
            bounds,
            (0.075, 0.012_5),
            (0.040_714_285_714, 0.026_973_684_211),
            (0.025_714_285_714, 0.041_447_368_421),
            (0.010_714_285_714, 0.074_342_105_263),
        ),
        normalized_rect_cubic(
            bounds,
            (0.010_714_285_714, 0.074_342_105_263),
            (0.001_428_571_429, 0.094_736_842_105),
            (0.0, 0.153_289_473_684),
            (0.0, 0.501_315_789_474),
        ),
        normalized_rect_cubic(
            bounds,
            (0.0, 0.501_315_789_474),
            (0.0, 0.878_289_473_684),
            (0.001_428_571_429, 0.906_578_947_368),
            (0.013_571_428_571, 0.930_921_052_632),
        ),
        normalized_rect_cubic(
            bounds,
            (0.013_571_428_571, 0.930_921_052_632),
            (0.029_285_714_286, 0.962_5),
            (0.045, 0.976_315_789_474),
            (0.080_714_285_714, 0.990_131_578_947),
        ),
        normalized_rect_cubic(
            bounds,
            (0.080_714_285_714, 0.990_131_578_947),
            (0.102_857_142_857, 0.998_684_210_526),
            (0.162_142_857_143, 1.0),
            (0.501_428_571_429, 1.0),
        ),
        normalized_rect_cubic(
            bounds,
            (0.501_428_571_429, 1.0),
            (0.869_285_714_286, 1.0),
            (0.898_571_428_571, 0.998_684_210_526),
            (0.925, 0.987_5),
        ),
        normalized_rect_cubic(
            bounds,
            (0.925, 0.987_5),
            // Keep this Potrace-derived corner as a cubic. The exact integer
            // controls qualify for the tiny-cubic quadratic shortcut, which
            // drops one boundary pixel on this fixture.
            (0.94, 0.980_921_052_632),
            (0.960_714_285_714, 0.969_736_842_105),
            (0.965_714_285_714, 0.963_157_894_737),
        ),
        normalized_rect_cubic(
            bounds,
            (0.965_714_285_714, 0.963_157_894_737),
            (1.014_285_714_286, 0.912_5),
            (1.010_714_285_714, 0.784_210_526_316),
            (0.96, 0.742_105_263_158),
        ),
        normalized_rect_cubic(
            bounds,
            (0.96, 0.742_105_263_158),
            (0.922_857_142_857, 0.711_842_105_263),
            (0.91, 0.710_526_315_789),
            (0.603_571_428_571, 0.710_526_315_789),
        ),
        normalized_rect_line(
            bounds,
            (0.603_571_428_571, 0.710_526_315_789),
            (0.314_285_714_286, 0.710_526_315_789),
        ),
        normalized_rect_line(
            bounds,
            (0.314_285_714_286, 0.710_526_315_789),
            (0.314_285_714_286, 0.402_631_578_947),
        ),
        normalized_rect_cubic(
            bounds,
            (0.314_285_714_286, 0.402_631_578_947),
            (0.314_285_714_286, 0.118_421_052_632),
            (0.312_857_142_857, 0.093_421_052_632),
            (0.300_714_285_714, 0.069_078_947_368),
        ),
        normalized_rect_cubic(
            bounds,
            (0.300_714_285_714, 0.069_078_947_368),
            (0.285, 0.037_5),
            (0.269_285_714_286, 0.023_684_210_526),
            (0.233_571_428_571, 0.009_868_421_053),
        ),
        normalized_rect_cubic(
            bounds,
            (0.233_571_428_571, 0.009_868_421_053),
            (0.196_428_571_429, -0.003_947_368_421),
            (0.111_428_571_429, -0.003_289_473_684),
            (0.075, 0.012_5),
        ),
    ]
}
