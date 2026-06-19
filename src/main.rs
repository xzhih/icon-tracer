use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use icon_tracer::{
    optimize_icon_trace, trace_bitmap, trace_scalar_field, AlphaBackground, Bitmap, ContourMode,
    CurveMode, IconOptimizationResult, RasterOptions, RgbaImage, SvgOptions, SvgRenderOptions,
    ThresholdMode, TraceOptions,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("icon-tracer: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(());
    }

    let mut preset = Preset::Icon;
    let mut threshold = None;
    let mut invert = None;
    let mut alpha_background = None;
    let mut curve_mode = None;
    let mut contour_mode = None;
    let mut turd_size = None;
    let mut opt_tolerance = None;
    let mut optimize_icon = false;
    let mut isolate_foreground = None;
    let mut optimization_report = None;
    let mut paths = Vec::new();
    let mut index = 0;

    while index < args.len() {
        let arg = args[index].to_string_lossy();

        match arg.as_ref() {
            "--invert" => invert = Some(true),
            "--no-invert" => invert = Some(false),
            "--smooth" => curve_mode = Some(CurveMode::Smooth),
            "--spline" => curve_mode = Some(CurveMode::Spline),
            "--fit" => curve_mode = Some(CurveMode::Fit),
            "--optimize-icon" => optimize_icon = true,
            "--isolate-foreground" => isolate_foreground = Some(true),
            "--no-isolate-foreground" => isolate_foreground = Some(false),
            "--optimization-report" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--optimization-report requires a path".to_owned())?;
                optimization_report = Some(PathBuf::from(value));
            }
            "--alpha-background" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--alpha-background requires black or white".to_owned())?;
                alpha_background = Some(parse_alpha_background(&value.to_string_lossy())?);
            }
            "--contour" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--contour requires pixel, subpixel, or scalar".to_owned())?;
                contour_mode = Some(parse_contour_mode(&value.to_string_lossy())?);
            }
            "--curve" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    "--curve requires polygon, smooth, spline, fit, or potrace".to_owned()
                })?;
                curve_mode = Some(parse_curve_mode(&value.to_string_lossy())?);
            }
            "--preset" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--preset requires default, logo, scan, or icon".to_owned())?;
                preset = parse_preset(&value.to_string_lossy())?;
            }
            "--turd-size" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--turd-size requires a non-negative integer".to_owned())?;
                turd_size = Some(parse_turd_size(&value.to_string_lossy())?);
            }
            "--opt-tolerance" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--opt-tolerance requires a non-negative number".to_owned())?;
                opt_tolerance = Some(parse_opt_tolerance(&value.to_string_lossy())?);
            }
            "--threshold" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    "--threshold requires auto or a value from 0 to 255".to_owned()
                })?;
                threshold = Some(parse_threshold(&value.to_string_lossy())?);
            }
            _ if arg.starts_with("--curve=") => {
                curve_mode = Some(parse_curve_mode(&arg["--curve=".len()..])?);
            }
            _ if arg.starts_with("--contour=") => {
                contour_mode = Some(parse_contour_mode(&arg["--contour=".len()..])?);
            }
            _ if arg.starts_with("--preset=") => {
                preset = parse_preset(&arg["--preset=".len()..])?;
            }
            _ if arg.starts_with("--threshold=") => {
                threshold = Some(parse_threshold(&arg["--threshold=".len()..])?);
            }
            _ if arg.starts_with("--alpha-background=") => {
                alpha_background =
                    Some(parse_alpha_background(&arg["--alpha-background=".len()..])?);
            }
            _ if arg.starts_with("--turd-size=") => {
                turd_size = Some(parse_turd_size(&arg["--turd-size=".len()..])?);
            }
            _ if arg.starts_with("--opt-tolerance=") => {
                opt_tolerance = Some(parse_opt_tolerance(&arg["--opt-tolerance=".len()..])?);
            }
            _ if arg.starts_with("--optimization-report=") => {
                optimization_report = Some(PathBuf::from(&arg["--optimization-report=".len()..]));
            }
            _ if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
            _ => paths.push(PathBuf::from(&args[index])),
        }

        index += 1;
    }

    if paths.len() != 2 {
        return Err(
            "usage: icon-tracer [--preset default|logo|scan|icon] [--threshold auto|0-255] [--invert|--no-invert] [--alpha-background black|white] [--contour pixel|subpixel|scalar] [--curve polygon|smooth|spline|fit|potrace] [--smooth|--spline|--fit] [--turd-size N] [--opt-tolerance N] [--optimize-icon] [--isolate-foreground|--no-isolate-foreground] [--optimization-report path.json] <input> <output.svg>"
                .into(),
        );
    }

    if optimization_report.is_some() && !optimize_icon {
        return Err("--optimization-report requires --optimize-icon".to_owned());
    }
    if isolate_foreground.is_some() && !optimize_icon {
        return Err("--isolate-foreground requires --optimize-icon".to_owned());
    }

    let defaults = preset.defaults();
    let threshold = threshold.unwrap_or(defaults.threshold);
    let invert = invert.unwrap_or(defaults.invert);
    let alpha_background = alpha_background.unwrap_or_default();
    let curve_mode = curve_mode.unwrap_or(defaults.curve_mode);
    let contour_mode = contour_mode.unwrap_or(defaults.contour_mode);
    let turd_size = turd_size.unwrap_or(defaults.turd_size);
    let opt_tolerance = opt_tolerance.unwrap_or(defaults.opt_tolerance);
    let pixel_potrace_mode =
        curve_mode == CurveMode::Potrace && contour_mode == ContourMode::Pixel && !optimize_icon;
    let trace_opt_tolerance = if pixel_potrace_mode {
        0.0
    } else {
        opt_tolerance
    };
    let svg_opt_tolerance = if pixel_potrace_mode {
        opt_tolerance
    } else {
        SvgRenderOptions::default().opt_tolerance
    };
    let svg_render_options = SvgRenderOptions {
        curve_mode,
        opt_tolerance: svg_opt_tolerance,
        pixel_potrace: pixel_potrace_mode,
    };

    let input = fs::read(&paths[0])
        .map_err(|error| format!("failed to read {}: {error}", paths[0].display()))?;
    let raster_options = RasterOptions {
        threshold,
        invert,
        alpha_background,
    };
    let trace_options = TraceOptions {
        turd_size,
        opt_tolerance: trace_opt_tolerance,
        contour_mode,
    };
    let svg = if optimize_icon {
        let image = RgbaImage::from_bytes(&input)
            .map_err(|error| format!("failed to parse {}: {error}", paths[0].display()))?;
        let result = optimize_icon_trace(
            &image,
            icon_tracer::IconOptimizeOptions {
                raster_options,
                trace_options,
                svg_options: SvgOptions { curve_mode },
                isolate_foreground: isolate_foreground.unwrap_or(false),
                ..icon_tracer::IconOptimizeOptions::default()
            },
        )
        .map_err(|error| format!("failed to optimize {}: {error}", paths[0].display()))?;

        if let Some(report_path) = optimization_report {
            fs::write(&report_path, optimization_report_json(&result)).map_err(|error| {
                format!(
                    "failed to write optimization report {}: {error}",
                    report_path.display()
                )
            })?;
        }

        result.to_svg()
    } else if contour_mode == ContourMode::Scalar {
        let image = RgbaImage::from_bytes(&input)
            .map_err(|error| format!("failed to parse {}: {error}", paths[0].display()))?;
        let field = image.to_scalar_field(alpha_background);
        trace_scalar_field(&field, raster_options, trace_options)
            .map_err(|error| format!("failed to trace {}: {error}", paths[0].display()))?
            .to_svg_with_render_options(svg_render_options)
    } else {
        let bitmap = Bitmap::from_bytes(&input, raster_options)
            .map_err(|error| format!("failed to parse {}: {error}", paths[0].display()))?;
        trace_bitmap(&bitmap, trace_options).to_svg_with_render_options(svg_render_options)
    };

    fs::write(&paths[1], svg)
        .map_err(|error| format!("failed to write {}: {error}", paths[1].display()))?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preset {
    Default,
    Logo,
    Scan,
    Icon,
}

