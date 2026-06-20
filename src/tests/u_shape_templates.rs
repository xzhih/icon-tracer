use super::*;

#[test]
fn pixel_u_shape_template_accepts_offset_stroke_archetype() {
    let bitmap = offset_u_shape_bitmap();
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
    let segments = fit_closed_staple_potrace_segments(&path.points)
        .expect("offset U shape should fit a Potrace-derived template");
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
    .expect("offset U path should render");

    assert_eq!(segments.len(), 27, "{segments:?}");
    assert!(compact_path_command_count(&data) <= 27, "{data}");
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
