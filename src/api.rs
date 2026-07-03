use std::error::Error;
use std::fmt;

use crate::{
    optimize_icon_trace, trace_bitmap, trace_scalar_field, Bitmap, BitmapError, ContourMode,
    CurveMode, IconOptimizeOptions, RasterError, RasterOptions, RgbaImage, SvgOptions,
    SvgRenderOptions, ThresholdMode, TraceOptions,
};

/// Preset defaults shared by the CLI and library API.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TracePreset {
    Default,
    Logo,
    Scan,
    #[default]
    Icon,
}

/// High-level options for tracing encoded image bytes directly to SVG.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceImageOptions {
    pub raster_options: RasterOptions,
    pub trace_options: TraceOptions,
    pub svg_render_options: SvgRenderOptions,
    pub optimize_icon: bool,
    pub isolate_foreground: bool,
}

impl TraceImageOptions {
    pub fn preset(preset: TracePreset) -> Self {
        match preset {
            TracePreset::Default => Self {
                raster_options: RasterOptions {
                    threshold: ThresholdMode::Auto,
                    invert: false,
                    ..RasterOptions::default()
                },
                trace_options: TraceOptions {
                    contour_mode: ContourMode::Pixel,
                    turd_size: 0,
                    opt_tolerance: 0.0,
                    preserve_collinear: false,
                },
                svg_render_options: SvgRenderOptions {
                    curve_mode: CurveMode::Polygon,
                    ..SvgRenderOptions::default()
                },
                optimize_icon: false,
                isolate_foreground: false,
            },
            TracePreset::Logo => Self {
                raster_options: RasterOptions {
                    threshold: ThresholdMode::Auto,
                    invert: false,
                    ..RasterOptions::default()
                },
                trace_options: TraceOptions {
                    contour_mode: ContourMode::Subpixel,
                    turd_size: 4,
                    opt_tolerance: 0.75,
                    preserve_collinear: false,
                },
                svg_render_options: SvgRenderOptions {
                    curve_mode: CurveMode::Potrace,
                    ..SvgRenderOptions::default()
                },
                optimize_icon: false,
                isolate_foreground: false,
            },
            TracePreset::Scan => Self {
                raster_options: RasterOptions {
                    threshold: ThresholdMode::Auto,
                    invert: false,
                    ..RasterOptions::default()
                },
                trace_options: TraceOptions {
                    contour_mode: ContourMode::Pixel,
                    turd_size: 2,
                    opt_tolerance: 0.0,
                    preserve_collinear: false,
                },
                svg_render_options: SvgRenderOptions {
                    curve_mode: CurveMode::Polygon,
                    ..SvgRenderOptions::default()
                },
                optimize_icon: false,
                isolate_foreground: false,
            },
            TracePreset::Icon => Self {
                raster_options: RasterOptions {
                    threshold: ThresholdMode::Auto,
                    invert: false,
                    ..RasterOptions::default()
                },
                trace_options: TraceOptions {
                    contour_mode: ContourMode::Subpixel,
                    turd_size: 2,
                    opt_tolerance: 0.75,
                    preserve_collinear: false,
                },
                svg_render_options: SvgRenderOptions {
                    curve_mode: CurveMode::Potrace,
                    ..SvgRenderOptions::default()
                },
                optimize_icon: false,
                isolate_foreground: false,
            },
        }
    }
}

impl Default for TraceImageOptions {
    fn default() -> Self {
        Self::preset(TracePreset::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceImageError {
    Raster(RasterError),
    Bitmap(BitmapError),
}

impl fmt::Display for TraceImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raster(error) => write!(formatter, "{error}"),
            Self::Bitmap(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for TraceImageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Raster(error) => Some(error),
            Self::Bitmap(error) => Some(error),
        }
    }
}

impl From<RasterError> for TraceImageError {
    fn from(error: RasterError) -> Self {
        Self::Raster(error)
    }
}

impl From<BitmapError> for TraceImageError {
    fn from(error: BitmapError) -> Self {
        Self::Bitmap(error)
    }
}

pub fn trace_image_to_svg(
    bytes: &[u8],
    options: TraceImageOptions,
) -> Result<String, TraceImageError> {
    if options.optimize_icon {
        let image = RgbaImage::from_bytes(bytes)?;
        let result = optimize_icon_trace(
            &image,
            IconOptimizeOptions {
                raster_options: options.raster_options,
                trace_options: options.trace_options,
                svg_options: SvgOptions {
                    curve_mode: options.svg_render_options.curve_mode,
                },
                isolate_foreground: options.isolate_foreground,
                ..IconOptimizeOptions::default()
            },
        )?;

        return Ok(result.to_svg());
    }

    if options.trace_options.contour_mode == ContourMode::Scalar {
        let image = RgbaImage::from_bytes(bytes)?;
        let field = image.to_scalar_field(options.raster_options.alpha_background);
        return Ok(
            trace_scalar_field(&field, options.raster_options, options.trace_options)?
                .to_svg_with_render_options(options.svg_render_options),
        );
    }

    let bitmap = Bitmap::from_bytes(bytes, options.raster_options)?;
    let mut trace_options = options.trace_options;
    if options.svg_render_options.pixel_potrace
        && bitmap.width().saturating_mul(bitmap.height()) >= 64 * 64
    {
        trace_options.preserve_collinear = true;
    }

    Ok(trace_bitmap(&bitmap, trace_options).to_svg_with_render_options(options.svg_render_options))
}
