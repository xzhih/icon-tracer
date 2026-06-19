use super::*;

pub(super) fn parity_triangle_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let top = (128.0, 42.0);
    let right = (214.0, 214.0);
    let left = (42.0, 214.0);
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                point_is_inside_triangle((x as f64 + 0.5, y as f64 + 0.5), top, right, left)
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

pub(super) fn parity_ring_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let center = (128.0, 128.0);
    let outer_radius_squared = 78.0_f64 * 78.0;
    let inner_radius_squared = 42.0_f64 * 42.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let delta = (x as f64 + 0.5 - center.0, y as f64 + 0.5 - center.1);
                let distance_squared = delta.0 * delta.0 + delta.1 * delta.1;
                inner_radius_squared < distance_squared && distance_squared <= outer_radius_squared
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

pub(super) fn parity_c_shape_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let center = (128.0, 128.0);
    let outer_radius_squared = 78.0_f64 * 78.0;
    let inner_radius_squared = 44.0_f64 * 44.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                let delta = (point.0 - center.0, point.1 - center.1);
                let distance_squared = delta.0 * delta.0 + delta.1 * delta.1;
                inner_radius_squared < distance_squared
                    && distance_squared <= outer_radius_squared
                    && !(point.0 > center.0 && delta.1.abs() < 34.0)
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

pub(super) fn parity_f_shape_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    const RUNS: &[(usize, usize, usize, usize)] = &[
        (62, 62, 98, 181),
        (63, 63, 96, 183),
        (64, 64, 95, 184),
        (65, 65, 94, 185),
        (66, 67, 93, 186),
        (68, 87, 92, 187),
        (88, 89, 92, 186),
        (90, 90, 92, 185),
        (91, 91, 92, 184),
        (92, 92, 92, 183),
        (93, 93, 92, 181),
        (94, 94, 68, 125),
        (95, 95, 66, 125),
        (96, 96, 65, 125),
        (97, 97, 64, 125),
        (98, 99, 63, 125),
        (100, 119, 62, 125),
        (120, 121, 63, 125),
        (122, 122, 64, 125),
        (123, 123, 65, 125),
        (124, 124, 66, 125),
        (125, 125, 68, 125),
        (126, 126, 92, 181),
        (127, 127, 92, 183),
        (128, 128, 92, 184),
        (129, 129, 92, 185),
        (130, 131, 92, 186),
        (132, 151, 92, 187),
        (152, 153, 92, 186),
        (154, 154, 92, 185),
        (155, 155, 92, 184),
        (156, 156, 92, 183),
        (157, 157, 92, 181),
        (158, 158, 68, 125),
        (159, 159, 66, 125),
        (160, 160, 65, 125),
        (161, 161, 64, 125),
        (162, 163, 63, 125),
        (164, 187, 62, 125),
        (188, 189, 63, 124),
        (190, 190, 64, 123),
        (191, 191, 65, 122),
        (192, 192, 66, 121),
        (193, 193, 68, 119),
    ];
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                RUNS.iter().any(|(top, bottom, left, right)| {
                    (*top..=*bottom).contains(&y) && (*left..=*right).contains(&x)
                })
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

pub(super) fn parity_e_shape_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let rects = [
        (58.0, 52.0, 104.0, 204.0, 16.0),
        (58.0, 52.0, 198.0, 96.0, 16.0),
        (58.0, 106.0, 182.0, 150.0, 16.0),
        (58.0, 160.0, 198.0, 204.0, 16.0),
    ];
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                rects.iter().any(|(left, top, right, bottom, radius)| {
                    let nearest_x = point.0.clamp(left + radius, right - radius);
                    let nearest_y = point.1.clamp(top + radius, bottom - radius);
                    (point.0 - nearest_x).powi(2) + (point.1 - nearest_y).powi(2) <= radius * radius
                })
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

pub(super) fn parity_two_circles_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let left_center = (84.0, 128.0);
    let right_center = (172.0, 128.0);
    let radius_squared = 42.0_f64 * 42.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                let left_delta = (point.0 - left_center.0, point.1 - left_center.1);
                let right_delta = (point.0 - right_center.0, point.1 - right_center.1);
                left_delta.0 * left_delta.0 + left_delta.1 * left_delta.1 <= radius_squared
                    || right_delta.0 * right_delta.0 + right_delta.1 * right_delta.1
                        <= radius_squared
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

pub(super) fn parity_diagonal_bar_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let start = (62.0, 186.0);
    let end = (194.0, 70.0);
    let half_width = 18.0;
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

pub(super) fn parity_chevron_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let left = (70.0, 70.0);
    let bottom = (128.0, 186.0);
    let right = (186.0, 70.0);
    let half_width = 16.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                distance_squared_to_segment(point, left, bottom).0.sqrt() <= half_width
                    || distance_squared_to_segment(point, bottom, right).0.sqrt() <= half_width
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

pub(super) fn parity_rounded_rect_bitmap(radius: f64) -> Bitmap {
    const CANVAS: usize = 256;
    let left = 54.0;
    let top = 62.0;
    let right = 202.0;
    let bottom = 194.0;
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                let nearest_x = point.0.clamp(left + radius, right - radius);
                let nearest_y = point.1.clamp(top + radius, bottom - radius);
                (point.0 - nearest_x).powi(2) + (point.1 - nearest_y).powi(2) <= radius * radius
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

pub(super) fn parity_u_shape_bitmap() -> Bitmap {
    const CANVAS: usize = 256;
    let rects = [
        (54.0, 50.0, 96.0, 194.0, 17.0),
        (160.0, 50.0, 202.0, 194.0, 17.0),
        (54.0, 152.0, 202.0, 202.0, 20.0),
    ];
    let pixels = (0..CANVAS)
        .flat_map(|y| {
            (0..CANVAS).map(move |x| {
                let point = (x as f64 + 0.5, y as f64 + 0.5);
                rects.iter().any(|(left, top, right, bottom, radius)| {
                    let nearest_x = point.0.clamp(left + radius, right - radius);
                    let nearest_y = point.1.clamp(top + radius, bottom - radius);
                    (point.0 - nearest_x).powi(2) + (point.1 - nearest_y).powi(2) <= radius * radius
                })
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(CANVAS, CANVAS, &pixels).expect("fixture pixels should match canvas")
}

fn point_is_inside_triangle(
    point: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
) -> bool {
    fn sign(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> f64 {
        (p1.0 - p3.0) * (p2.1 - p3.1) - (p2.0 - p3.0) * (p1.1 - p3.1)
    }

    let d1 = sign(point, a, b);
    let d2 = sign(point, b, c);
    let d3 = sign(point, c, a);
    !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
}