#[derive(Debug, Clone, Copy)]
struct PresetDefaults {
    threshold: ThresholdMode,
    invert: bool,
    curve_mode: CurveMode,
    contour_mode: ContourMode,
    turd_size: usize,
    opt_tolerance: f64,
}

impl Preset {
    fn defaults(self) -> PresetDefaults {
        match self {
            Self::Default => PresetDefaults {
                threshold: ThresholdMode::Auto,
                invert: false,
                curve_mode: CurveMode::Polygon,
                contour_mode: ContourMode::Pixel,
                turd_size: 0,
                opt_tolerance: 0.0,
            },
            Self::Logo => PresetDefaults {
                threshold: ThresholdMode::Auto,
                invert: false,
                curve_mode: CurveMode::Potrace,
                contour_mode: ContourMode::Subpixel,
                turd_size: 4,
                opt_tolerance: 0.75,
            },
            Self::Scan => PresetDefaults {
                threshold: ThresholdMode::Auto,
                invert: false,
                curve_mode: CurveMode::Polygon,
                contour_mode: ContourMode::Pixel,
                turd_size: 2,
                opt_tolerance: 0.0,
            },
            Self::Icon => PresetDefaults {
                threshold: ThresholdMode::Auto,
                invert: false,
                curve_mode: CurveMode::Potrace,
                contour_mode: ContourMode::Subpixel,
                turd_size: 2,
                opt_tolerance: 0.75,
            },
        }
    }
}

