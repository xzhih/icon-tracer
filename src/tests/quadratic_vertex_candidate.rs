use super::*;

#[test]
fn quadratic_vertex_candidate_rejects_polygon_boundary_regression() {
    let bitmap =
        local_polygon_bitmap(&[(58.0, 62.0), (194.0, 46.0), (212.0, 194.0), (42.0, 210.0)]);
    let path = trace_first_path(&bitmap);
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let best = pixel_potrace_segments_for_points(&path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce base candidate");
    let quadratic = quadratic_vertex_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce quadratic candidate");

    assert!(!pixel_potrace_quadratic_vertex_candidate_is_better(
        &path,
        canvas_size,
        &quadratic,
        &best,
    ));

    let selected = choose_pixel_potrace_point_set(&path, 0.2, canvas_size, false)
        .expect("fixture should produce selected candidate");
    assert_ne!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(quadratic.0, &quadratic.1)
    );
}

#[test]
fn quadratic_vertex_candidate_defers_to_low_angle_capsule_template() {
    let bitmap = local_capsule_bitmap((38.0, 184.0), (218.0, 72.0), 17.0);
    let path = trace_first_path(&bitmap);
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let best = pixel_potrace_segments_for_points(
        &path,
        &path.points,
        PIXEL_POTRACE_FINE_OPT_TOLERANCE,
        canvas_size,
        false,
    )
    .expect("fixture should produce fine candidate");
    let quadratic = quadratic_vertex_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce quadratic candidate");
    let primitive = fit_closed_diagonal_capsule_potrace_segments(&path.points)
        .expect("fixture should fit a low-angle diagonal capsule template");
    let primitive_candidate = (primitive[0].start(), primitive);

    assert_eq!(primitive_candidate.1.len(), 6);
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(best.0, &best.1),
        compact_svg_path_data_from_segments_without_arcs(
            primitive_candidate.0,
            &primitive_candidate.1
        )
    );
    assert!(!pixel_potrace_quadratic_vertex_candidate_is_better(
        &path,
        canvas_size,
        &quadratic,
        &best,
    ));

    let selected = choose_pixel_potrace_point_set(&path, 0.2, canvas_size, false)
        .expect("fixture should produce selected candidate");
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(
            primitive_candidate.0,
            &primitive_candidate.1
        )
    );
}

