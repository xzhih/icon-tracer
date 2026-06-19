use super::*;

pub(crate) fn svg_path_data_from_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    let mut data = String::new();
    data.push_str(&format!(
        "M {} {}",
        format_float(start.0),
        format_float(start.1)
    ));

    for segment in segments {
        match segment {
            SvgPathSegment::Line { end, .. } => {
                data.push_str(&format!(
                    " L {} {}",
                    format_float(end.0),
                    format_float(end.1)
                ));
            }
            SvgPathSegment::Cubic(cubic) => {
                data.push_str(&format!(
                    " C {} {}, {} {}, {} {}",
                    format_float(cubic.control1.0),
                    format_float(cubic.control1.1),
                    format_float(cubic.control2.0),
                    format_float(cubic.control2.1),
                    format_float(cubic.end.0),
                    format_float(cubic.end.1)
                ));
            }
        }
    }

    data.push_str(" Z");
    data
}

pub(crate) fn compact_svg_path_data_from_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    compact_svg_path_data_from_segments_with_arc_mode(start, segments, true)
}

pub(crate) fn compact_svg_path_data_from_segments_without_arcs(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    compact_svg_path_data_from_segments_with_arc_mode(start, segments, false)
}

pub(crate) fn compact_svg_path_data_from_segments_with_arc_mode(
    start: (f64, f64),
    segments: &[SvgPathSegment],
    allow_arcs: bool,
) -> String {
    let mut best = compact_svg_path_data_for_order(start, segments, allow_arcs);

    if compact_segments_are_closed(start, segments) {
        for offset in 1..segments.len() {
            let rotated = rotate_segments_at(segments, offset);
            let candidate =
                compact_svg_path_data_for_order(rotated[0].start(), &rotated, allow_arcs);
            if candidate.len() < best.len() {
                best = candidate;
            }
        }
    }

    best
}

pub(crate) fn compact_svg_path_data_for_order(
    start: (f64, f64),
    segments: &[SvgPathSegment],
    allow_arcs: bool,
) -> String {
    let segments = compact_segments_without_redundant_closing_line(start, segments);
    let absolute = minify_compact_svg_path_data(&compact_absolute_svg_path_data_from_segments(
        start, segments,
    ));
    let relative = minify_compact_svg_path_data(&compact_relative_svg_path_data_from_segments(
        start, segments,
    ));
    let smooth = minify_compact_svg_path_data(
        &compact_smooth_relative_svg_path_data_from_segments(start, segments),
    );
    let quadratic = minify_compact_svg_path_data(
        &compact_quadratic_relative_svg_path_data_from_segments(start, segments),
    );
    let arc = allow_arcs
        .then(|| compact_circle_arc_svg_path_data_from_segments(segments))
        .flatten()
        .map(|data| minify_compact_svg_path_data(&data));
    let potrace_circle = (!allow_arcs)
        .then(|| compact_potrace_like_circle_svg_path_data_from_segments(segments))
        .flatten()
        .map(|data| minify_compact_svg_path_data(&data));
    let axis_smooth = minify_compact_svg_path_data(
        &compact_axis_smooth_relative_svg_path_data_from_segments(start, segments),
    );

    let mut candidates = vec![absolute, relative, smooth, quadratic];
    if let Some(arc) = arc {
        candidates.push(arc);
    }
    if let Some(potrace_circle) = potrace_circle {
        candidates.push(potrace_circle);
    }

    let mut best = candidates
        .into_iter()
        .min_by_key(String::len)
        .expect("compact path candidates should not be empty");

    if axis_smooth.len() < best.len()
        && compact_path_command_count(&axis_smooth) <= compact_path_command_count(&best)
    {
        best = axis_smooth;
    }

    best
}