fn parse_preset(value: &str) -> Result<Preset, String> {
    match value {
        "default" => Ok(Preset::Default),
        "logo" => Ok(Preset::Logo),
        "scan" => Ok(Preset::Scan),
        "icon" => Ok(Preset::Icon),
        _ => Err(format!("invalid preset: {value}")),
    }
}

fn parse_curve_mode(value: &str) -> Result<CurveMode, String> {
    match value {
        "polygon" => Ok(CurveMode::Polygon),
        "smooth" => Ok(CurveMode::Smooth),
        "spline" => Ok(CurveMode::Spline),
        "fit" => Ok(CurveMode::Fit),
        "potrace" => Ok(CurveMode::Potrace),
        _ => Err(format!("invalid curve mode: {value}")),
    }
}

fn parse_contour_mode(value: &str) -> Result<ContourMode, String> {
    match value {
        "pixel" => Ok(ContourMode::Pixel),
        "subpixel" => Ok(ContourMode::Subpixel),
        "scalar" => Ok(ContourMode::Scalar),
        _ => Err(format!("invalid contour mode: {value}")),
    }
}

fn parse_alpha_background(value: &str) -> Result<AlphaBackground, String> {
    match value {
        "black" => Ok(AlphaBackground::Black),
        "white" => Ok(AlphaBackground::White),
        _ => Err(format!("invalid alpha background: {value}")),
    }
}

fn parse_threshold(value: &str) -> Result<ThresholdMode, String> {
    if value == "auto" {
        return Ok(ThresholdMode::Auto);
    }

    value
        .parse()
        .map(ThresholdMode::Fixed)
        .map_err(|_| format!("invalid threshold: {value}"))
}

fn parse_turd_size(value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("invalid turd size: {value}"))
}

fn parse_opt_tolerance(value: &str) -> Result<f64, String> {
    let tolerance = value
        .parse()
        .map_err(|_| format!("invalid opt tolerance: {value}"))?;

    if tolerance < 0.0 {
        return Err(format!("invalid opt tolerance: {value}"));
    }

    Ok(tolerance)
}

