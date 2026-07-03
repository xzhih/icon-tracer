use icon_tracer::{
    trace_image_to_svg, AlphaBackground, ContourMode, CurveMode, SvgRenderOptions, ThresholdMode,
    TraceImageOptions, TracePreset,
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsTraceOptions {
    preset: Option<String>,
    threshold: Option<String>,
    invert: Option<bool>,
    alpha_background: Option<String>,
    contour_mode: Option<String>,
    curve_mode: Option<String>,
    turd_size: Option<usize>,
    opt_tolerance: Option<f64>,
    optimize_icon: Option<bool>,
    isolate_foreground: Option<bool>,
    pixel_potrace: Option<bool>,
}

#[wasm_bindgen]
pub fn trace_image_to_svg_wasm(bytes: &[u8], options: JsValue) -> Result<String, JsValue> {
    let options = parse_trace_options(options)?;
    trace_image_to_svg(bytes, options).map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub fn icon_tracer_wasm_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

fn parse_trace_options(value: JsValue) -> Result<TraceImageOptions, JsValue> {
    let js_options = if value.is_undefined() || value.is_null() {
        JsTraceOptions::default()
    } else {
        serde_wasm_bindgen::from_value(value)
            .map_err(|error| JsValue::from_str(&format!("invalid options: {error}")))?
    };

    let preset = match js_options.preset.as_deref().unwrap_or("icon") {
        "default" => TracePreset::Default,
        "logo" => TracePreset::Logo,
        "scan" => TracePreset::Scan,
        "icon" => TracePreset::Icon,
        value => return Err(JsValue::from_str(&format!("invalid preset: {value}"))),
    };
    let mut options = TraceImageOptions::preset(preset);

    if let Some(threshold) = js_options.threshold {
        options.raster_options.threshold = parse_threshold(&threshold)?;
    }
    if let Some(invert) = js_options.invert {
        options.raster_options.invert = invert;
    }
    if let Some(alpha_background) = js_options.alpha_background {
        options.raster_options.alpha_background = parse_alpha_background(&alpha_background)?;
    }
    if let Some(contour_mode) = js_options.contour_mode {
        options.trace_options.contour_mode = parse_contour_mode(&contour_mode)?;
    }
    if let Some(curve_mode) = js_options.curve_mode {
        options.svg_render_options.curve_mode = parse_curve_mode(&curve_mode)?;
    }
    if let Some(turd_size) = js_options.turd_size {
        options.trace_options.turd_size = turd_size;
    }
    if let Some(opt_tolerance) = js_options.opt_tolerance {
        if opt_tolerance < 0.0 {
            return Err(JsValue::from_str("optTolerance must be non-negative"));
        }
        options.trace_options.opt_tolerance = opt_tolerance;
        options.svg_render_options.opt_tolerance = opt_tolerance;
    }
    if let Some(optimize_icon) = js_options.optimize_icon {
        options.optimize_icon = optimize_icon;
    }
    if let Some(isolate_foreground) = js_options.isolate_foreground {
        options.isolate_foreground = isolate_foreground;
    }
    if let Some(pixel_potrace) = js_options.pixel_potrace {
        options.svg_render_options.pixel_potrace = pixel_potrace;
    } else {
        options.svg_render_options.pixel_potrace = pixel_potrace_default(
            options.trace_options.contour_mode,
            options.svg_render_options,
        );
    }

    if options.svg_render_options.pixel_potrace {
        options.trace_options.opt_tolerance = 0.0;
    }

    Ok(options)
}

fn parse_threshold(value: &str) -> Result<ThresholdMode, JsValue> {
    if value == "auto" {
        return Ok(ThresholdMode::Auto);
    }

    value
        .parse::<u8>()
        .map(ThresholdMode::Fixed)
        .map_err(|_| JsValue::from_str(&format!("invalid threshold: {value}")))
}

fn parse_alpha_background(value: &str) -> Result<AlphaBackground, JsValue> {
    match value {
        "black" => Ok(AlphaBackground::Black),
        "white" => Ok(AlphaBackground::White),
        value => Err(JsValue::from_str(&format!(
            "invalid alphaBackground: {value}"
        ))),
    }
}

fn parse_contour_mode(value: &str) -> Result<ContourMode, JsValue> {
    match value {
        "pixel" => Ok(ContourMode::Pixel),
        "subpixel" => Ok(ContourMode::Subpixel),
        "scalar" => Ok(ContourMode::Scalar),
        value => Err(JsValue::from_str(&format!("invalid contourMode: {value}"))),
    }
}

fn parse_curve_mode(value: &str) -> Result<CurveMode, JsValue> {
    match value {
        "polygon" => Ok(CurveMode::Polygon),
        "smooth" => Ok(CurveMode::Smooth),
        "spline" => Ok(CurveMode::Spline),
        "fit" => Ok(CurveMode::Fit),
        "potrace" => Ok(CurveMode::Potrace),
        value => Err(JsValue::from_str(&format!("invalid curveMode: {value}"))),
    }
}

fn pixel_potrace_default(contour_mode: ContourMode, svg_options: SvgRenderOptions) -> bool {
    contour_mode == ContourMode::Pixel && svg_options.curve_mode == CurveMode::Potrace
}