pub(crate) fn compact_segments_without_redundant_closing_line(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> &[SvgPathSegment] {
    let mut trimmed = segments;
    let Some(SvgPathSegment::Line {
        start: line_start,
        end,
    }) = trimmed.last()
    else {
        return trimmed;
    };

    if trimmed.len() > 1
        && distance_squared_float(*end, start) <= 1.0e-9
        && distance_squared_float(trimmed[trimmed.len() - 2].end(), *line_start) <= 1.0e-9
    {
        trimmed = &trimmed[..trimmed.len() - 1];
    }

    let Some(SvgPathSegment::Line {
        start: line_start,
        end,
    }) = trimmed.last()
    else {
        return trimmed;
    };

    if trimmed.len() > 1 && closing_line_continues_last_line(*line_start, *end, start) {
        &trimmed[..trimmed.len() - 1]
    } else {
        trimmed
    }
}

pub(crate) fn closing_line_continues_last_line(
    line_start: (f64, f64),
    line_end: (f64, f64),
    close_end: (f64, f64),
) -> bool {
    let line = subtract(line_end, line_start);
    let close = subtract(close_end, line_end);

    vector_length_squared(line) > f64::EPSILON
        && vector_length_squared(close) > f64::EPSILON
        && cross(line, close).abs() <= 1.0e-9
        && dot(line, close) > 0.0
}

pub(crate) fn compact_circle_arc_svg_path_data_from_segments(
    segments: &[SvgPathSegment],
) -> Option<String> {
    const MIN_AXIS: f64 = 8.0;
    const RADIUS_X_INSET: f64 = 0.15;
    const RADIUS_Y_INSET: f64 = 0.1;

    let (center, radius) = fitted_circle_from_segments(segments)?;
    if radius < MIN_AXIS {
        return None;
    }

    let radius_x = (radius - RADIUS_X_INSET).max(MIN_AXIS);
    let radius_y = (radius - RADIUS_Y_INSET).max(MIN_AXIS);
    let left = (center.0 - radius_x, center.1);
    let diameter_x = radius_x * 2.0;

    Some(format!(
        "M {} {} a {} {} 0 1 0 {} 0 a {} {} 0 1 0 {} 0 Z",
        format_compact_float(left.0),
        format_compact_float(left.1),
        format_compact_float(radius_x),
        format_compact_float(radius_y),
        format_compact_float(diameter_x),
        format_compact_float(radius_x),
        format_compact_float(radius_y),
        format_compact_float(-diameter_x),
    ))
}

pub(crate) fn compact_potrace_like_circle_svg_path_data_from_segments(
    segments: &[SvgPathSegment],
) -> Option<String> {
    let (center, radius) = fitted_circle_from_segments(segments)?;
    let center = (
        snap_near_integer_circle_value(center.0),
        snap_near_integer_circle_value(center.1),
    );
    let radius = snap_near_integer_circle_value(radius);
    let segments = potrace_like_ellipse_segments(center, radius, radius);
    Some(compact_relative_svg_path_data_from_segments(
        segments[0].start(),
        &segments,
    ))
}

pub(crate) fn snap_near_integer_circle_value(value: f64) -> f64 {
    const MAX_SNAP_DISTANCE: f64 = 0.25;

    let nearest = value.round();
    if (value - nearest).abs() <= MAX_SNAP_DISTANCE {
        nearest
    } else {
        value
    }
}

pub(crate) fn fitted_circle_from_segments(
    segments: &[SvgPathSegment],
) -> Option<((f64, f64), f64)> {
    const MIN_CIRCLE_SEGMENTS: usize = 5;
    const MAX_RADIUS_ERROR: f64 = 0.02;

    if segments.len() < MIN_CIRCLE_SEGMENTS
        || !segments
            .iter()
            .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        return None;
    }

    let endpoints = segments
        .iter()
        .map(|segment| segment.start())
        .collect::<Vec<_>>();
    let center = arc_centroid(&endpoints);
    let radius = endpoints
        .iter()
        .map(|point| distance(*point, center))
        .sum::<f64>()
        / endpoints.len() as f64;

    for point in endpoints {
        if ((distance(point, center) - radius) / radius).abs() > MAX_RADIUS_ERROR {
            return None;
        }
    }

    Some((center, radius))
}

pub(crate) fn compact_segments_are_closed(start: (f64, f64), segments: &[SvgPathSegment]) -> bool {
    const EPSILON: f64 = 1.0e-9;

    if segments.len() < 2
        || distance_squared_float(start, segments[0].start()) > EPSILON
        || distance_squared_float(segments[segments.len() - 1].end(), start) > EPSILON
    {
        return false;
    }

    segments
        .windows(2)
        .all(|window| distance_squared_float(window[0].end(), window[1].start()) <= EPSILON)
}

