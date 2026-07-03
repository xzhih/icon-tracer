mod cli;
mod optimization_report;

use std::env;
use std::fs;
use std::process::ExitCode;

use cli::{CliCommand, CliOptions};
use icon_tracer::{
    optimize_icon_trace, trace_bitmap, trace_scalar_field, Bitmap, ContourMode, RgbaImage,
    SvgOptions,
};
use optimization_report::optimization_report_json;

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
    match cli::parse_args(env::args_os().skip(1))? {
        CliCommand::Help => {
            println!("{}", cli::USAGE);
            Ok(())
        }
        CliCommand::Trace(options) => trace_file(options),
    }
}

fn trace_file(options: CliOptions) -> Result<(), String> {
    let input = fs::read(&options.input_path)
        .map_err(|error| format!("failed to read {}: {error}", options.input_path.display()))?;
    let raster_options = options.raster_options();
    let trace_options = options.trace_options();
    let svg_render_options = options.svg_render_options();

    let svg = if options.optimize_icon {
        let image = RgbaImage::from_bytes(&input).map_err(|error| {
            format!("failed to parse {}: {error}", options.input_path.display())
        })?;
        let result = optimize_icon_trace(
            &image,
            icon_tracer::IconOptimizeOptions {
                raster_options,
                trace_options,
                svg_options: SvgOptions {
                    curve_mode: options.curve_mode,
                },
                isolate_foreground: options.isolate_foreground,
                ..icon_tracer::IconOptimizeOptions::default()
            },
        )
        .map_err(|error| {
            format!(
                "failed to optimize {}: {error}",
                options.input_path.display()
            )
        })?;

        if let Some(report_path) = &options.optimization_report {
            fs::write(report_path, optimization_report_json(&result)).map_err(|error| {
                format!(
                    "failed to write optimization report {}: {error}",
                    report_path.display()
                )
            })?;
        }

        result.to_svg()
    } else if options.contour_mode == ContourMode::Scalar {
        let image = RgbaImage::from_bytes(&input).map_err(|error| {
            format!("failed to parse {}: {error}", options.input_path.display())
        })?;
        let field = image.to_scalar_field(options.alpha_background);
        trace_scalar_field(&field, raster_options, trace_options)
            .map_err(|error| format!("failed to trace {}: {error}", options.input_path.display()))?
            .to_svg_with_render_options(svg_render_options)
    } else {
        let bitmap = Bitmap::from_bytes(&input, raster_options).map_err(|error| {
            format!("failed to parse {}: {error}", options.input_path.display())
        })?;
        let mut trace_options = trace_options;
        if options.pixel_potrace_mode() && bitmap.width().saturating_mul(bitmap.height()) >= 64 * 64
        {
            trace_options.preserve_collinear = true;
        }
        trace_bitmap(&bitmap, trace_options).to_svg_with_render_options(svg_render_options)
    };

    fs::write(&options.output_path, svg)
        .map_err(|error| format!("failed to write {}: {error}", options.output_path.display()))?;

    Ok(())
}
