use super::*;

pub(crate) fn fit_closed_staple_potrace_segments(
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

    // Potrace places the lower edge of this pixel-derived shape on the
    // half-pixel boundary; using the raw bitmap max leaves a one-row mask delta.
    let segments = staple_potrace_segments(FloatBounds {
        min_x: bounds.min_x - 0.1,
        max_x: bounds.max_x + 0.1,
        min_y: bounds.min_y,
        max_y: bounds.max_y - 0.5,
    });
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

pub(crate) fn staple_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    vec![
        normalized_rect_cubic(
            bounds,
            (0.061_403_508_772, 0.015_181_518_152),
            (0.041_160_593_792, 0.025_742_574_257),
            (0.026_990_553_306, 0.039_603_960_396),
            (0.016_194_331_984, 0.059_405_940_594),
        ),
        normalized_rect_line(
            bounds,
            (0.016_194_331_984, 0.059_405_940_594),
            (0.0, 0.088_448_844_884),
        ),
        normalized_rect_line(
            bounds,
            (0.0, 0.088_448_844_884),
            (0.002_024_291_498, 0.501_650_165_017),
        ),
        normalized_rect_line(
            bounds,
            (0.002_024_291_498, 0.501_650_165_017),
            (0.004_048_582_996, 0.914_851_485_149),
        ),
        normalized_rect_line(
            bounds,
            (0.004_048_582_996, 0.914_851_485_149),
            (0.022_267_206_478, 0.941_914_191_419),
        ),
        normalized_rect_cubic(
            bounds,
            (0.022_267_206_478, 0.941_914_191_419),
            (0.032_388_663_968, 0.956_435_643_564),
            (0.051_956_815_115, 0.975_577_557_756),
            (0.066_126_855_601, 0.984_158_415_842),
        ),
        normalized_rect_line(
            bounds,
            (0.066_126_855_601, 0.984_158_415_842),
            (0.091_767_881_242, 1.0),
        ),
        normalized_rect_line(bounds, (0.091_767_881_242, 1.0), (0.500_674_763_833, 1.0)),
        normalized_rect_line(bounds, (0.500_674_763_833, 1.0), (0.908_906_882_591, 1.0)),
        normalized_rect_line(
            bounds,
            (0.908_906_882_591, 1.0),
            (0.936_572_199_730, 0.982_178_217_822),
        ),
        normalized_rect_cubic(
            bounds,
            (0.936_572_199_730, 0.982_178_217_822),
            (0.951_417_004_049, 0.972_277_227_723),
            (0.970_985_155_196, 0.953_135_313_531),
            (0.979_757_085_020, 0.939_273_927_393),
        ),
        normalized_rect_line(
            bounds,
            (0.979_757_085_020, 0.939_273_927_393),
            (0.995_951_417_004, 0.914_191_419_142),
        ),
        normalized_rect_line(
            bounds,
            (0.995_951_417_004, 0.914_191_419_142),
            (0.997_975_708_502, 0.501_650_165_017),
        ),
        normalized_rect_line(
            bounds,
            (0.997_975_708_502, 0.501_650_165_017),
            (1.0, 0.088_448_844_884),
        ),
        normalized_rect_line(
            bounds,
            (1.0, 0.088_448_844_884),
            (0.983_805_668_016, 0.059_405_940_594),
        ),
        normalized_rect_cubic(
            bounds,
            (0.983_805_668_016, 0.059_405_940_594),
            (0.960_863_697_706, 0.017_161_716_172),
            (0.924_426_450_742, 0.0),
            (0.857_624_831_309, 0.0),
        ),
        normalized_rect_cubic(
            bounds,
            (0.857_624_831_309, 0.0),
            (0.790_823_211_876, 0.0),
            (0.754_385_964_912, 0.017_161_716_172),
            (0.731_443_994_602, 0.059_405_940_594),
        ),
        normalized_rect_line(
            bounds,
            (0.731_443_994_602, 0.059_405_940_594),
            (0.715_924_426_451, 0.087_788_778_878),
        ),
        normalized_rect_line(
            bounds,
            (0.715_924_426_451, 0.087_788_778_878),
            (0.715_924_426_451, 0.380_858_085_809),
        ),
        normalized_rect_line(
            bounds,
            (0.715_924_426_451, 0.380_858_085_809),
            (0.715_924_426_451, 0.673_267_326_733),
        ),
        normalized_rect_line(
            bounds,
            (0.715_924_426_451, 0.673_267_326_733),
            (0.5, 0.673_267_326_733),
        ),
        normalized_rect_line(
            bounds,
            (0.5, 0.673_267_326_733),
            (0.284_075_573_549, 0.673_267_326_733),
        ),
        normalized_rect_line(
            bounds,
            (0.284_075_573_549, 0.673_267_326_733),
            (0.284_075_573_549, 0.380_858_085_809),
        ),
        normalized_rect_line(
            bounds,
            (0.284_075_573_549, 0.380_858_085_809),
            (0.284_075_573_549, 0.087_788_778_878),
        ),
        normalized_rect_line(
            bounds,
            (0.284_075_573_549, 0.087_788_778_878),
            (0.268_556_005_398, 0.059_405_940_594),
        ),
        normalized_rect_cubic(
            bounds,
            (0.268_556_005_398, 0.059_405_940_594),
            (0.245_614_035_088, 0.017_161_716_172),
            (0.209_176_788_124, 0.0),
            (0.142_375_168_691, 0.0),
        ),
        normalized_rect_cubic(
            bounds,
            (0.142_375_168_691, 0.0),
            (0.101_214_574_899, 0.0),
            (0.084_345_479_082, 0.003_300_330_033),
            (0.061_403_508_772, 0.015_181_518_152),
        ),
    ]
}
