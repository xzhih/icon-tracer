mod api;
mod components;
mod optimize;
mod raster;
mod svg;
mod trace;

pub use api::{trace_image_to_svg, TraceImageError, TraceImageOptions, TracePreset};
pub use components::{
    analyze_components, BinaryMask, Bounds, ComponentAnalysis, ComponentFacts, FloatPoint,
    HoleFacts,
};
pub use optimize::{compare_icon_masks, optimize_icon_trace};
pub use raster::{
    AlphaBackground, Bitmap, BitmapError, BmpError, JpegError, PngError, PnmError, RasterError,
    RasterOptions, Rgba8, RgbaImage, ScalarField, ThresholdMode,
};
pub use trace::{trace_bitmap, trace_scalar_field};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceOptions {
    pub turd_size: usize,
    pub opt_tolerance: f64,
    pub contour_mode: ContourMode,
    pub preserve_collinear: bool,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            turd_size: 0,
            opt_tolerance: 0.0,
            contour_mode: ContourMode::Pixel,
            preserve_collinear: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContourMode {
    #[default]
    Pixel,
    Subpixel,
    Scalar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TracePath {
    pub points: Vec<(f64, f64)>,
    pub is_hole: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TracedBitmap {
    pub width: usize,
    pub height: usize,
    pub paths: Vec<TracePath>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconDiffMetrics {
    pub total_pixels: usize,
    pub target_foreground_pixels: usize,
    pub candidate_foreground_pixels: usize,
    pub true_positive_pixels: usize,
    pub false_positive_pixels: usize,
    pub false_negative_pixels: usize,
    pub xor_pixels: usize,
    pub xor_ratio: f64,
    pub foreground_error_ratio: f64,
    pub false_positive_ratio: f64,
    pub false_negative_ratio: f64,
    pub precision: f64,
    pub recall: f64,
    pub iou: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconOptimizationCandidate {
    pub trace_options: TraceOptions,
    pub metrics: IconDiffMetrics,
    pub score: f64,
    pub path_count: usize,
    pub point_count: usize,
    pub svg_command_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconOptimizationResult {
    pub traced: TracedBitmap,
    pub svg_options: SvgOptions,
    pub best_candidate: IconOptimizationCandidate,
    pub candidates: Vec<IconOptimizationCandidate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconOptimizeOptions {
    pub raster_options: RasterOptions,
    pub trace_options: TraceOptions,
    pub svg_options: SvgOptions,
    pub contour_modes: Vec<ContourMode>,
    pub opt_tolerances: Vec<f64>,
    pub complexity_weight: f64,
    pub isolate_foreground: bool,
}

impl Default for IconOptimizeOptions {
    fn default() -> Self {
        Self {
            raster_options: RasterOptions::default(),
            trace_options: TraceOptions {
                turd_size: 2,
                opt_tolerance: 0.75,
                contour_mode: ContourMode::Subpixel,
                preserve_collinear: false,
            },
            svg_options: SvgOptions {
                curve_mode: CurveMode::Potrace,
            },
            contour_modes: vec![ContourMode::Scalar, ContourMode::Subpixel],
            opt_tolerances: vec![0.25, 0.5, 0.75, 1.0, 1.25],
            complexity_weight: 0.5,
            isolate_foreground: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SvgOptions {
    pub curve_mode: CurveMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgRenderOptions {
    pub curve_mode: CurveMode,
    pub opt_tolerance: f64,
    pub pixel_potrace: bool,
}

impl Default for SvgRenderOptions {
    fn default() -> Self {
        Self {
            curve_mode: CurveMode::Polygon,
            opt_tolerance: 0.2,
            pixel_potrace: false,
        }
    }
}

impl From<SvgOptions> for SvgRenderOptions {
    fn from(options: SvgOptions) -> Self {
        Self {
            curve_mode: options.curve_mode,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CurveMode {
    #[default]
    Polygon,
    Smooth,
    Spline,
    Fit,
    Potrace,
}

impl TracedBitmap {
    pub fn to_svg(&self) -> String {
        self.to_svg_with_options(SvgOptions::default())
    }

    pub fn to_svg_with_options(&self, options: SvgOptions) -> String {
        self.to_svg_with_render_options(options.into())
    }

    pub fn to_svg_with_render_options(&self, options: SvgRenderOptions) -> String {
        let has_holes = self.paths.iter().any(|path| path.is_hole);
        let has_sibling_paths = self.paths.len() > 1;
        let path_data = self
            .paths
            .iter()
            .filter_map(|path| {
                svg::path_to_svg_data_with_context(
                    path,
                    options,
                    Some((self.width, self.height)),
                    has_holes,
                    has_sibling_paths,
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let precision = if options.pixel_potrace {
            self.paths
                .iter()
                .map(|path| {
                    svg::pixel_potrace_path_precision_preference(
                        path,
                        Some((self.width, self.height)),
                        has_holes,
                        has_sibling_paths,
                        options.opt_tolerance.max(0.0),
                    )
                })
                .fold(svg::SvgPathPrecision::Compact, svg::SvgPathPrecision::max)
        } else {
            svg::SvgPathPrecision::Compact
        };
        let path = svg::svg_path_element_with_precision(
            &path_data,
            options.pixel_potrace,
            self.height,
            precision,
        );

        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">{}</svg>"#,
            self.width, self.height, path
        )
    }

    pub fn to_mask(&self) -> BinaryMask {
        let mut pixels = vec![false; self.width.saturating_mul(self.height)];

        for path in &self.paths {
            trace::rasterize_path_evenodd(path, self.width, self.height, &mut pixels);
        }

        BinaryMask {
            width: self.width,
            height: self.height,
            pixels,
        }
    }
}

#[cfg(test)]
use optimize::*;
#[cfg(test)]
use svg::*;

#[cfg(test)]
mod tests;
