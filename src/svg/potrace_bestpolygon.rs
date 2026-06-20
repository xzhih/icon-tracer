const INFTY: i64 = 10_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntPoint {
    x: i64,
    y: i64,
}

pub(crate) fn potrace_best_polygon_indices(points: &[(f64, f64)]) -> Option<Vec<usize>> {
    if points.len() <= 8 {
        return Some((0..points.len()).collect());
    }

    let points = integer_points_from_float(points)?;
    let lon = calc_lon(&points);
    let indices = best_polygon_from_lon(&points, &lon)?;
    (indices.len() >= 3).then_some(indices)
}

fn integer_points_from_float(points: &[(f64, f64)]) -> Option<Vec<IntPoint>> {
    points
        .iter()
        .map(|point| {
            let x = point.0.round();
            let y = point.1.round();
            ((point.0 - x).abs() <= 1.0e-6 && (point.1 - y).abs() <= 1.0e-6).then_some(IntPoint {
                x: x as i64,
                y: y as i64,
            })
        })
        .collect()
}

fn calc_lon(points: &[IntPoint]) -> Vec<usize> {
    let n = points.len();
    let mut pivk = vec![0usize; n];
    let mut nc = vec![0usize; n];
    let mut k = 0usize;

    for i in (0..n).rev() {
        if points[i].x != points[k % n].x && points[i].y != points[k % n].y {
            k = i + 1;
        }
        nc[i] = k;
    }

    for i in (0..n).rev() {
        let mut direction_count = [0usize; 4];
        let first_direction = direction_index(subtract_int(point_at(points, i + 1), points[i]));
        direction_count[first_direction] += 1;

        let mut constraint = [IntPoint { x: 0, y: 0 }; 2];
        let mut k = nc[i];
        let mut k1 = i;

        loop {
            let direction =
                direction_index(subtract_int(point_at(points, k), point_at(points, k1)));
            direction_count[direction] += 1;

            if direction_count.iter().all(|count| *count > 0) {
                pivk[i] = k1 % n;
                break;
            }

            let current = subtract_int(point_at(points, k), points[i]);
            if cross_int(constraint[0], current) < 0 || cross_int(constraint[1], current) > 0 {
                let step = sign_point(subtract_int(point_at(points, k), point_at(points, k1)));
                let previous = subtract_int(point_at(points, k1), points[i]);
                let a = cross_int(constraint[0], previous);
                let b = cross_int(constraint[0], step);
                let c = cross_int(constraint[1], previous);
                let d = cross_int(constraint[1], step);
                let mut offset = INFTY;
                if b < 0 {
                    offset = floor_div(a, -b);
                }
                if d > 0 {
                    offset = offset.min(floor_div(-c, d));
                }
                pivk[i] = mod_index(k1 as i64 + offset, n);
                break;
            }

            if current.x.abs() > 1 || current.y.abs() > 1 {
                let first = IntPoint {
                    x: current.x
                        + if current.y >= 0 && (current.y > 0 || current.x < 0) {
                            1
                        } else {
                            -1
                        },
                    y: current.y
                        + if current.x <= 0 && (current.x < 0 || current.y < 0) {
                            1
                        } else {
                            -1
                        },
                };
                if cross_int(constraint[0], first) >= 0 {
                    constraint[0] = first;
                }

                let second = IntPoint {
                    x: current.x
                        + if current.y <= 0 && (current.y < 0 || current.x < 0) {
                            1
                        } else {
                            -1
                        },
                    y: current.y
                        + if current.x >= 0 && (current.x > 0 || current.y < 0) {
                            1
                        } else {
                            -1
                        },
                };
                if cross_int(constraint[1], second) <= 0 {
                    constraint[1] = second;
                }
            }

            k1 = k;
            k = nc[k1 % n];
            if !cyclic(k, i, k1) {
                pivk[i] = k1 % n;
                break;
            }
        }
    }

    let mut lon = vec![0usize; n];
    let mut j = pivk[n - 1];
    lon[n - 1] = j;

    for i in (0..n - 1).rev() {
        if cyclic(i + 1, pivk[i], j) {
            j = pivk[i];
        }
        lon[i] = j;
    }

    let mut i = n - 1;
    loop {
        if !cyclic((i + 1) % n, j, lon[i]) {
            break;
        }
        lon[i] = j;
        if i == 0 {
            break;
        }
        i -= 1;
    }

    lon
}