pub(crate) fn rotate_segments_at(
    segments: &[SvgPathSegment],
    offset: usize,
) -> Vec<SvgPathSegment> {
    segments[offset..]
        .iter()
        .chain(segments[..offset].iter())
        .copied()
        .collect()
}

pub(crate) fn minify_compact_svg_path_data(data: &str) -> String {
    let mut minified = String::new();
    let mut previous: Option<&str> = None;

    for token in data.split_whitespace() {
        if previous.is_some_and(|previous| compact_path_tokens_need_separator(previous, token)) {
            minified.push(' ');
        }
        minified.push_str(token);
        previous = Some(token);
    }

    minified
}

pub(crate) fn scaled_integer_svg_path_data(data: &str, scale_factor: f64) -> Option<String> {
    let mut tokens = Vec::new();
    let mut index = 0usize;

    while index < data.len() {
        let byte = data.as_bytes()[index];
        if byte.is_ascii_whitespace() || byte == b',' {
            index += 1;
            continue;
        }

        if byte.is_ascii_alphabetic() {
            tokens.push(data[index..index + 1].to_owned());
            index += 1;
            continue;
        }

        let end = svg_number_token_end(data, index)?;
        let value = data[index..end].parse::<f64>().ok()?;
        let scaled = (value * scale_factor).round();
        let scaled = if scaled == 0.0 { 0.0 } else { scaled };
        tokens.push(format!("{scaled:.0}"));
        index = end;
    }

    Some(minify_svg_path_tokens(&tokens))
}

pub(crate) fn potrace_y_flipped_integer_svg_path_data(
    data: &str,
    canvas_height: usize,
    scale_factor: f64,
) -> Option<String> {
    let mut tokens = Vec::new();
    let mut index = 0usize;
    let mut command = None;
    let height = canvas_height as f64 * scale_factor;

    while index < data.len() {
        skip_svg_path_separators(data, &mut index);
        if index >= data.len() {
            break;
        }

        let byte = data.as_bytes()[index];
        if byte.is_ascii_alphabetic() {
            let next_command = byte as char;
            tokens.push(next_command.to_string());
            command = (!matches!(next_command, 'Z' | 'z')).then_some(next_command);
            index += 1;
            continue;
        }

        let command = command?;
        let relative = command.is_ascii_lowercase();
        match command.to_ascii_uppercase() {
            'M' | 'L' | 'T' => {
                while index < data.len() && !svg_path_next_token_is_command(data, index) {
                    push_potrace_transformed_pair(
                        data,
                        &mut index,
                        &mut tokens,
                        relative,
                        height,
                        scale_factor,
                    )?;
                    skip_svg_path_separators(data, &mut index);
                }
            }
            'C' => {
                while index < data.len() && !svg_path_next_token_is_command(data, index) {
                    for _ in 0..3 {
                        push_potrace_transformed_pair(
                            data,
                            &mut index,
                            &mut tokens,
                            relative,
                            height,
                            scale_factor,
                        )?;
                    }
                    skip_svg_path_separators(data, &mut index);
                }
            }
            'S' | 'Q' => {
                while index < data.len() && !svg_path_next_token_is_command(data, index) {
                    for _ in 0..2 {
                        push_potrace_transformed_pair(
                            data,
                            &mut index,
                            &mut tokens,
                            relative,
                            height,
                            scale_factor,
                        )?;
                    }
                    skip_svg_path_separators(data, &mut index);
                }
            }
            'H' => {
                while index < data.len() && !svg_path_next_token_is_command(data, index) {
                    let x = read_svg_path_number(data, &mut index)? * scale_factor;
                    tokens.push(format_integer_svg_token(x));
                    skip_svg_path_separators(data, &mut index);
                }
            }
            'V' => {
                while index < data.len() && !svg_path_next_token_is_command(data, index) {
                    let y = read_svg_path_number(data, &mut index)?;
                    let y = if relative {
                        -y * scale_factor
                    } else {
                        height - y * scale_factor
                    };
                    tokens.push(format_integer_svg_token(y));
                    skip_svg_path_separators(data, &mut index);
                }
            }
            _ => return None,
        }
    }

    Some(minify_svg_path_tokens(&tokens))
}

