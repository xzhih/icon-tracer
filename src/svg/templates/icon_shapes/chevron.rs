use super::*;

pub(crate) fn fit_closed_chevron_potrace_segments(
    points: &[(f64, f64)],
) -> Option<Vec<SvgPathSegment>> {
    const MIN_AXIS: f64 = 24.0;
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

    let template_bounds = FloatBounds {
        min_x: bounds.min_x,
        max_x: bounds.max_x,
        min_y: bounds.min_y - height * 0.033_783_783_784,
        max_y: bounds.max_y + height * 0.007_432_432_432,
    };
    let segments = chevron_potrace_segments(template_bounds);
    let candidate = (segments[0].start(), segments);
    let candidate_boundary_error = pixel_potrace_candidate_boundary_rms_error(
        &TracePath {
            points: points.to_vec(),
            is_hole: false,
        },
        &candidate,
    );

    (candidate_boundary_error <= MAX_TEMPLATE_BOUNDARY_ERROR).then_some(candidate.1)
}

pub(crate) fn chevron_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    let points = [
        [
            (0.064_189_189_189, 0.041_531_473_069),
            (0.025, 0.058_403_634_004),
            (0.0, 0.096_690_460_740),
            (0.0, 0.139_519_792_343),
        ],
        [
            (0.0, 0.139_519_792_343),
            (0.0, 0.171_966_255_678),
            (0.402_702_702_703, 0.945_489_941_596),
            (0.431_756_756_757, 0.968_851_395_198),
        ],
        [
            (0.431_756_756_757, 0.968_851_395_198),
            (0.470_270_270_270, 1.0),
            (0.529_729_729_730, 1.0),
            (0.568_243_243_243, 0.968_851_395_198),
        ],
        [
            (0.568_243_243_243, 0.968_851_395_198),
            (0.597_972_972_973, 0.944_841_012_330),
            (1.0, 0.171_966_255_678),
            (1.0, 0.138_870_863_076),
        ],
        [
            (1.0, 0.138_870_863_076),
            (1.0, 0.049_318_624_270),
            (0.891_216_216_216, 0.0),
            (0.822_297_297_297, 0.058_403_634_004),
        ],
        [
            (0.822_297_297_297, 0.058_403_634_004),
            (0.806_756_756_757, 0.071_382_219_338),
            (0.760_135_135_135, 0.154_445_165_477),
            (0.652_027_027_027, 0.362_751_460_091),
        ],
        [
            (0.652_027_027_027, 0.362_751_460_091),
            (0.570_270_270_270, 0.520_441_271_901),
            (0.502_027_027_027, 0.648_929_266_710),
            (0.5, 0.648_929_266_710),
        ],
        [
            (0.5, 0.648_929_266_710),
            (0.497_972_972_973, 0.648_929_266_710),
            (0.429_729_729_730, 0.520_441_271_901),
            (0.347_972_972_973, 0.363_400_389_358),
        ],
        [
            (0.347_972_972_973, 0.363_400_389_358),
            (0.266_216_216_216, 0.205_710_577_547),
            (0.191_891_891_892, 0.070_084_360_805),
            (0.182_432_432_432, 0.061_648_280_337),
        ],
        [
            (0.182_432_432_432, 0.061_648_280_337),
            (0.150_675_675_676, 0.032_446_463_335),
            (0.104_054_054_054, 0.024_659_312_135),
            (0.064_189_189_189, 0.041_531_473_069),
        ],
    ];

    normalized_rect_cubic_segments(bounds, &points)
}
