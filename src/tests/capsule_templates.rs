use super::*;

#[test]
fn pixel_diagonal_capsule_uses_narrow_potrace_template() {
    let bitmap = narrow_diagonal_capsule_bitmap();
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.2,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced.paths.first().expect("fixture should trace one path");
    let segments = fit_closed_diagonal_capsule_potrace_segments(&path.points)
        .expect("narrow diagonal capsule should fit a Potrace-derived template");
    let data = path_to_svg_data(
        path,
        SvgRenderOptions {
            curve_mode: CurveMode::Potrace,
            opt_tolerance: 0.2,
            pixel_potrace: true,
        },
        Some((bitmap.width(), bitmap.height())),
        false,
    )
    .expect("narrow diagonal capsule path should render");

    assert_eq!(segments.len(), 6, "{segments:?}");
    assert!(compact_path_command_count(&data) <= 8, "{data}");
}

#[test]
fn pixel_vertical_capsule_prefers_regular_template_when_boundary_is_closer() {
    let bitmap = rounded_rect_union_bitmap(&[
        (160.0, 65.0, 206.0, 205.0, 23.0),
        (46.0, 54.0, 139.0, 125.0, 11.0),
    ]);
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    let path = traced
        .paths
        .iter()
        .find(|path| {
            FloatBounds::from_points(&path.points).is_some_and(|bounds| bounds.min_x > 150.0)
        })
        .expect("fixture should include the vertical capsule");
    let bounds = FloatBounds::from_points(&path.points).unwrap();
    let radius = (bounds.max_x - bounds.min_x) / 2.0;
    let expected = vertical_capsule_segments(bounds, radius);
    let segments = fit_closed_capsule_potrace_segments(&path.points)
        .expect("vertical capsule should fit a capsule primitive");
    let candidate = (segments[0].start(), segments);
    let expected_candidate = (expected[0].start(), expected);

    assert_eq!(
        compact_svg_path_data_from_segments(candidate.0, &candidate.1),
        compact_svg_path_data_from_segments(expected_candidate.0, &expected_candidate.1)
    );
    assert!(
        pixel_potrace_candidate_mask_error(path, &candidate, bitmap.width(), bitmap.height()) <= 12,
        "{}",
        compact_svg_path_data_from_segments(candidate.0, &candidate.1)
    );
}

fn compact_path_command_count(data: &str) -> usize {
    data.chars()
        .filter(|character| {
            matches!(
                character,
                'M' | 'L'
                    | 'H'
                    | 'V'
                    | 'C'
                    | 'S'
                    | 'Q'
                    | 'T'
                    | 'A'
                    | 'Z'
                    | 'm'
                    | 'l'
                    | 'h'
                    | 'v'
                    | 'c'
                    | 's'
                    | 'q'
                    | 't'
                    | 'a'
                    | 'z'
            )
        })
        .count()
}