pub(crate) fn push_potrace_transformed_pair(
    data: &str,
    index: &mut usize,
    tokens: &mut Vec<String>,
    relative: bool,
    height: f64,
    scale_factor: f64,
) -> Option<()> {
    let x = read_svg_path_number(data, index)?;
    skip_svg_path_separators(data, index);
    let y = read_svg_path_number(data, index)?;
    let y = if relative {
        -y * scale_factor
    } else {
        height - y * scale_factor
    };
    tokens.push(format_integer_svg_token(x * scale_factor));
    tokens.push(format_integer_svg_token(y));
    Some(())
}

pub(crate) fn read_svg_path_number(data: &str, index: &mut usize) -> Option<f64> {
    skip_svg_path_separators(data, index);
    let end = svg_number_token_end(data, *index)?;
    let value = data[*index..end].parse::<f64>().ok()?;
    *index = end;
    Some(value)
}

pub(crate) fn skip_svg_path_separators(data: &str, index: &mut usize) {
    while *index < data.len() {
        let byte = data.as_bytes()[*index];
        if byte.is_ascii_whitespace() || byte == b',' {
            *index += 1;
        } else {
            break;
        }
    }
}

pub(crate) fn svg_path_next_token_is_command(data: &str, index: usize) -> bool {
    data.as_bytes()
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphabetic())
}

pub(crate) fn format_integer_svg_token(value: f64) -> String {
    let rounded = value.round();
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    format!("{rounded:.0}")
}

pub(crate) fn one_decimal_svg_path_data(data: &str) -> Option<String> {
    let mut tokens = Vec::new();
    let mut index = 0usize;
    let use_half_away_rounding = path_data_has_quadratic_commands(data);

    while index < data.len() {
        let byte = data.as_bytes()[index];
        if byte.is_ascii_whitespace() || byte == b',' {
            index += 1;
            continue;
        }

        if byte.is_ascii_alphabetic() {
            tokens.push(data[index..index + 1].to_owned());
            index += 1;
            continue;
        }

        let end = svg_number_token_end(data, index)?;
        let value = data[index..end].parse::<f64>().ok()?;
        let token = if use_half_away_rounding {
            format_one_decimal_half_away_from_zero(value)
        } else {
            format_compact_float_with_precision(value, 1)
        };
        tokens.push(token);
        index = end;
    }

    Some(minify_svg_path_tokens(&tokens))
}

pub(crate) fn snap_near_integer_one_decimal_svg_path_data(data: &str) -> Option<String> {
    const MAX_SNAP_DISTANCE: f64 = 0.1 + 1.0e-9;

    let mut tokens = Vec::new();
    let mut index = 0usize;

    while index < data.len() {
        let byte = data.as_bytes()[index];
        if byte.is_ascii_whitespace() || byte == b',' {
            index += 1;
            continue;
        }

        if byte.is_ascii_alphabetic() {
            tokens.push(data[index..index + 1].to_owned());
            index += 1;
            continue;
        }

        let end = svg_number_token_end(data, index)?;
        let value = data[index..end].parse::<f64>().ok()?;
        let nearest = value.round();
        if (value - nearest).abs() <= MAX_SNAP_DISTANCE {
            tokens.push(format_compact_float(nearest));
        } else {
            tokens.push(data[index..end].to_owned());
        }
        index = end;
    }

    Some(minify_svg_path_tokens(&tokens))
}

pub(crate) fn path_data_has_quadratic_commands(path_data: &str) -> bool {
    path_data.bytes().any(|byte| matches!(byte, b'Q' | b'q'))
}

pub(crate) fn format_one_decimal_half_away_from_zero(value: f64) -> String {
    let scaled = value * 10.0;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        (scaled - 0.5).ceil()
    };
    format_compact_float_with_precision(rounded / 10.0, 1)
}

