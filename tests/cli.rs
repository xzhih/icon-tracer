use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn cli_converts_pbm_file_to_svg_file() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n2 2\n1 0\n0 0\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2 2""#));
    assert!(svg.contains(r#"d="M 0 0 L 1 0 L 1 1 L 0 1 Z""#));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_uses_icon_preset_by_default() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let default_output = work_dir.join("default.svg");
    let icon_output = work_dir.join("icon.svg");
    fs::write(&input, b"P1\n3 3\n1 1 1\n1 1 1\n1 1 1\n").expect("input should be written");

    let default_status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg(&input)
        .arg(&default_output)
        .status()
        .expect("icon-tracer should run");
    let icon_status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("icon")
        .arg(&input)
        .arg(&icon_output)
        .status()
        .expect("icon-tracer should run");

    assert!(default_status.success());
    assert!(icon_status.success());

    let default_svg = fs::read_to_string(&default_output).expect("SVG should be written");
    let icon_svg = fs::read_to_string(&icon_output).expect("SVG should be written");
    assert_eq!(default_svg, icon_svg);

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_smooth_flag_outputs_cubic_segments() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n3 3\n1 1 1\n1 1 1\n1 1 1\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--smooth")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(" C "));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_spline_flag_outputs_continuous_cubic_segments() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n3 3\n1 1 1\n1 1 1\n1 1 1\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--spline")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"d="M 0 0 C"#));
    assert!(!svg.contains(" L "));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_fit_flag_outputs_bounded_cubic_segments() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(
        &input,
        b"P1\n5 5\n1 0 0 0 0\n1 1 0 0 0\n1 1 1 0 0\n1 1 1 1 0\n1 1 1 1 1\n",
    )
    .expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--fit")
        .arg("--opt-tolerance")
        .arg("0.75")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"d="M 0 0 C"#));
    assert!(!svg.contains(" -"));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_potrace_curve_outputs_midpoint_cubic_segments() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(
        &input,
        b"P1\n9 9\n0 0 1 1 1 1 1 0 0\n0 1 1 1 1 1 1 1 0\n1 1 1 1 1 1 1 1 1\n1 1 1 1 1 1 1 1 1\n1 1 1 1 1 1 1 1 1\n1 1 1 1 1 1 1 1 1\n1 1 1 1 1 1 1 1 1\n0 1 1 1 1 1 1 1 0\n0 0 1 1 1 1 1 0 0\n",
    )
    .expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--curve")
        .arg("potrace")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    let path_data = svg
        .split_once(r#"d=""#)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(path_data, _)| path_data)
        .expect("SVG should contain path data");
    assert!(path_data.contains('C') || path_data.contains('c'));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_turd_size_filters_small_components() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n5 3\n1 0 0 0 0\n0 0 1 1 0\n0 0 1 1 0\n")
        .expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--turd-size")
        .arg("2")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"d="M 2 1 L 4 1 L 4 3 L 2 3 Z""#));
    assert!(!svg.contains("M 0 0 L 1 0 L 1 1 L 0 1 Z"));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_opt_tolerance_simplifies_stair_step_paths() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(
        &input,
        b"P1\n5 5\n1 0 0 0 0\n1 1 0 0 0\n1 1 1 0 0\n1 1 1 1 0\n1 1 1 1 1\n",
    )
    .expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--opt-tolerance")
        .arg("0.75")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"d="M 0 0 L 5 5 L 0 5 Z""#));
    assert!(!svg.contains("L 1 1 L 2 1"));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_converts_bmp_file_to_svg_file() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.bmp");
    let output = work_dir.join("output.svg");
    fs::write(&input, bmp_24_bit_2x2()).expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2 2""#));
    assert!(svg.contains(r#"d="M 0 0 L 1 0 L 1 1 L 0 1 Z""#));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_converts_png_file_to_svg_file() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.png");
    let output = work_dir.join("output.svg");
    fs::write(&input, png_2x1_rgb()).expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2 1""#));
    assert!(svg.contains(r#"d="M 0 0 L 1 0 L 1 1 L 0 1 Z""#));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_alpha_background_black_composites_transparent_png_over_black() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.png");
    let output = work_dir.join("output.svg");
    fs::write(&input, png_2x1_rgba()).expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--threshold")
        .arg("128")
        .arg("--alpha-background")
        .arg("black")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"d="M 0 0 L 2 0 L 2 1 L 0 1 Z""#));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_converts_jpeg_file_to_svg_file() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.jpg");
    let output = work_dir.join("output.svg");
    fs::write(&input, jpeg_1x1_black()).expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1""#));
    assert!(svg.contains(r#"d="M 0 0 L 1 0 L 1 1 L 0 1 Z""#));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_threshold_auto_option_uses_automatic_luma_split() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pgm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P2\n4 1\n255\n10 20 220 230\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--threshold=auto")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"d="M 0 0 L 2 0 L 2 1 L 0 1 Z""#));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_subpixel_contour_outputs_half_pixel_polygon() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n1 1\n1\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--contour")
        .arg("subpixel")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"d="M 0.5 0 L 1 0.5 L 0.5 1 L 0 0.5 Z""#));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_scalar_contour_outputs_interpolated_polygon() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.png");
    let output = work_dir.join("output.svg");
    fs::write(&input, png_2x1_gray(&[0, 192])).expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("default")
        .arg("--threshold")
        .arg("128")
        .arg("--contour")
        .arg("scalar")
        .arg("--curve")
        .arg("polygon")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains("1.166667"), "{svg}");

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_optimize_icon_writes_svg_and_feedback_report() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.png");
    let output = work_dir.join("output.svg");
    let report = work_dir.join("report.json");
    fs::write(&input, png_4x4_center_square()).expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--threshold")
        .arg("128")
        .arg("--optimize-icon")
        .arg("--optimization-report")
        .arg(&report)
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains("<svg"));

    let report_json = fs::read_to_string(&report).expect("report should be written");
    assert!(report_json.contains(r#""best_candidate""#));
    assert!(report_json.contains(r#""candidates""#));
    assert!(report_json.contains(r#""turd_size""#));
    assert!(report_json.contains(r#""xor_ratio""#));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_logo_preset_uses_subpixel_contour() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n3 3\n1 1 1\n1 1 1\n1 1 1\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("logo")
        .arg("--curve")
        .arg("polygon")
        .arg("--turd-size")
        .arg("0")
        .arg("--opt-tolerance")
        .arg("0")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(
        svg.contains(r#"d="M 0.5 0 L 0 0.5 L 0 2.5 L 0.5 3 L 2.5 3 L 3 2.5 L 3 0.5 L 2.5 0 Z""#)
    );

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_logo_preset_uses_potrace_curve() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(
        &input,
        b"P1\n9 9\n0 0 1 1 1 1 1 0 0\n0 1 1 1 1 1 1 1 0\n1 1 1 1 1 1 1 1 1\n1 1 1 1 1 1 1 1 1\n1 1 1 1 1 1 1 1 1\n1 1 1 1 1 1 1 1 1\n1 1 1 1 1 1 1 1 1\n0 1 1 1 1 1 1 1 0\n0 0 1 1 1 1 1 0 0\n",
    )
    .expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("logo")
        .arg("--turd-size")
        .arg("0")
        .arg("--opt-tolerance")
        .arg("0")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(" C "));
    assert!(!svg.contains(" L "));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_preset_logo_can_be_overridden_to_polygon_after_preset() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n3 3\n1 1 1\n1 1 1\n1 1 1\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset")
        .arg("logo")
        .arg("--contour")
        .arg("pixel")
        .arg("--curve")
        .arg("polygon")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"d="M 0 0 L 3 0 L 3 3 L 0 3 Z""#));
    assert!(!svg.contains(" C "));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_explicit_curve_override_is_order_independent_from_preset() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n3 3\n1 1 1\n1 1 1\n1 1 1\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--curve=polygon")
        .arg("--contour=pixel")
        .arg("--preset=logo")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(status.success());

    let svg = fs::read_to_string(&output).expect("SVG should be written");
    assert!(svg.contains(r#"d="M 0 0 L 3 0 L 3 3 L 0 3 Z""#));
    assert!(!svg.contains(" C "));

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_rejects_unknown_preset() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n1 1\n1\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--preset=photo")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(!status.success());

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

#[test]
fn cli_rejects_isolate_foreground_without_icon_optimizer() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("temp dir should be created");

    let input = work_dir.join("input.pbm");
    let output = work_dir.join("output.svg");
    fs::write(&input, b"P1\n1 1\n1\n").expect("input should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_icon-tracer"))
        .arg("--isolate-foreground")
        .arg(&input)
        .arg(&output)
        .status()
        .expect("icon-tracer should run");

    assert!(!status.success());

    fs::remove_dir_all(work_dir).expect("temp dir should be removed");
}

fn unique_temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);

    std::env::temp_dir().join(format!(
        "icon-tracer-test-{}-{nanos}-{counter}",
        std::process::id()
    ))
}

fn bmp_24_bit_2x2() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&70u32.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&54u32.to_le_bytes());

    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&2i32.to_le_bytes());
    bytes.extend_from_slice(&2i32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&24u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());

    bytes.extend_from_slice(&[255, 255, 255, 255, 255, 255, 0, 0]);
    bytes.extend_from_slice(&[0, 0, 0, 255, 255, 255, 0, 0]);
    bytes
}

fn png_2x1_rgb() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x7b,
        0x40, 0xe8, 0xdd, 0x00, 0x00, 0x00, 0x0f, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60,
        0x60, 0x60, 0xf8, 0xff, 0xff, 0x3f, 0x00, 0x06, 0x01, 0x02, 0xfe, 0x02, 0xb2, 0x39, 0xae,
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

fn png_2x1_rgba() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0xf4,
        0x22, 0x7f, 0x8a, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60,
        0x80, 0x80, 0xff, 0x00, 0x01, 0x08, 0x01, 0x00, 0x4d, 0x19, 0x8f, 0x39, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

fn png_2x1_gray(samples: &[u8; 2]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header should write");
        writer
            .write_image_data(samples)
            .expect("PNG data should write");
    }
    bytes
}

fn png_4x4_center_square() -> Vec<u8> {
    let mut samples = Vec::with_capacity(16);

    for y in 0..4 {
        for x in 0..4 {
            samples.push(if (1..=2).contains(&x) && (1..=2).contains(&y) {
                0
            } else {
                255
            });
        }
    }

    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 4, 4);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header should write");
        writer
            .write_image_data(&samples)
            .expect("PNG data should write");
    }
    bytes
}

fn jpeg_1x1_black() -> &'static [u8] {
    &[
        0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00,
        0x01, 0x00, 0x01, 0x00, 0x00, 0xff, 0xdb, 0x00, 0x43, 0x00, 0x03, 0x02, 0x02, 0x02, 0x02,
        0x02, 0x03, 0x02, 0x02, 0x02, 0x03, 0x03, 0x03, 0x03, 0x04, 0x06, 0x04, 0x04, 0x04, 0x04,
        0x04, 0x08, 0x06, 0x06, 0x05, 0x06, 0x09, 0x08, 0x0a, 0x0a, 0x09, 0x08, 0x09, 0x09, 0x0a,
        0x0c, 0x0f, 0x0c, 0x0a, 0x0b, 0x0e, 0x0b, 0x09, 0x09, 0x0d, 0x11, 0x0d, 0x0e, 0x0f, 0x10,
        0x10, 0x11, 0x10, 0x0a, 0x0c, 0x12, 0x13, 0x12, 0x10, 0x13, 0x0f, 0x10, 0x10, 0x10, 0xff,
        0xc0, 0x00, 0x0b, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xff, 0xc4, 0x00,
        0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x09, 0xff, 0xc4, 0x00, 0x14, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xda, 0x00, 0x08,
        0x01, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x2a, 0x9f, 0xff, 0xd9,
    ]
}