fn best_polygon_from_lon(points: &[IntPoint], lon: &[usize]) -> Option<Vec<usize>> {
    let n = points.len();
    let sums = IntPathSums::new(points);
    let mut clip0 = vec![0usize; n];

    for i in 0..n {
        let mut c = mod_index(lon[(i + n - 1) % n] as i64 - 1, n);
        if c == i {
            c = (i + 1) % n;
        }
        clip0[i] = if c < i { n } else { c };
    }

    let mut clip1 = vec![0usize; n + 1];
    let mut j = 1usize;
    for (i, clip) in clip0.iter().enumerate() {
        while j <= *clip && j <= n {
            clip1[j] = i;
            j += 1;
        }
    }

    let mut seg0 = vec![0usize; n + 1];
    let mut i = 0usize;
    let mut segment_count = 0usize;
    while i < n {
        seg0[segment_count] = i;
        i = clip0[i];
        segment_count += 1;
        if segment_count > n {
            return None;
        }
    }
    seg0[segment_count] = n;

    let mut seg1 = vec![0usize; n + 1];
    i = n;
    for j in (1..=segment_count).rev() {
        seg1[j] = i;
        i = clip1[i];
    }
    seg1[0] = 0;

    let mut penalty = vec![f64::INFINITY; n + 1];
    let mut previous = vec![0usize; n + 1];
    penalty[0] = 0.0;

    for j in 1..=segment_count {
        for i in seg1[j]..=seg0[j] {
            let mut best = f64::INFINITY;
            let mut best_previous = seg0[j - 1];
            for k in (clip1[i]..=seg0[j - 1]).rev() {
                let candidate = penalty[k] + penalty3(points, &sums, k, i);
                if candidate < best {
                    best = candidate;
                    best_previous = k;
                }
            }
            penalty[i] = best;
            previous[i] = best_previous;
        }
    }

    let mut indices = vec![0usize; segment_count];
    i = n;
    for j in (0..segment_count).rev() {
        i = previous[i];
        indices[j] = i % n;
    }
    indices.dedup();
    Some(indices)
}

#[derive(Debug)]
struct IntPathSums {
    x: Vec<f64>,
    y: Vec<f64>,
    x2: Vec<f64>,
    xy: Vec<f64>,
    y2: Vec<f64>,
}

impl IntPathSums {
    fn new(points: &[IntPoint]) -> Self {
        let origin = points[0];
        let mut sums = Self {
            x: vec![0.0],
            y: vec![0.0],
            x2: vec![0.0],
            xy: vec![0.0],
            y2: vec![0.0],
        };

        for point in points {
            let x = (point.x - origin.x) as f64;
            let y = (point.y - origin.y) as f64;
            let last = sums.x.len() - 1;
            sums.x.push(sums.x[last] + x);
            sums.y.push(sums.y[last] + y);
            sums.x2.push(sums.x2[last] + x * x);
            sums.xy.push(sums.xy[last] + x * y);
            sums.y2.push(sums.y2[last] + y * y);
        }

        sums
    }
}

fn penalty3(points: &[IntPoint], sums: &IntPathSums, i: usize, mut j: usize) -> f64 {
    let n = points.len();
    let mut rotations = 0.0;
    if j >= n {
        j -= n;
        rotations = 1.0;
    }

    let x = sums.x[j + 1] - sums.x[i] + rotations * sums.x[n];
    let y = sums.y[j + 1] - sums.y[i] + rotations * sums.y[n];
    let x2 = sums.x2[j + 1] - sums.x2[i] + rotations * sums.x2[n];
    let xy = sums.xy[j + 1] - sums.xy[i] + rotations * sums.xy[n];
    let y2 = sums.y2[j + 1] - sums.y2[i] + rotations * sums.y2[n];
    let k = j as f64 + 1.0 - i as f64 + rotations * n as f64;
    let origin = points[0];
    let start = points[i];
    let end = points[j];
    let px = (start.x + end.x) as f64 / 2.0 - origin.x as f64;
    let py = (start.y + end.y) as f64 / 2.0 - origin.y as f64;
    let ey = (end.x - start.x) as f64;
    let ex = -(end.y - start.y) as f64;
    let a = (x2 - 2.0 * x * px) / k + px * px;
    let b = (xy - x * py - y * px) / k + px * py;
    let c = (y2 - 2.0 * y * py) / k + py * py;
    (ex * ex * a + 2.0 * ex * ey * b + ey * ey * c)
        .max(0.0)
        .sqrt()
}

fn point_at(points: &[IntPoint], index: usize) -> IntPoint {
    points[index % points.len()]
}

fn subtract_int(left: IntPoint, right: IntPoint) -> IntPoint {
    IntPoint {
        x: left.x - right.x,
        y: left.y - right.y,
    }
}

fn sign_point(point: IntPoint) -> IntPoint {
    IntPoint {
        x: point.x.signum(),
        y: point.y.signum(),
    }
}

fn cross_int(left: IntPoint, right: IntPoint) -> i64 {
    left.x * right.y - left.y * right.x
}

fn direction_index(vector: IntPoint) -> usize {
    ((3 + 3 * vector.x.signum() + vector.y.signum()) / 2) as usize
}

fn cyclic(a: usize, b: usize, c: usize) -> bool {
    if a <= c {
        a <= b && b < c
    } else {
        a <= b || b < c
    }
}

fn floor_div(numerator: i64, denominator: i64) -> i64 {
    numerator.div_euclid(denominator)
}

fn mod_index(index: i64, len: usize) -> usize {
    index.rem_euclid(len as i64) as usize
}