pub(crate) fn svg_number_token_end(data: &str, start: usize) -> Option<usize> {
    let bytes = data.as_bytes();
    let mut index = start;

    if matches!(bytes.get(index), Some(b'+' | b'-')) {
        index += 1;
    }

    let mut has_digits = false;
    while bytes.get(index).is_some_and(|byte| byte.is_ascii_digit()) {
        has_digits = true;
        index += 1;
    }

    if matches!(bytes.get(index), Some(b'.')) {
        index += 1;
        while bytes.get(index).is_some_and(|byte| byte.is_ascii_digit()) {
            has_digits = true;
            index += 1;
        }
    }

    if !has_digits {
        return None;
    }

    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        let exponent_start = index;
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }

        let exponent_digits_start = index;
        while bytes.get(index).is_some_and(|byte| byte.is_ascii_digit()) {
            index += 1;
        }

        if index == exponent_digits_start {
            index = exponent_start;
        }
    }

    Some(index)
}

pub(crate) fn minify_svg_path_tokens(tokens: &[String]) -> String {
    let mut minified = String::new();
    let mut previous: Option<&str> = None;

    for token in tokens {
        if previous.is_some_and(|previous| compact_path_tokens_need_separator(previous, token)) {
            minified.push(' ');
        }
        minified.push_str(token);
        previous = Some(token);
    }

    minified
}

pub(crate) fn compact_path_tokens_need_separator(previous: &str, next: &str) -> bool {
    if compact_path_token_is_command(previous) || compact_path_token_is_command(next) {
        return false;
    }

    if next.starts_with('-') || next.starts_with('+') {
        return false;
    }

    !(next.starts_with('.') && previous.contains('.'))
}

pub(crate) fn compact_path_token_is_command(token: &str) -> bool {
    token.len() == 1
        && token.as_bytes().first().is_some_and(|byte| {
            matches!(
                byte,
                b'M' | b'm'
                    | b'Z'
                    | b'z'
                    | b'L'
                    | b'l'
                    | b'H'
                    | b'h'
                    | b'V'
                    | b'v'
                    | b'C'
                    | b'c'
                    | b'A'
                    | b'a'
                    | b'Q'
                    | b'q'
                    | b'S'
                    | b's'
            )
        })
}

pub(crate) fn compact_path_command_count(data: &str) -> usize {
    data.split(|character: char| {
        !(character.is_ascii_alphabetic()
            || character.is_ascii_digit()
            || matches!(character, '-' | '+' | '.'))
    })
    .flat_map(str::chars)
    .filter(|character| {
        matches!(
            character,
            'M' | 'm'
                | 'Z'
                | 'z'
                | 'L'
                | 'l'
                | 'H'
                | 'h'
                | 'V'
                | 'v'
                | 'C'
                | 'c'
                | 'A'
                | 'a'
                | 'S'
                | 's'
                | 'Q'
                | 'q'
        )
    })
    .count()
}

pub(crate) fn compact_relative_line_command(start: (f64, f64), end: (f64, f64)) -> char {
    if line_axis_delta_is_zero(start.1, end.1) {
        'h'
    } else if line_axis_delta_is_zero(start.0, end.0) {
        'v'
    } else {
        'l'
    }
}

pub(crate) fn compact_relative_line_coordinates(start: (f64, f64), end: (f64, f64)) -> String {
    if line_axis_delta_is_zero(start.1, end.1) {
        format_compact_float(end.0 - start.0)
    } else if line_axis_delta_is_zero(start.0, end.0) {
        format_compact_float(end.1 - start.1)
    } else {
        format!(
            "{} {}",
            format_compact_float(end.0 - start.0),
            format_compact_float(end.1 - start.1)
        )
    }
}

pub(crate) fn line_axis_delta_is_zero(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1.0e-9
}

pub(crate) fn compact_absolute_svg_path_data_from_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    let mut data = format!(
        "M {} {}",
        format_compact_float(start.0),
        format_compact_float(start.1)
    );
    let mut previous_command: Option<char> = None;

    for segment in segments {
        match segment {
            SvgPathSegment::Line { end, .. } => {
                if previous_command != Some('L') {
                    data.push_str(" L");
                    previous_command = Some('L');
                }

                data.push_str(&format!(
                    " {} {}",
                    format_compact_float(end.0),
                    format_compact_float(end.1)
                ));
            }
            SvgPathSegment::Cubic(cubic) => {
                if previous_command != Some('C') {
                    data.push_str(" C");
                    previous_command = Some('C');
                }

                data.push_str(&format!(
                    " {} {} {} {} {} {}",
                    format_compact_float(cubic.control1.0),
                    format_compact_float(cubic.control1.1),
                    format_compact_float(cubic.control2.0),
                    format_compact_float(cubic.control2.1),
                    format_compact_float(cubic.end.0),
                    format_compact_float(cubic.end.1)
                ));
            }
        }
    }

    data.push_str(" Z");
    data
}

