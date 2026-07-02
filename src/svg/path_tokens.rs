use super::*;

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

pub(crate) fn path_data_has_bezier_commands(path_data: &str) -> bool {
    path_data
        .bytes()
        .any(|byte| matches!(byte, b'C' | b'c' | b'S' | b's' | b'Q' | b'q' | b'T' | b't'))
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
