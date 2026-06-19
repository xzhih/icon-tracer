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

    // Match Potrace's integer-control bounds for pixel-sampled open-ring icons.
    let segments = open_ring_potrace_segments(FloatBounds {
        min_x: bounds.min_x - 0.1,
        max_x: bounds.max_x + 0.1,
        min_y: bounds.min_y - 0.3,
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

pub(crate) fn open_ring_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    vec![
        normalized_rect_cubic(
            bounds,
            (0.425_775_978_408, 0.011_553_273_427),
            (0.224_696_356_275, 0.051_989_730_424),
            (0.069_500_674_764, 0.192_554_557_125),
            (0.016_194_331_984, 0.380_616_174_583),
        ),
        normalized_rect_cubic(
            bounds,
            (0.016_194_331_984, 0.380_616_174_583),
            (0.0, 0.437_740_693_196),
            (0.0, 0.567_394_094_994),
            (0.016_194_331_984, 0.624_518_613_607),
        ),
        normalized_rect_cubic(
            bounds,
            (0.016_194_331_984, 0.624_518_613_607),
            (0.056_680_161_943, 0.767_650_834_403),
            (0.157_219_973_009, 0.884_467_265_725),
            (0.294_197_031_039, 0.949_293_966_624),
        ),
        normalized_rect_cubic(
            bounds,
            (0.294_197_031_039, 0.949_293_966_624),
            (0.381_241_565_452, 0.989_730_423_62),
            (0.426_450_742_24, 1.0),
            (0.526_990_553_306, 1.0),
        ),
        normalized_rect_cubic(
            bounds,
            (0.526_990_553_306, 1.0),
            (0.624_831_309_042, 1.0),
            (0.670_715_249_663, 0.990_372_272_144),
            (0.753_036_437_247, 0.952_503_209_243),
        ),
        normalized_rect_cubic(
            bounds,
            (0.753_036_437_247, 0.952_503_209_243),
            (0.850_877_192_982, 0.907_573_812_58),
            (0.935_222_672_065, 0.834_403_080_873),
            (0.981_781_376_518, 0.752_888_318_357),
        ),
        normalized_rect_line(
            bounds,
            (0.981_781_376_518, 0.752_888_318_357),
            (1.0, 0.720_795_892_169),
        ),
        normalized_rect_line(
            bounds,
            (1.0, 0.720_795_892_169),
            (0.854_925_775_978, 0.720_795_892_169),
        ),
        normalized_rect_cubic(
            bounds,
            (0.854_925_775_978, 0.720_795_892_169),
            (0.763_832_658_57, 0.720_795_892_169),
            (0.708_502_024_291, 0.723_363_286_264),
            (0.705_802_968_961, 0.727_214_377_407),
        ),
        normalized_rect_cubic(
            bounds,
            (0.705_802_968_961, 0.727_214_377_407),
            (0.699_055_330_634, 0.737_483_953_787),
            (0.634_952_766_532, 0.767_008_985_879),
            (0.597_840_755_735, 0.775_994_865_212),
        ),
        normalized_rect_cubic(
            bounds,
            (0.597_840_755_735, 0.775_994_865_212),
            (0.417_004_048_583, 0.822_207_958_922),
            (0.230_094_466_937, 0.682_926_829_268),
            (0.230_094_466_937, 0.502_567_394_095),
        ),
        normalized_rect_cubic(
            bounds,
            (0.230_094_466_937, 0.502_567_394_095),
            (0.230_094_466_937, 0.401_797_175_866),
            (0.294_871_794_872, 0.299_101_412_067),
            (0.385_964_912_281, 0.254_813_863_928),
        ),
        normalized_rect_cubic(
            bounds,
            (0.385_964_912_281, 0.254_813_863_928),
            (0.481_106_612_686, 0.209_242_618_742),
            (0.572_199_730_094, 0.208_600_770_218),
            (0.666_666_666_667, 0.254_172_015_404),
        ),
        normalized_rect_cubic(
            bounds,
            (0.666_666_666_667, 0.254_172_015_404),
            (0.685_560_053_981, 0.263_799_743_261),
            (0.703_103_913_63, 0.274_069_319_641),
            (0.705_802_968_961, 0.277_920_410_783),
        ),
        normalized_rect_cubic(
            bounds,
            (0.705_802_968_961, 0.277_920_410_783),
            (0.708_502_024_291, 0.281_771_501_926),
            (0.763_832_658_57, 0.284_338_896_021),
            (0.854_925_775_978, 0.284_338_896_021),
        ),
        normalized_rect_line(
            bounds,
            (0.854_925_775_978, 0.284_338_896_021),
            (1.0, 0.284_338_896_021),
        ),
        normalized_rect_line(
            bounds,
            (1.0, 0.284_338_896_021),
            (0.981_781_376_518, 0.252_246_469_833),
        ),
        normalized_rect_cubic(
            bounds,
            (0.981_781_376_518, 0.252_246_469_833),
            (0.954_116_059_379, 0.204_107_830_552),
            (0.898_110_661_269, 0.142_490_372_272),
            (0.850_202_429_15, 0.107_830_551_99),
        ),
        normalized_rect_cubic(
            bounds,
            (0.850_202_429_15, 0.107_830_551_99),
            (0.799_595_141_7, 0.071_887_034_66),
            (0.715_924_426_451, 0.032_092_426_187),
            (0.658_569_500_675, 0.017_329_910_141),
        ),
        normalized_rect_cubic(
            bounds,
            (0.658_569_500_675, 0.017_329_910_141),
            (0.602_564_102_564, 0.003_209_242_619),
            (0.480_431_848_853, 0.0),
            (0.425_775_978_408, 0.011_553_273_427),
        ),
    ]
}