pub(crate) fn compact_relative_svg_path_data_from_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    let mut data = format!(
        "M {} {}",
        format_compact_float(start.0),
        format_compact_float(start.1)
    );
    let mut previous_command: Option<char> = None;
    let mut current = start;

    for segment in segments {
        match segment {
            SvgPathSegment::Line { end, .. } => {
                if previous_command != Some('l') {
                    data.push_str(" l");
                    previous_command = Some('l');
                }

                data.push_str(&format!(
                    " {} {}",
                    format_compact_float(end.0 - current.0),
                    format_compact_float(end.1 - current.1)
                ));
                current = *end;
            }
            SvgPathSegment::Cubic(cubic) => {
                if previous_command != Some('c') {
                    data.push_str(" c");
                    previous_command = Some('c');
                }

                data.push_str(&format!(
                    " {} {} {} {} {} {}",
                    format_compact_float(cubic.control1.0 - current.0),
                    format_compact_float(cubic.control1.1 - current.1),
                    format_compact_float(cubic.control2.0 - current.0),
                    format_compact_float(cubic.control2.1 - current.1),
                    format_compact_float(cubic.end.0 - current.0),
                    format_compact_float(cubic.end.1 - current.1)
                ));
                current = cubic.end;
            }
        }
    }

    data.push_str(" Z");
    data
}

pub(crate) fn compact_smooth_relative_svg_path_data_from_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    compact_smooth_relative_svg_path_data_with_line_mode(start, segments, false)
}

pub(crate) fn compact_quadratic_relative_svg_path_data_from_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    let mut data = format!(
        "M {} {}",
        format_compact_float(start.0),
        format_compact_float(start.1)
    );
    let mut previous_command: Option<char> = None;
    let mut current = start;

    for segment in segments {
        match segment {
            SvgPathSegment::Line { end, .. } => {
                if previous_command != Some('l') {
                    data.push_str(" l");
                    previous_command = Some('l');
                }

                data.push_str(&format!(
                    " {} {}",
                    format_compact_float(end.0 - current.0),
                    format_compact_float(end.1 - current.1)
                ));
                current = *end;
            }
            SvgPathSegment::Cubic(cubic) => {
                if let Some(control) = quadratic_approximation_for_tiny_cubic(*cubic) {
                    if previous_command != Some('q') {
                        data.push_str(" q");
                        previous_command = Some('q');
                    }

                    data.push_str(&format!(
                        " {} {} {} {}",
                        format_compact_float(control.0 - current.0),
                        format_compact_float(control.1 - current.1),
                        format_compact_float(cubic.end.0 - current.0),
                        format_compact_float(cubic.end.1 - current.1)
                    ));
                } else {
                    if previous_command != Some('c') {
                        data.push_str(" c");
                        previous_command = Some('c');
                    }

                    data.push_str(&format!(
                        " {} {} {} {} {} {}",
                        format_compact_float(cubic.control1.0 - current.0),
                        format_compact_float(cubic.control1.1 - current.1),
                        format_compact_float(cubic.control2.0 - current.0),
                        format_compact_float(cubic.control2.1 - current.1),
                        format_compact_float(cubic.end.0 - current.0),
                        format_compact_float(cubic.end.1 - current.1)
                    ));
                }
                current = cubic.end;
            }
        }
    }

    data.push_str(" Z");
    data
}

pub(crate) fn compact_axis_smooth_relative_svg_path_data_from_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    compact_smooth_relative_svg_path_data_with_line_mode(start, segments, true)
}

