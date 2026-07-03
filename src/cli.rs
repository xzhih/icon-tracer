use std::ffi::OsString;
use std::path::PathBuf;

use icon_tracer::{
    AlphaBackground, ContourMode, CurveMode, RasterOptions, SvgRenderOptions, ThresholdMode,
    TraceImageOptions, TraceOptions, TracePreset,
};

pub const USAGE: &str = "usage: icon-tracer [--preset default|logo|scan|icon] [--threshold auto|0-255] [--invert|--no-invert] [--alpha-background black|white] [--contour pixel|subpixel|scalar] [--curve polygon|smooth|spline|fit|potrace] [--smooth|--spline|--fit] [--turd-size N] [--opt-tolerance N] [--optimize-icon] [--isolate-foreground|--no-isolate-foreground] [--optimization-report path.json] <input> <output.svg>";

#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    Help,
    Trace(CliOptions),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CliOptions {
    pub threshold: ThresholdMode,
    pub invert: bool,
    pub alpha_background: AlphaBackground,
    pub curve_mode: CurveMode,
    pub contour_mode: ContourMode,
    pub turd_size: usize,
    pub opt_tolerance: f64,
    pub optimize_icon: bool,
    pub isolate_foreground: bool,
    pub optimization_report: Option<PathBuf>,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
}

impl CliOptions {
    pub fn raster_options(&self) -> RasterOptions {
        RasterOptions {
            threshold: self.threshold,
            invert: self.invert,
            alpha_background: self.alpha_background,
        }
    }

    pub fn trace_options(&self) -> TraceOptions {
        TraceOptions {
            turd_size: self.turd_size,
            opt_tolerance: self.trace_opt_tolerance(),
            contour_mode: self.contour_mode,
            preserve_collinear: false,
        }
    }

    pub fn svg_render_options(&self) -> SvgRenderOptions {
        SvgRenderOptions {
            curve_mode: self.curve_mode,
            opt_tolerance: self.svg_opt_tolerance(),
            pixel_potrace: self.pixel_potrace_mode(),
        }
    }

    pub fn pixel_potrace_mode(&self) -> bool {
        self.curve_mode == CurveMode::Potrace
            && self.contour_mode == ContourMode::Pixel
            && !self.optimize_icon
    }

    fn trace_opt_tolerance(&self) -> f64 {
        if self.pixel_potrace_mode() {
            0.0
        } else {
            self.opt_tolerance
        }
    }

    fn svg_opt_tolerance(&self) -> f64 {
        if self.pixel_potrace_mode() {
            self.opt_tolerance
        } else {
            SvgRenderOptions::default().opt_tolerance
        }
    }
}

pub fn parse_args<I>(args: I) -> Result<CliCommand, String>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(CliCommand::Help);
    }

    let mut preset = TracePreset::Icon;
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
        return Err(USAGE.to_owned());
    }

    if optimization_report.is_some() && !optimize_icon {
        return Err("--optimization-report requires --optimize-icon".to_owned());
    }
    if isolate_foreground.is_some() && !optimize_icon {
        return Err("--isolate-foreground requires --optimize-icon".to_owned());
    }

    let defaults = TraceImageOptions::preset(preset);
    Ok(CliCommand::Trace(CliOptions {
        threshold: threshold.unwrap_or(defaults.raster_options.threshold),
        invert: invert.unwrap_or(defaults.raster_options.invert),
        alpha_background: alpha_background.unwrap_or_default(),
        curve_mode: curve_mode.unwrap_or(defaults.svg_render_options.curve_mode),
        contour_mode: contour_mode.unwrap_or(defaults.trace_options.contour_mode),
        turd_size: turd_size.unwrap_or(defaults.trace_options.turd_size),
        opt_tolerance: opt_tolerance.unwrap_or(defaults.trace_options.opt_tolerance),
        optimize_icon,
        isolate_foreground: isolate_foreground.unwrap_or(false),
        optimization_report,
        input_path: paths.remove(0),
        output_path: paths.remove(0),
    }))
}

fn parse_preset(value: &str) -> Result<TracePreset, String> {
    match value {
        "default" => Ok(TracePreset::Default),
        "logo" => Ok(TracePreset::Logo),
        "scan" => Ok(TracePreset::Scan),
        "icon" => Ok(TracePreset::Icon),
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