fn optimization_report_json(result: &IconOptimizationResult) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"best_candidate\": ");
    push_candidate_json(&mut json, &result.best_candidate, 2);
    json.push_str(",\n  \"candidates\": [\n");

    for (index, candidate) in result.candidates.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }

        json.push_str("    ");
        push_candidate_json(&mut json, candidate, 4);
    }

    json.push_str("\n  ]\n}\n");
    json
}

fn push_candidate_json(
    json: &mut String,
    candidate: &icon_tracer::IconOptimizationCandidate,
    indent: usize,
) {
    let nested = " ".repeat(indent + 2);
    json.push_str("{\n");
    json.push_str(&format!(
        "{nested}\"contour_mode\": \"{}\",\n",
        contour_mode_name(candidate.trace_options.contour_mode)
    ));
    json.push_str(&format!(
        "{nested}\"opt_tolerance\": {:.6},\n",
        candidate.trace_options.opt_tolerance
    ));
    json.push_str(&format!("{nested}\"score\": {:.9},\n", candidate.score));
    json.push_str(&format!(
        "{nested}\"path_count\": {},\n",
        candidate.path_count
    ));
    json.push_str(&format!(
        "{nested}\"point_count\": {},\n",
        candidate.point_count
    ));
    json.push_str(&format!(
        "{nested}\"svg_command_count\": {},\n",
        candidate.svg_command_count
    ));
    json.push_str(&format!("{nested}\"metrics\": {{\n"));

    let metric_indent = " ".repeat(indent + 4);
    let metrics = candidate.metrics;
    json.push_str(&format!(
        "{metric_indent}\"total_pixels\": {},\n",
        metrics.total_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"target_foreground_pixels\": {},\n",
        metrics.target_foreground_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"candidate_foreground_pixels\": {},\n",
        metrics.candidate_foreground_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"true_positive_pixels\": {},\n",
        metrics.true_positive_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"false_positive_pixels\": {},\n",
        metrics.false_positive_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"false_negative_pixels\": {},\n",
        metrics.false_negative_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"xor_pixels\": {},\n",
        metrics.xor_pixels
    ));
    json.push_str(&format!(
        "{metric_indent}\"xor_ratio\": {:.9},\n",
        metrics.xor_ratio
    ));
    json.push_str(&format!(
        "{metric_indent}\"foreground_error_ratio\": {:.9},\n",
        metrics.foreground_error_ratio
    ));
    json.push_str(&format!(
        "{metric_indent}\"false_positive_ratio\": {:.9},\n",
        metrics.false_positive_ratio
    ));
    json.push_str(&format!(
        "{metric_indent}\"false_negative_ratio\": {:.9},\n",
        metrics.false_negative_ratio
    ));
    json.push_str(&format!(
        "{metric_indent}\"precision\": {:.9},\n",
        metrics.precision
    ));
    json.push_str(&format!(
        "{metric_indent}\"recall\": {:.9},\n",
        metrics.recall
    ));
    json.push_str(&format!("{metric_indent}\"iou\": {:.9}\n", metrics.iou));
    json.push_str(&format!("{nested}}}\n"));
    json.push_str(&format!("{}}}", " ".repeat(indent)));
}

fn contour_mode_name(mode: ContourMode) -> &'static str {
    match mode {
        ContourMode::Pixel => "pixel",
        ContourMode::Subpixel => "subpixel",
        ContourMode::Scalar => "scalar",
    }
}

fn print_usage() {
    println!(
        "usage: icon-tracer [--preset default|logo|scan|icon] [--threshold auto|0-255] [--invert|--no-invert] [--alpha-background black|white] [--contour pixel|subpixel|scalar] [--curve polygon|smooth|spline|fit|potrace] [--smooth|--spline|--fit] [--turd-size N] [--opt-tolerance N] [--optimize-icon] [--isolate-foreground|--no-isolate-foreground] [--optimization-report path.json] <input> <output.svg>"
    );
}