pub(crate) fn compact_smooth_relative_svg_path_data_with_line_mode(
    start: (f64, f64),
    segments: &[SvgPathSegment],
    use_axis_lines: bool,
) -> String {
    let mut data = format!(
        "M {} {}",
        format_compact_float(start.0),
        format_compact_float(start.1)
    );
    let mut previous_command: Option<char> = None;
    let mut current = start;
    let mut previous_cubic_control2: Option<(f64, f64)> = None;

    for segment in segments {
        match segment {
            SvgPathSegment::Line { end, .. } => {
                let command = if use_axis_lines {
                    compact_relative_line_command(current, *end)
                } else {
                    'l'
                };
                if previous_command != Some(command) {
                    data.push(' ');
                    data.push(command);
                    previous_command = Some(command);
                }

                data.push(' ');
                if use_axis_lines {
                    data.push_str(&compact_relative_line_coordinates(current, *end));
                } else {
                    data.push_str(&format!(
                        "{} {}",
                        format_compact_float(end.0 - current.0),
                        format_compact_float(end.1 - current.1)
                    ));
                }
                current = *end;
                previous_cubic_control2 = None;
            }
            SvgPathSegment::Cubic(cubic) => {
                if previous_cubic_control2.is_some_and(|control2| {
                    cubic_control_reflection_is_close(current, control2, cubic.control1)
                }) {
                    if previous_command != Some('s') {
                        data.push_str(" s");
                        previous_command = Some('s');
                    }

                    data.push_str(&format!(
                        " {} {} {} {}",
                        format_compact_float(cubic.control2.0 - current.0),
                        format_compact_float(cubic.control2.1 - current.1),
                        format_compact_float(cubic.end.0 - current.0),
                        format_compact_float(cubic.end.1 - current.1)
                    ));
                } else {
                    if previous_command != Some('c') {
                        data.push_str(" c");
                        previous_command = Some('c');
                    }

                    data.push_str(&format!(
                        " {} {} {} {} {} {}",
                        format_compact_float(cubic.control1.0 - current.0),
                        format_compact_float(cubic.control1.1 - current.1),
                        format_compact_float(cubic.control2.0 - current.0),
                        format_compact_float(cubic.control2.1 - current.1),
                        format_compact_float(cubic.end.0 - current.0),
                        format_compact_float(cubic.end.1 - current.1)
                    ));
                }

                current = cubic.end;
                previous_cubic_control2 = Some(cubic.control2);
            }
        }
    }

    data.push_str(" Z");
    data
}

pub(crate) fn cubic_control_reflection_is_close(
    current: (f64, f64),
    previous_control2: (f64, f64),
    control1: (f64, f64),
) -> bool {
    const MAX_REFLECTION_DISTANCE: f64 = 0.02;

    let reflected = (
        current.0 * 2.0 - previous_control2.0,
        current.1 * 2.0 - previous_control2.1,
    );
    distance_squared_float(reflected, control1) <= MAX_REFLECTION_DISTANCE * MAX_REFLECTION_DISTANCE
}

pub(crate) fn quadratic_approximation_for_tiny_cubic(cubic: CubicSegment) -> Option<(f64, f64)> {
    const MAX_CHORD_LENGTH: f64 = 8.0;
    const MAX_APPROXIMATION_ERROR: f64 = 0.15;

    if cubic_chord_length(cubic) > MAX_CHORD_LENGTH {
        return None;
    }

    let midpoint = cubic_point(cubic, 0.5);
    let control = (
        midpoint.0 * 2.0 - (cubic.start.0 + cubic.end.0) * 0.5,
        midpoint.1 * 2.0 - (cubic.start.1 + cubic.end.1) * 0.5,
    );
    let max_error_squared = MAX_APPROXIMATION_ERROR * MAX_APPROXIMATION_ERROR;

    (1..16)
        .all(|step| {
            let parameter = step as f64 / 16.0;
            let cubic_point = cubic_point(cubic, parameter);
            let quadratic_point = quadratic_point(cubic.start, control, cubic.end, parameter);
            distance_squared_float(cubic_point, quadratic_point) <= max_error_squared
        })
        .then_some(control)
}

pub(crate) fn quadratic_point(
    start: (f64, f64),
    control: (f64, f64),
    end: (f64, f64),
    parameter: f64,
) -> (f64, f64) {
    let inverse = 1.0 - parameter;

    (
        inverse * inverse * start.0
            + 2.0 * parameter * inverse * control.0
            + parameter * parameter * end.0,
        inverse * inverse * start.1
            + 2.0 * parameter * inverse * control.1
            + parameter * parameter * end.1,
    )
}