#[test]
fn quadratic_vertex_candidate_defers_to_medium_angle_capsule_template() {
    let bitmap = local_capsule_bitmap((38.0, 190.0), (164.0, 54.0), 22.0);
    let path = trace_first_path(&bitmap);
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let best = pixel_potrace_segments_for_points(&path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce base candidate");
    let quadratic = quadratic_vertex_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce quadratic candidate");
    let primitive = fit_closed_diagonal_capsule_potrace_segments(&path.points)
        .expect("fixture should fit a medium-angle diagonal capsule template");
    let primitive_candidate = (primitive[0].start(), primitive);

    assert_eq!(primitive_candidate.1.len(), 6);
    assert_eq!(
        compact_svg_path_data_from_segments_without_arcs(best.0, &best.1),
        compact_svg_path_data_from_segments_without_arcs(
            primitive_candidate.0,
            &primitive_candidate.1
        )
    );
    assert!(!pixel_potrace_quadratic_vertex_candidate_is_better(
        &path,
        canvas_size,
        &quadratic,
        &best,
    ));
    assert!(
        pixel_potrace_candidate_mask_error(&path, &best, bitmap.width(), bitmap.height())
            <= pixel_potrace_candidate_mask_error(
                &path,
                &quadratic,
                bitmap.width(),
                bitmap.height()
            )
    );
}

#[test]
fn quadratic_vertex_candidate_accepts_concave_polygon_smoothing_rescue() {
    let bitmap = sharp_v_polygon_bitmap();
    let path = trace_first_path(&bitmap);
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let best = pixel_potrace_segments_for_points(&path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce base candidate");
    let quadratic = quadratic_vertex_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce quadratic candidate");

    assert!(pixel_potrace_quadratic_vertex_candidate_is_better(
        &path,
        canvas_size,
        &quadratic,
        &best,
    ));
    assert!(
        pixel_potrace_candidate_mask_error(&path, &quadratic, bitmap.width(), bitmap.height())
            < pixel_potrace_candidate_mask_error(&path, &best, bitmap.width(), bitmap.height())
    );
}

#[test]
fn quadratic_vertex_candidate_rejects_rounded_union_canaries() {
    let fixtures = [
        rounded_rect_union_bitmap(&[
            (106.0, 42.0, 150.0, 212.0, 17.0),
            (58.0, 64.0, 198.0, 112.0, 20.0),
        ]),
        rounded_rect_union_bitmap(&[
            (48.0, 50.0, 94.0, 204.0, 18.0),
            (162.0, 50.0, 208.0, 204.0, 18.0),
            (48.0, 112.0, 208.0, 152.0, 16.0),
        ]),
        rounded_rect_union_bitmap(&[
            (84.0, 50.0, 203.0, 195.0, 12.0),
            (74.0, 95.0, 116.0, 149.0, 18.0),
            (98.0, 186.0, 159.0, 219.0, 10.0),
        ]),
    ];

    for bitmap in fixtures {
        let path = trace_first_path(&bitmap);
        let canvas_size = Some((bitmap.width(), bitmap.height()));
        let best = pixel_potrace_segments_for_points(&path, &path.points, 0.2, canvas_size, false)
            .expect("fixture should produce base candidate");
        let quadratic = quadratic_vertex_pixel_potrace_segments_for_points(&path.points, 0.2)
            .expect("fixture should produce quadratic candidate");

        assert!(!pixel_potrace_quadratic_vertex_candidate_is_better(
            &path,
            canvas_size,
            &quadratic,
            &best,
        ));
    }
}

#[test]
fn quadratic_vertex_candidate_rejects_shallow_capsule_canary() {
    let bitmap = local_capsule_bitmap((42.0, 104.0), (218.0, 154.0), 16.0);
    let path = trace_first_path(&bitmap);
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let best = pixel_potrace_segments_for_points(&path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce base candidate");
    let quadratic = quadratic_vertex_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce quadratic candidate");

    assert!(!pixel_potrace_quadratic_vertex_candidate_is_better(
        &path,
        canvas_size,
        &quadratic,
        &best,
    ));

    let selected = choose_pixel_potrace_point_set(&path, 0.2, canvas_size, false)
        .expect("fixture should produce selected candidate");
    assert_ne!(
        compact_svg_path_data_from_segments_without_arcs(selected.0, &selected.1),
        compact_svg_path_data_from_segments_without_arcs(quadratic.0, &quadratic.1)
    );
}

#[test]
fn quadratic_vertex_candidate_rejects_ring_sector_canary() {
    let bitmap = ring_sector_bitmap(70.0, 290.0, 38.0, 80.0);
    let path = trace_first_path(&bitmap);
    let canvas_size = Some((bitmap.width(), bitmap.height()));
    let best = pixel_potrace_segments_for_points(&path, &path.points, 0.2, canvas_size, false)
        .expect("fixture should produce base candidate");
    let quadratic = quadratic_vertex_pixel_potrace_segments_for_points(&path.points, 0.2)
        .expect("fixture should produce quadratic candidate");

    assert!(!pixel_potrace_quadratic_vertex_candidate_is_better(
        &path,
        canvas_size,
        &quadratic,
        &best,
    ));
}

fn trace_first_path(bitmap: &Bitmap) -> TracePath {
    let traced = trace_bitmap(
        bitmap,
        TraceOptions {
            turd_size: 2,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: true,
        },
    );
    traced
        .paths
        .first()
        .expect("fixture should trace one path")
        .clone()
}

fn local_capsule_bitmap(start: (f64, f64), end: (f64, f64), half_width: f64) -> Bitmap {
    const CANVAS: usize = 256;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                distance_squared_to_segment((x as f64 + 0.5, y as f64 + 0.5), start, end)
                    .0
                    .sqrt()
                    <= half_width
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn local_polygon_bitmap(points: &[(f64, f64)]) -> Bitmap {
    const CANVAS: usize = 256;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                local_point_is_inside_polygon((x as f64 + 0.5, y as f64 + 0.5), points)
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn local_point_is_inside_polygon(point: (f64, f64), points: &[(f64, f64)]) -> bool {
    let mut hit = false;
    let mut previous = points.len() - 1;

    for index in 0..points.len() {
        let (x1, y1) = points[index];
        let (x0, y0) = points[previous];
        if (y1 > point.1) != (y0 > point.1) {
            let crossing = (x0 - x1) * (point.1 - y1) / (y0 - y1) + x1;
            if point.0 < crossing {
                hit = !hit;
            }
        }
        previous = index;
    }

    hit
}
