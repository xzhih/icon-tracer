use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::io::Cursor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: usize,
    height: usize,
    pixels: Vec<Rgba8>,
}

impl RgbaImage {
    pub fn from_rows(width: usize, height: usize, pixels: &[Rgba8]) -> Result<Self, BitmapError> {
        if pixels.len() != width.saturating_mul(height) {
            return Err(BitmapError::DimensionMismatch {
                width,
                height,
                pixels: pixels.len(),
            });
        }

        Ok(Self {
            width,
            height,
            pixels: pixels.to_vec(),
        })
    }

    pub fn from_png(bytes: &[u8]) -> Result<Self, PngError> {
        parse_png_rgba_image(bytes)
    }

    pub fn from_jpeg(bytes: &[u8]) -> Result<Self, JpegError> {
        parse_jpeg_rgba_image(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RasterError> {
        if bytes.get(0..8) == Some(b"\x89PNG\r\n\x1a\n") {
            return Self::from_png(bytes).map_err(RasterError::Png);
        }

        if bytes.get(0..2) == Some(b"\xff\xd8") {
            return Self::from_jpeg(bytes).map_err(RasterError::Jpeg);
        }

        Err(RasterError::UnsupportedFormat)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixels(&self) -> &[Rgba8] {
        &self.pixels
    }

    pub fn pixel(&self, x: usize, y: usize) -> Rgba8 {
        self.pixels[y * self.width + x]
    }

    pub fn to_scalar_field(&self, alpha_background: AlphaBackground) -> ScalarField {
        ScalarField {
            width: self.width,
            height: self.height,
            samples: rgba_pixels_to_luma_samples(&self.pixels, alpha_background),
        }
    }

    pub fn to_bitmap(&self, options: RasterOptions) -> Result<Bitmap, BitmapError> {
        Bitmap::from_rows(
            self.width,
            self.height,
            &rgba_pixels_to_binary_pixels(&self.pixels, options),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarField {
    width: usize,
    height: usize,
    samples: Vec<u8>,
}

impl ScalarField {
    pub fn from_rows(width: usize, height: usize, samples: &[u8]) -> Result<Self, BitmapError> {
        if samples.len() != width.saturating_mul(height) {
            return Err(BitmapError::DimensionMismatch {
                width,
                height,
                pixels: samples.len(),
            });
        }

        Ok(Self {
            width,
            height,
            samples: samples.to_vec(),
        })
    }

    pub fn from_rgba_image(image: &RgbaImage, alpha_background: AlphaBackground) -> Self {
        image.to_scalar_field(alpha_background)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn samples(&self) -> &[u8] {
        &self.samples
    }

    pub fn sample(&self, x: usize, y: usize) -> u8 {
        self.samples[y * self.width + x]
    }

    pub fn to_bitmap(&self, options: RasterOptions) -> Result<Bitmap, BitmapError> {
        Bitmap::from_rows(
            self.width,
            self.height,
            &samples_to_pixels(&self.samples, options),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryMask {
    width: usize,
    height: usize,
    pixels: Vec<bool>,
}

impl BinaryMask {
    pub fn from_rows(width: usize, height: usize, pixels: &[bool]) -> Result<Self, BitmapError> {
        if pixels.len() != width.saturating_mul(height) {
            return Err(BitmapError::DimensionMismatch {
                width,
                height,
                pixels: pixels.len(),
            });
        }

        Ok(Self {
            width,
            height,
            pixels: pixels.to_vec(),
        })
    }

    pub fn from_bitmap(bitmap: &Bitmap) -> Self {
        Self {
            width: bitmap.width,
            height: bitmap.height,
            pixels: bitmap.pixels.clone(),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixels(&self) -> &[bool] {
        &self.pixels
    }

    pub fn is_foreground(&self, x: usize, y: usize) -> bool {
        self.pixels[y * self.width + x]
    }

    pub fn to_bitmap(&self) -> Bitmap {
        Bitmap {
            width: self.width,
            height: self.height,
            pixels: self.pixels.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub min_x: usize,
    pub min_y: usize,
    pub max_x: usize,
    pub max_y: usize,
}

impl Bounds {
    pub fn width(self) -> usize {
        self.max_x - self.min_x + 1
    }

    pub fn height(self) -> usize {
        self.max_y - self.min_y + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleFacts {
    pub area_pixels: usize,
    pub bbox: Bounds,
    pub centroid: FloatPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentFacts {
    pub id: usize,
    pub area_pixels: usize,
    pub bbox: Bounds,
    pub centroid: FloatPoint,
    pub touches_canvas_edge: bool,
    pub holes: Vec<HoleFacts>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentAnalysis {
    pub width: usize,
    pub height: usize,
    pub components: Vec<ComponentFacts>,
    pub interior_component_count: usize,
    pub edge_touching_component_count: usize,
}

pub fn analyze_components(mask: &BinaryMask, min_pixels: usize) -> ComponentAnalysis {
    let raw_components = connected_components(mask, min_pixels.max(1));
    let components = raw_components
        .iter()
        .enumerate()
        .map(|(index, component)| component_facts(index + 1, component, mask))
        .collect::<Vec<_>>();

    ComponentAnalysis {
        width: mask.width,
        height: mask.height,
        interior_component_count: components
            .iter()
            .filter(|component| !component.touches_canvas_edge)
            .count(),
        edge_touching_component_count: components
            .iter()
            .filter(|component| component.touches_canvas_edge)
            .count(),
        components,
    }
}

#[derive(Debug, Clone)]
struct RawComponent {
    pixels: Vec<usize>,
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
    sum_x: usize,
    sum_y: usize,
}

fn connected_components(mask: &BinaryMask, min_pixels: usize) -> Vec<RawComponent> {
    let mut visited = vec![false; mask.pixels.len()];
    let mut queue = Vec::new();
    let mut components = Vec::new();

    for start in 0..mask.pixels.len() {
        if !mask.pixels[start] || visited[start] {
            continue;
        }

        let mut component = RawComponent {
            pixels: Vec::new(),
            min_x: mask.width,
            min_y: mask.height,
            max_x: 0,
            max_y: 0,
            sum_x: 0,
            sum_y: 0,
        };
        queue.clear();
        queue.push(start);
        visited[start] = true;

        let mut cursor = 0;
        while cursor < queue.len() {
            let current = queue[cursor];
            cursor += 1;
            let x = current % mask.width;
            let y = current / mask.width;

            component.pixels.push(current);
            component.min_x = component.min_x.min(x);
            component.min_y = component.min_y.min(y);
            component.max_x = component.max_x.max(x);
            component.max_y = component.max_y.max(y);
            component.sum_x += x;
            component.sum_y += y;

            for (next_x, next_y) in orthogonal_neighbors(x, y, mask.width, mask.height) {
                let index = next_y * mask.width + next_x;
                if mask.pixels[index] && !visited[index] {
                    visited[index] = true;
                    queue.push(index);
                }
            }
        }

        if component.pixels.len() >= min_pixels {
            components.push(component);
        }
    }

    components.sort_by_key(|component| std::cmp::Reverse(component.pixels.len()));
    components
}

fn orthogonal_neighbors(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> impl Iterator<Item = (usize, usize)> {
    let mut neighbors = [(0usize, 0usize); 4];
    let mut count = 0usize;

    if x > 0 {
        neighbors[count] = (x - 1, y);
        count += 1;
    }
    if x + 1 < width {
        neighbors[count] = (x + 1, y);
        count += 1;
    }
    if y > 0 {
        neighbors[count] = (x, y - 1);
        count += 1;
    }
    if y + 1 < height {
        neighbors[count] = (x, y + 1);
        count += 1;
    }

    neighbors.into_iter().take(count)
}

fn component_facts(id: usize, component: &RawComponent, mask: &BinaryMask) -> ComponentFacts {
    let bbox = Bounds {
        min_x: component.min_x,
        min_y: component.min_y,
        max_x: component.max_x,
        max_y: component.max_y,
    };
    let touches_canvas_edge = bbox.min_x == 0
        || bbox.min_y == 0
        || bbox.max_x + 1 == mask.width
        || bbox.max_y + 1 == mask.height;

    ComponentFacts {
        id,
        area_pixels: component.pixels.len(),
        bbox,
        centroid: FloatPoint {
            x: component.sum_x as f64 / component.pixels.len() as f64,
            y: component.sum_y as f64 / component.pixels.len() as f64,
        },
        touches_canvas_edge,
        holes: detect_component_holes(component, mask),
    }
}

fn detect_component_holes(component: &RawComponent, mask: &BinaryMask) -> Vec<HoleFacts> {
    let bbox = Bounds {
        min_x: component.min_x,
        min_y: component.min_y,
        max_x: component.max_x,
        max_y: component.max_y,
    };
    let local_width = bbox.width();
    let local_height = bbox.height();
    let mut foreground = vec![false; local_width * local_height];

    for y in bbox.min_y..=bbox.max_y {
        for x in bbox.min_x..=bbox.max_x {
            if mask.is_foreground(x, y) {
                foreground[(y - bbox.min_y) * local_width + (x - bbox.min_x)] = true;
            }
        }
    }

    let mut visited = vec![false; foreground.len()];
    let mut queue = Vec::new();
    let mut holes = Vec::new();

    for start in 0..foreground.len() {
        if foreground[start] || visited[start] {
            continue;
        }

        let mut touches_edge = false;
        let mut pixels = 0usize;
        let mut min_x = local_width;
        let mut min_y = local_height;
        let mut max_x = 0usize;
        let mut max_y = 0usize;
        let mut sum_x = 0usize;
        let mut sum_y = 0usize;

        queue.clear();
        queue.push(start);
        visited[start] = true;
        let mut cursor = 0;

        while cursor < queue.len() {
            let current = queue[cursor];
            cursor += 1;
            let x = current % local_width;
            let y = current / local_width;

            if x == 0 || y == 0 || x + 1 == local_width || y + 1 == local_height {
                touches_edge = true;
            }

            pixels += 1;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            sum_x += x + bbox.min_x;
            sum_y += y + bbox.min_y;

            for (next_x, next_y) in orthogonal_neighbors(x, y, local_width, local_height) {
                let index = next_y * local_width + next_x;
                if !foreground[index] && !visited[index] {
                    visited[index] = true;
                    queue.push(index);
                }
            }
        }

        if !touches_edge {
            holes.push(HoleFacts {
                area_pixels: pixels,
                bbox: Bounds {
                    min_x: min_x + bbox.min_x,
                    min_y: min_y + bbox.min_y,
                    max_x: max_x + bbox.min_x,
                    max_y: max_y + bbox.min_y,
                },
                centroid: FloatPoint {
                    x: sum_x as f64 / pixels as f64,
                    y: sum_y as f64 / pixels as f64,
                },
            });
        }
    }

    holes.sort_by_key(|hole| std::cmp::Reverse(hole.area_pixels));
    holes
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    width: usize,
    height: usize,
    pixels: Vec<bool>,
}

impl Bitmap {
    pub fn from_rows(width: usize, height: usize, pixels: &[bool]) -> Result<Self, BitmapError> {
        if pixels.len() != width.saturating_mul(height) {
            return Err(BitmapError::DimensionMismatch {
                width,
                height,
                pixels: pixels.len(),
            });
        }

        Ok(Self {
            width,
            height,
            pixels: pixels.to_vec(),
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn is_black(&self, x: usize, y: usize) -> bool {
        self.pixels[y * self.width + x]
    }

    pub fn as_mask(&self) -> BinaryMask {
        BinaryMask::from_bitmap(self)
    }

    pub fn from_pnm(bytes: &[u8], options: RasterOptions) -> Result<Self, PnmError> {
        PnmParser::new(bytes).parse(options)
    }

    pub fn from_bmp(bytes: &[u8], options: RasterOptions) -> Result<Self, BmpError> {
        BmpParser::new(bytes).parse(options)
    }

    pub fn from_png(bytes: &[u8], options: RasterOptions) -> Result<Self, PngError> {
        parse_png(bytes, options)
    }

    pub fn from_jpeg(bytes: &[u8], options: RasterOptions) -> Result<Self, JpegError> {
        parse_jpeg(bytes, options)
    }

    pub fn from_bytes(bytes: &[u8], options: RasterOptions) -> Result<Self, RasterError> {
        if bytes.get(0..8) == Some(b"\x89PNG\r\n\x1a\n") {
            return Self::from_png(bytes, options).map_err(RasterError::Png);
        }

        if bytes.get(0..2) == Some(b"\xff\xd8") {
            return Self::from_jpeg(bytes, options).map_err(RasterError::Jpeg);
        }

        if matches!(
            bytes.get(0..2),
            Some(b"P1" | b"P2" | b"P3" | b"P4" | b"P5" | b"P6")
        ) {
            return Self::from_pnm(bytes, options).map_err(RasterError::Pnm);
        }

        if matches!(bytes.get(0..2), Some(b"BM")) {
            return Self::from_bmp(bytes, options).map_err(RasterError::Bmp);
        }

        Err(RasterError::UnsupportedFormat)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitmapError {
    DimensionMismatch {
        width: usize,
        height: usize,
        pixels: usize,
    },
}

impl fmt::Display for BitmapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch {
                width,
                height,
                pixels,
            } => write!(
                formatter,
                "bitmap dimensions {}x{} require {} pixels, got {}",
                width,
                height,
                width.saturating_mul(*height),
                pixels
            ),
        }
    }
}

impl std::error::Error for BitmapError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterOptions {
    pub threshold: ThresholdMode,
    pub invert: bool,
    pub alpha_background: AlphaBackground,
}

impl Default for RasterOptions {
    fn default() -> Self {
        Self {
            threshold: ThresholdMode::Auto,
            invert: false,
            alpha_background: AlphaBackground::White,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AlphaBackground {
    Black,
    #[default]
    White,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdMode {
    Fixed(u8),
    Auto,
}

impl ThresholdMode {
    fn resolve(self, samples: &[u8]) -> u8 {
        match self {
            Self::Fixed(threshold) => threshold,
            Self::Auto => otsu_threshold(samples),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PnmError {
    UnexpectedEof,
    InvalidToken(String),
    InvalidDimensions { width: usize, height: usize },
    UnsupportedFormat(String),
    UnsupportedMaxValue(u32),
}

impl fmt::Display for PnmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(formatter, "unexpected end of PNM data"),
            Self::InvalidToken(token) => write!(formatter, "invalid PNM token: {token}"),
            Self::InvalidDimensions { width, height } => {
                write!(formatter, "invalid PNM dimensions: {width}x{height}")
            }
            Self::UnsupportedFormat(format) => {
                write!(formatter, "unsupported PNM format: {format}")
            }
            Self::UnsupportedMaxValue(max_value) => {
                write!(formatter, "unsupported PNM max value: {max_value}")
            }
        }
    }
}

impl std::error::Error for PnmError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmpError {
    UnexpectedEof,
    InvalidSignature,
    InvalidDimensions { width: i32, height: i32 },
    UnsupportedDibHeader(u32),
    UnsupportedPlanes(u16),
    UnsupportedCompression(u32),
    UnsupportedBitsPerPixel(u16),
    InvalidPixelOffset(usize),
}

impl fmt::Display for BmpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(formatter, "unexpected end of BMP data"),
            Self::InvalidSignature => write!(formatter, "invalid BMP signature"),
            Self::InvalidDimensions { width, height } => {
                write!(formatter, "invalid BMP dimensions: {width}x{height}")
            }
            Self::UnsupportedDibHeader(size) => {
                write!(formatter, "unsupported BMP DIB header size: {size}")
            }
            Self::UnsupportedPlanes(planes) => {
                write!(formatter, "unsupported BMP color planes: {planes}")
            }
            Self::UnsupportedCompression(compression) => {
                write!(formatter, "unsupported BMP compression: {compression}")
            }
            Self::UnsupportedBitsPerPixel(bits_per_pixel) => {
                write!(
                    formatter,
                    "unsupported BMP bits per pixel: {bits_per_pixel}"
                )
            }
            Self::InvalidPixelOffset(offset) => {
                write!(formatter, "invalid BMP pixel offset: {offset}")
            }
        }
    }
}

impl std::error::Error for BmpError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RasterError {
    Pnm(PnmError),
    Bmp(BmpError),
    Png(PngError),
    Jpeg(JpegError),
    UnsupportedFormat,
}

impl fmt::Display for RasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pnm(error) => write!(formatter, "{error}"),
            Self::Bmp(error) => write!(formatter, "{error}"),
            Self::Png(error) => write!(formatter, "{error}"),
            Self::Jpeg(error) => write!(formatter, "{error}"),
            Self::UnsupportedFormat => write!(formatter, "unsupported raster format"),
        }
    }
}

impl std::error::Error for RasterError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngError {
    Decode(String),
    UnsupportedColorType(String),
    InvalidDimensions { width: u32, height: u32 },
}

impl fmt::Display for PngError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(formatter, "failed to decode PNG: {error}"),
            Self::UnsupportedColorType(color_type) => {
                write!(formatter, "unsupported PNG color type: {color_type}")
            }
            Self::InvalidDimensions { width, height } => {
                write!(formatter, "invalid PNG dimensions: {width}x{height}")
            }
        }
    }
}

impl std::error::Error for PngError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JpegError {
    Decode(String),
    MissingInfo,
    UnsupportedPixelFormat(String),
    InvalidDimensions { width: u16, height: u16 },
}

impl fmt::Display for JpegError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(formatter, "failed to decode JPEG: {error}"),
            Self::MissingInfo => write!(formatter, "failed to decode JPEG metadata"),
            Self::UnsupportedPixelFormat(pixel_format) => {
                write!(formatter, "unsupported JPEG pixel format: {pixel_format}")
            }
            Self::InvalidDimensions { width, height } => {
                write!(formatter, "invalid JPEG dimensions: {width}x{height}")
            }
        }
    }
}

impl std::error::Error for JpegError {}

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
        let path_data = self
            .paths
            .iter()
            .filter_map(|path| path_to_svg_data(path, options, Some((self.width, self.height))))
            .collect::<Vec<_>>()
            .join(" ");
        let path = svg_path_element(&path_data, options.pixel_potrace);

        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">{}</svg>"#,
            self.width, self.height, path
        )
    }

    pub fn to_mask(&self) -> BinaryMask {
        let mut pixels = vec![false; self.width.saturating_mul(self.height)];

        for path in &self.paths {
            rasterize_path_evenodd(path, self.width, self.height, &mut pixels);
        }

        BinaryMask {
            width: self.width,
            height: self.height,
            pixels,
        }
    }
}

fn svg_path_element(path_data: &str, allow_scaled_potrace_path: bool) -> String {
    let plain = format!(r#"<path fill="black" fill-rule="evenodd" d="{path_data}"/>"#);
    if !allow_scaled_potrace_path {
        return plain;
    }
    let mut best = plain;

    if let Some(scaled_path_data) = scaled_integer_svg_path_data(path_data, 100.0) {
        let scaled = format!(
            r#"<path fill="black" fill-rule="evenodd" transform="scale(.01)" d="{scaled_path_data}"/>"#
        );

        if scaled.len() < best.len() {
            best = scaled;
        }
    }

    if !path_data_has_arc_commands(path_data) {
        if let Some(one_decimal_path_data) = one_decimal_svg_path_data(path_data) {
            let one_decimal =
                format!(r#"<path fill="black" fill-rule="evenodd" d="{one_decimal_path_data}"/>"#);
            if one_decimal.len() < best.len() {
                best = one_decimal;
            }
        }
    }

    best
}

fn path_data_has_arc_commands(path_data: &str) -> bool {
    path_data.bytes().any(|byte| matches!(byte, b'A' | b'a'))
}

pub fn trace_bitmap(bitmap: &Bitmap, options: TraceOptions) -> TracedBitmap {
    match options.contour_mode {
        ContourMode::Pixel => trace_pixel_bitmap(bitmap, options),
        ContourMode::Subpixel | ContourMode::Scalar => trace_subpixel_bitmap(bitmap, options),
    }
}

pub fn trace_scalar_field(
    field: &ScalarField,
    raster_options: RasterOptions,
    trace_options: TraceOptions,
) -> Result<TracedBitmap, BitmapError> {
    if trace_options.contour_mode != ContourMode::Scalar {
        let bitmap = field.to_bitmap(raster_options)?;
        return Ok(trace_bitmap(&bitmap, trace_options));
    }

    Ok(trace_scalar_bitmap(field, raster_options, trace_options))
}

pub fn compare_icon_masks(
    target: &BinaryMask,
    candidate: &BinaryMask,
) -> Result<IconDiffMetrics, BitmapError> {
    if target.width != candidate.width || target.height != candidate.height {
        return Err(BitmapError::DimensionMismatch {
            width: target.width,
            height: target.height,
            pixels: candidate.pixels.len(),
        });
    }

    let mut target_foreground_pixels = 0usize;
    let mut candidate_foreground_pixels = 0usize;
    let mut true_positive_pixels = 0usize;
    let mut false_positive_pixels = 0usize;
    let mut false_negative_pixels = 0usize;

    for (target_pixel, candidate_pixel) in target.pixels.iter().zip(&candidate.pixels) {
        if *target_pixel {
            target_foreground_pixels += 1;
        }

        if *candidate_pixel {
            candidate_foreground_pixels += 1;
        }

        match (*target_pixel, *candidate_pixel) {
            (true, true) => true_positive_pixels += 1,
            (false, true) => false_positive_pixels += 1,
            (true, false) => false_negative_pixels += 1,
            (false, false) => {}
        }
    }

    let total_pixels = target.pixels.len();
    let xor_pixels = false_positive_pixels + false_negative_pixels;
    let union_pixels = true_positive_pixels + false_positive_pixels + false_negative_pixels;
    let predicted_pixels = true_positive_pixels + false_positive_pixels;
    let target_positive_pixels = true_positive_pixels + false_negative_pixels;

    Ok(IconDiffMetrics {
        total_pixels,
        target_foreground_pixels,
        candidate_foreground_pixels,
        true_positive_pixels,
        false_positive_pixels,
        false_negative_pixels,
        xor_pixels,
        xor_ratio: ratio(xor_pixels, total_pixels),
        foreground_error_ratio: ratio(xor_pixels, target_foreground_pixels.max(1)),
        false_positive_ratio: ratio(false_positive_pixels, total_pixels),
        false_negative_ratio: ratio(false_negative_pixels, target_foreground_pixels.max(1)),
        precision: if predicted_pixels == 0 {
            1.0
        } else {
            ratio(true_positive_pixels, predicted_pixels)
        },
        recall: if target_positive_pixels == 0 {
            1.0
        } else {
            ratio(true_positive_pixels, target_positive_pixels)
        },
        iou: if union_pixels == 0 {
            1.0
        } else {
            ratio(true_positive_pixels, union_pixels)
        },
    })
}

pub fn optimize_icon_trace(
    image: &RgbaImage,
    options: IconOptimizeOptions,
) -> Result<IconOptimizationResult, BitmapError> {
    let source_scalar_field = image.to_scalar_field(options.raster_options.alpha_background);
    let source_threshold = options
        .raster_options
        .threshold
        .resolve(source_scalar_field.samples());
    let source_bitmap = image.to_bitmap(options.raster_options)?;
    let source_mask = source_bitmap.as_mask();
    let target_mask = if options.isolate_foreground {
        isolate_icon_foreground_mask(image, options.raster_options, &source_mask)
    } else {
        source_mask.clone()
    };
    let target_bitmap = target_mask.to_bitmap();
    let foreground_isolated = target_mask != source_mask;
    let scalar_field = if foreground_isolated {
        masked_scalar_field(&source_scalar_field, &target_mask, options.raster_options)
    } else {
        source_scalar_field.clone()
    };
    let scalar_raster_options = if foreground_isolated {
        RasterOptions {
            threshold: ThresholdMode::Fixed(source_threshold),
            ..options.raster_options
        }
    } else {
        options.raster_options
    };
    let contour_modes = if options.contour_modes.is_empty() {
        vec![options.trace_options.contour_mode]
    } else {
        options.contour_modes.clone()
    };
    let opt_tolerances = if options.opt_tolerances.is_empty() {
        vec![options.trace_options.opt_tolerance]
    } else {
        options.opt_tolerances.clone()
    };

    let mut evaluated_candidates = Vec::new();

    for contour_mode in contour_modes {
        for opt_tolerance in &opt_tolerances {
            let trace_options = TraceOptions {
                contour_mode,
                opt_tolerance: opt_tolerance.max(0.0),
                ..options.trace_options
            };
            let traced = if contour_mode == ContourMode::Scalar {
                trace_scalar_field(&scalar_field, scalar_raster_options, trace_options)?
            } else {
                trace_bitmap(&target_bitmap, trace_options)
            };
            let candidate_mask = traced.to_mask();
            let metrics = compare_icon_masks(&target_mask, &candidate_mask)?;
            let path_count = traced.paths.len();
            let point_count = traced_point_count(&traced);
            let svg_command_count = traced_svg_command_count(&traced, options.svg_options);
            let score = icon_candidate_score(
                metrics,
                point_count,
                svg_command_count,
                options.complexity_weight,
            );
            let candidate = IconOptimizationCandidate {
                trace_options,
                metrics,
                score,
                path_count,
                point_count,
                svg_command_count,
            };

            evaluated_candidates.push((candidate, traced));
        }
    }

    let candidates = evaluated_candidates
        .iter()
        .map(|(candidate, _)| candidate.clone())
        .collect::<Vec<_>>();
    let best_index = best_icon_candidate_index(&candidates)
        .expect("optimizer should evaluate at least one candidate");
    let best_candidate = candidates[best_index].clone();
    let best_traced = evaluated_candidates[best_index].1.clone();

    Ok(IconOptimizationResult {
        traced: best_traced,
        svg_options: options.svg_options,
        best_candidate,
        candidates,
    })
}

impl IconOptimizationResult {
    pub fn to_svg(&self) -> String {
        self.traced.to_svg_with_options(self.svg_options)
    }
}

fn isolate_icon_foreground_mask(
    image: &RgbaImage,
    raster_options: RasterOptions,
    fallback: &BinaryMask,
) -> BinaryMask {
    let edge_pruned_fallback = prune_edge_touching_foreground(fallback);
    let pruned_fallback = prune_peripheral_foreground_noise(&edge_pruned_fallback, fallback);
    let fallback_area = mask_area(fallback);
    let pruned_fallback_area = mask_area(&pruned_fallback);
    let fallback_ratio = foreground_ratio(fallback_area, fallback.pixels.len());

    if mask_is_plausible_foreground(&pruned_fallback, fallback)
        && (fallback_ratio > 0.35
            || pruned_fallback_area * 10 < fallback_area * 9
            || edge_pruned_foreground_drop_is_useful(
                fallback_area,
                pruned_fallback_area,
                fallback.pixels.len(),
            ))
    {
        return pruned_fallback;
    }

    let contrast = background_contrast_mask(image, raster_options.alpha_background);
    let edge_pruned_contrast = prune_edge_touching_foreground(&contrast);
    let pruned_contrast = prune_peripheral_foreground_noise(&edge_pruned_contrast, &contrast);
    let contrast_area = mask_area(&pruned_contrast);
    if mask_is_plausible_foreground(&pruned_contrast, fallback)
        && (fallback_ratio > 0.35
            || contrast_area * 4 < fallback_area.saturating_mul(3)
            || !component_area_is_plausible(fallback_area, fallback.pixels.len()))
    {
        return pruned_contrast;
    }

    fallback.clone()
}

fn background_contrast_mask(image: &RgbaImage, alpha_background: AlphaBackground) -> BinaryMask {
    let background = estimate_border_rgb(image, alpha_background);
    let distances = image
        .pixels()
        .iter()
        .map(|pixel| {
            let color = composited_rgb(pixel, alpha_background);
            rgb_distance_sample(color, background)
        })
        .collect::<Vec<_>>();
    let threshold = otsu_threshold(&distances).max(18);
    let pixels = distances
        .iter()
        .map(|distance| *distance > threshold)
        .collect::<Vec<_>>();

    BinaryMask {
        width: image.width(),
        height: image.height(),
        pixels,
    }
}

fn estimate_border_rgb(image: &RgbaImage, alpha_background: AlphaBackground) -> (f64, f64, f64) {
    let mut count = 0usize;
    let mut sum = (0.0, 0.0, 0.0);

    for y in 0..image.height() {
        for x in 0..image.width() {
            if x != 0 && y != 0 && x + 1 != image.width() && y + 1 != image.height() {
                continue;
            }

            let color = composited_rgb(&image.pixel(x, y), alpha_background);
            sum.0 += color.0;
            sum.1 += color.1;
            sum.2 += color.2;
            count += 1;
        }
    }

    if count == 0 {
        (0.0, 0.0, 0.0)
    } else {
        (
            sum.0 / count as f64,
            sum.1 / count as f64,
            sum.2 / count as f64,
        )
    }
}

fn composited_rgb(pixel: &Rgba8, alpha_background: AlphaBackground) -> (f64, f64, f64) {
    (
        f64::from(composite_over_background(
            pixel.red,
            pixel.alpha,
            alpha_background,
        )),
        f64::from(composite_over_background(
            pixel.green,
            pixel.alpha,
            alpha_background,
        )),
        f64::from(composite_over_background(
            pixel.blue,
            pixel.alpha,
            alpha_background,
        )),
    )
}

fn rgb_distance_sample(color: (f64, f64, f64), background: (f64, f64, f64)) -> u8 {
    let distance = ((color.0 - background.0).powi(2)
        + (color.1 - background.1).powi(2)
        + (color.2 - background.2).powi(2))
    .sqrt();

    ((distance / (3.0_f64.sqrt() * 255.0)) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn prune_edge_touching_foreground(mask: &BinaryMask) -> BinaryMask {
    let components = connected_components(mask, 1);
    let interior_area = components
        .iter()
        .filter(|component| !raw_component_touches_edge(component, mask.width, mask.height))
        .map(|component| component.pixels.len())
        .sum::<usize>();

    if !component_area_is_plausible(interior_area, mask.pixels.len()) {
        return mask.clone();
    }

    let mut pixels = vec![false; mask.pixels.len()];
    for component in components {
        if raw_component_touches_edge(&component, mask.width, mask.height) {
            continue;
        }

        for index in component.pixels {
            pixels[index] = true;
        }
    }

    BinaryMask {
        width: mask.width,
        height: mask.height,
        pixels,
    }
}

fn prune_peripheral_foreground_noise(mask: &BinaryMask, original: &BinaryMask) -> BinaryMask {
    let components = connected_components(mask, 1);
    if components.len() <= 1 {
        return mask.clone();
    }

    let original_area = mask_area(original);
    let mask_area = mask_area(mask);
    if mask_area >= original_area {
        return mask.clone();
    }

    let largest_area = components[0].pixels.len();
    if !component_area_is_plausible(largest_area, mask.pixels.len()) {
        return mask.clone();
    }

    let corner_band = (mask.width.min(mask.height) / 10).max(2);
    let canvas_noise_cap = ((mask.pixels.len() as f64 * 0.0001).round() as usize).clamp(4, 128);
    let foreground_noise_cap = (largest_area / 500).max(4);
    let max_noise_area = canvas_noise_cap.min(foreground_noise_cap);
    let mut pixels = vec![false; mask.pixels.len()];
    let mut kept_area = 0usize;

    for component in components {
        if component.pixels.len() <= max_noise_area {
            if let Some(corner) =
                raw_component_corner_band(&component, mask.width, mask.height, corner_band)
            {
                if original_has_edge_residue_in_corner(original, corner_band, corner) {
                    continue;
                }
            }
        }

        kept_area += component.pixels.len();
        for index in component.pixels {
            pixels[index] = true;
        }
    }

    if !component_area_is_plausible(kept_area, mask.pixels.len()) {
        return mask.clone();
    }

    BinaryMask {
        width: mask.width,
        height: mask.height,
        pixels,
    }
}

fn raw_component_touches_edge(component: &RawComponent, width: usize, height: usize) -> bool {
    component.min_x == 0
        || component.min_y == 0
        || component.max_x + 1 == width
        || component.max_y + 1 == height
}

fn raw_component_corner_band(
    component: &RawComponent,
    width: usize,
    height: usize,
    band: usize,
) -> Option<(bool, bool)> {
    let near_left = component.max_x < band;
    let near_right = component.min_x.saturating_add(band) >= width;
    let near_top = component.max_y < band;
    let near_bottom = component.min_y.saturating_add(band) >= height;

    match (near_left, near_right, near_top, near_bottom) {
        (true, false, true, false) => Some((true, true)),
        (true, false, false, true) => Some((true, false)),
        (false, true, true, false) => Some((false, true)),
        (false, true, false, true) => Some((false, false)),
        _ => None,
    }
}

fn original_has_edge_residue_in_corner(
    original: &BinaryMask,
    band: usize,
    corner: (bool, bool),
) -> bool {
    connected_components(original, 1).iter().any(|component| {
        raw_component_touches_edge(component, original.width, original.height)
            && raw_component_intersects_corner_band(
                component,
                original.width,
                original.height,
                band,
                corner,
            )
    })
}

fn raw_component_intersects_corner_band(
    component: &RawComponent,
    width: usize,
    height: usize,
    band: usize,
    corner: (bool, bool),
) -> bool {
    let horizontal = if corner.0 {
        component.min_x < band
    } else {
        component.max_x.saturating_add(band) >= width
    };
    let vertical = if corner.1 {
        component.min_y < band
    } else {
        component.max_y.saturating_add(band) >= height
    };

    horizontal && vertical
}

fn mask_is_plausible_foreground(mask: &BinaryMask, fallback: &BinaryMask) -> bool {
    let area = mask_area(mask);
    let fallback_area = mask_area(fallback);

    component_area_is_plausible(area, mask.pixels.len())
        && (area <= fallback_area.saturating_mul(2).max(1)
            || foreground_ratio(fallback_area, mask.pixels.len()) > 0.35)
}

fn edge_pruned_foreground_drop_is_useful(
    fallback_area: usize,
    pruned_area: usize,
    total_pixels: usize,
) -> bool {
    if pruned_area >= fallback_area {
        return false;
    }

    let removed_area = fallback_area - pruned_area;
    let min_removed_from_canvas = ((total_pixels as f64 * 0.001).round() as usize).max(4);
    let min_removed_from_foreground = ((fallback_area as f64 * 0.02).round() as usize).max(4);
    let max_removed_from_foreground = ((fallback_area as f64 * 0.25).round() as usize).max(4);

    removed_area >= min_removed_from_canvas.max(min_removed_from_foreground)
        && removed_area <= max_removed_from_foreground
}

fn component_area_is_plausible(area: usize, total: usize) -> bool {
    if total == 0 || area == 0 {
        return false;
    }

    let ratio = foreground_ratio(area, total);
    area >= ((total as f64 * 0.005).round() as usize).max(4) && ratio <= 0.70
}

fn foreground_ratio(area: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        area as f64 / total as f64
    }
}

fn mask_area(mask: &BinaryMask) -> usize {
    mask.pixels.iter().filter(|pixel| **pixel).count()
}

fn masked_scalar_field(
    field: &ScalarField,
    mask: &BinaryMask,
    raster_options: RasterOptions,
) -> ScalarField {
    let background_sample = if raster_options.invert { 0 } else { 255 };
    let samples = field
        .samples()
        .iter()
        .zip(mask.pixels())
        .map(|(sample, keep)| if *keep { *sample } else { background_sample })
        .collect::<Vec<_>>();

    ScalarField {
        width: field.width(),
        height: field.height(),
        samples,
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn traced_point_count(traced: &TracedBitmap) -> usize {
    traced.paths.iter().map(|path| path.points.len()).sum()
}

fn traced_svg_command_count(traced: &TracedBitmap, options: SvgOptions) -> usize {
    let options = SvgRenderOptions::from(options);
    traced
        .paths
        .iter()
        .filter_map(|path| path_to_svg_data(path, options, None))
        .map(|path_data| {
            path_data
                .split_whitespace()
                .filter(|token| matches!(*token, "M" | "L" | "C" | "Q" | "Z"))
                .count()
        })
        .sum()
}

const ICON_COMPLEXITY_FIT_SCORE_BAND: f64 = 0.002;

fn icon_candidate_fit_score(metrics: IconDiffMetrics) -> f64 {
    metrics.foreground_error_ratio + metrics.false_negative_ratio * 0.5 + (1.0 - metrics.iou) * 0.25
}

fn icon_candidate_complexity_score(
    metrics: IconDiffMetrics,
    point_count: usize,
    svg_command_count: usize,
    complexity_weight: f64,
) -> f64 {
    let target_foreground_pixels = metrics.target_foreground_pixels.max(1);
    let complexity_ratio = ratio(point_count, target_foreground_pixels)
        + ratio(svg_command_count, target_foreground_pixels) * 2.0;

    complexity_weight.max(0.0) * complexity_ratio
}

fn icon_candidate_score(
    metrics: IconDiffMetrics,
    point_count: usize,
    svg_command_count: usize,
    complexity_weight: f64,
) -> f64 {
    icon_candidate_fit_score(metrics)
        + icon_candidate_complexity_score(
            metrics,
            point_count,
            svg_command_count,
            complexity_weight,
        )
}

fn best_icon_candidate_index(candidates: &[IconOptimizationCandidate]) -> Option<usize> {
    let min_fit_score = candidates
        .iter()
        .map(|candidate| icon_candidate_fit_score(candidate.metrics))
        .min_by(f64::total_cmp)?;

    candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            icon_candidate_fit_score(candidate.metrics)
                <= min_fit_score + ICON_COMPLEXITY_FIT_SCORE_BAND
        })
        .min_by(|(_, left), (_, right)| compare_eligible_icon_candidates(left, right))
        .map(|(index, _)| index)
}

fn compare_eligible_icon_candidates(
    left: &IconOptimizationCandidate,
    right: &IconOptimizationCandidate,
) -> std::cmp::Ordering {
    left.score
        .total_cmp(&right.score)
        .then_with(|| left.svg_command_count.cmp(&right.svg_command_count))
        .then_with(|| left.point_count.cmp(&right.point_count))
}

fn rasterize_path_evenodd(path: &TracePath, width: usize, height: usize, pixels: &mut [bool]) {
    if path.points.len() < 3 {
        return;
    }

    let mut intersections = Vec::new();

    for y in 0..height {
        let scan_y = y as f64 + 0.5;
        intersections.clear();

        for (start, end) in path.points.iter().zip(path.points.iter().cycle().skip(1)) {
            if (start.1 <= scan_y && scan_y < end.1) || (end.1 <= scan_y && scan_y < start.1) {
                let amount = (scan_y - start.1) / (end.1 - start.1);
                intersections.push(start.0 + (end.0 - start.0) * amount);
            }
        }

        intersections.sort_by(|a, b| a.total_cmp(b));

        for pair in intersections.chunks_exact(2) {
            let left = pair[0].min(pair[1]);
            let right = pair[0].max(pair[1]);
            let start_x = clamp_scanline_x((left - 0.5).ceil(), width);
            let end_x = clamp_scanline_x((right - 0.5).ceil(), width);

            for x in start_x..end_x {
                let index = y * width + x;
                pixels[index] = !pixels[index];
            }
        }
    }
}

fn clamp_scanline_x(value: f64, width: usize) -> usize {
    if value <= 0.0 {
        0
    } else if value >= width as f64 {
        width
    } else {
        value as usize
    }
}

fn trace_pixel_bitmap(bitmap: &Bitmap, options: TraceOptions) -> TracedBitmap {
    let edges = boundary_edges(bitmap);
    let mut outgoing: BTreeMap<Point, Vec<Edge>> = BTreeMap::new();

    for edge in &edges {
        outgoing.entry(edge.start).or_default().push(*edge);
    }

    for edges in outgoing.values_mut() {
        edges.sort();
    }

    let mut visited = HashSet::new();
    let mut paths = Vec::new();

    for edge in edges {
        if visited.contains(&edge) {
            continue;
        }

        if let Some(points) = trace_path(edge, &outgoing, &mut visited) {
            let points = if options.preserve_collinear {
                points
            } else {
                simplify_collinear(&points)
            };
            let points = optimize_path(&points, options.opt_tolerance.max(0.0));

            if points.len() >= 3 {
                let area2 = signed_area2(&points);

                if is_below_turd_size(area2, options.turd_size) {
                    continue;
                }

                paths.push(TracePath {
                    is_hole: area2 < 0,
                    points: points
                        .into_iter()
                        .map(|point| (f64::from(point.x), f64::from(point.y)))
                        .collect(),
                });
            }
        }
    }

    TracedBitmap {
        width: bitmap.width(),
        height: bitmap.height(),
        paths,
    }
}

fn trace_subpixel_bitmap(bitmap: &Bitmap, options: TraceOptions) -> TracedBitmap {
    let segments = subpixel_segments(bitmap);
    let loops = trace_subpixel_loops(&segments);
    let mut paths = Vec::new();

    for points in loops {
        let points = simplify_subpixel_collinear(&points);
        let points = optimize_subpixel_path(&points, options.opt_tolerance.max(0.0));
        let points = rotate_subpixel_loop_to_top(points);
        if points.len() < 3 {
            continue;
        }

        let area = signed_subpixel_area(&points);
        if is_below_turd_size_float(area, options.turd_size) {
            continue;
        }

        paths.push(TracePath {
            is_hole: area < 0.0,
            points: points
                .into_iter()
                .map(|point| (point.to_float().0, point.to_float().1))
                .collect(),
        });
    }

    TracedBitmap {
        width: bitmap.width(),
        height: bitmap.height(),
        paths,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SubPoint {
    x2: i32,
    y2: i32,
}

impl SubPoint {
    fn to_float(self) -> (f64, f64) {
        (f64::from(self.x2) / 2.0, f64::from(self.y2) / 2.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SubSegment {
    start: SubPoint,
    end: SubPoint,
}

impl SubSegment {
    fn new(start: SubPoint, end: SubPoint) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct IsoPoint {
    x: i64,
    y: i64,
}

impl IsoPoint {
    const SCALE: f64 = 1_000_000.0;

    fn new(x: f64, y: f64) -> Self {
        Self {
            x: (x * Self::SCALE).round() as i64,
            y: (y * Self::SCALE).round() as i64,
        }
    }

    fn to_float(self) -> (f64, f64) {
        (self.x as f64 / Self::SCALE, self.y as f64 / Self::SCALE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct IsoSegment {
    start: IsoPoint,
    end: IsoPoint,
}

impl IsoSegment {
    fn new(start: IsoPoint, end: IsoPoint) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }
}

fn trace_scalar_bitmap(
    field: &ScalarField,
    raster_options: RasterOptions,
    trace_options: TraceOptions,
) -> TracedBitmap {
    let threshold = f64::from(raster_options.threshold.resolve(field.samples()));
    let segments = scalar_segments(field, raster_options, threshold);
    let loops = trace_iso_loops(&segments);
    let mut paths = Vec::new();

    for points in loops {
        let points = optimize_iso_path(&points, trace_options.opt_tolerance.max(0.0));
        let points = rotate_iso_loop_to_top(points);
        if points.len() < 3 {
            continue;
        }

        let area = signed_iso_area(&points);
        if is_below_turd_size_float(area, trace_options.turd_size) {
            continue;
        }

        paths.push(TracePath {
            is_hole: area < 0.0,
            points: points.into_iter().map(IsoPoint::to_float).collect(),
        });
    }

    TracedBitmap {
        width: field.width(),
        height: field.height(),
        paths,
    }
}

fn scalar_segments(
    field: &ScalarField,
    raster_options: RasterOptions,
    threshold: f64,
) -> Vec<IsoSegment> {
    let mut segments = Vec::new();

    for y in 0..=field.height() {
        for x in 0..=field.width() {
            let top_left = padded_scalar_sample(field, raster_options, x, y);
            let top_right = padded_scalar_sample(field, raster_options, x + 1, y);
            let bottom_right = padded_scalar_sample(field, raster_options, x + 1, y + 1);
            let bottom_left = padded_scalar_sample(field, raster_options, x, y + 1);

            let top_left_inside = scalar_is_foreground(top_left, threshold, raster_options);
            let top_right_inside = scalar_is_foreground(top_right, threshold, raster_options);
            let bottom_right_inside = scalar_is_foreground(bottom_right, threshold, raster_options);
            let bottom_left_inside = scalar_is_foreground(bottom_left, threshold, raster_options);
            let cell = (top_left_inside as u8)
                | ((top_right_inside as u8) << 1)
                | ((bottom_right_inside as u8) << 2)
                | ((bottom_left_inside as u8) << 3);

            let top = scalar_edge_point(
                x as f64 - 0.5,
                y as f64 - 0.5,
                x as f64 + 0.5,
                y as f64 - 0.5,
                top_left,
                top_right,
                threshold,
            );
            let right = scalar_edge_point(
                x as f64 + 0.5,
                y as f64 - 0.5,
                x as f64 + 0.5,
                y as f64 + 0.5,
                top_right,
                bottom_right,
                threshold,
            );
            let bottom = scalar_edge_point(
                x as f64 - 0.5,
                y as f64 + 0.5,
                x as f64 + 0.5,
                y as f64 + 0.5,
                bottom_left,
                bottom_right,
                threshold,
            );
            let left = scalar_edge_point(
                x as f64 - 0.5,
                y as f64 - 0.5,
                x as f64 - 0.5,
                y as f64 + 0.5,
                top_left,
                bottom_left,
                threshold,
            );

            match cell {
                0 | 15 => {}
                1 => segments.push(IsoSegment::new(top, left)),
                2 => segments.push(IsoSegment::new(right, top)),
                3 => segments.push(IsoSegment::new(right, left)),
                4 => segments.push(IsoSegment::new(bottom, right)),
                5 => {
                    segments.push(IsoSegment::new(top, left));
                    segments.push(IsoSegment::new(bottom, right));
                }
                6 => segments.push(IsoSegment::new(bottom, top)),
                7 => segments.push(IsoSegment::new(bottom, left)),
                8 => segments.push(IsoSegment::new(left, bottom)),
                9 => segments.push(IsoSegment::new(top, bottom)),
                10 => {
                    segments.push(IsoSegment::new(right, top));
                    segments.push(IsoSegment::new(left, bottom));
                }
                11 => segments.push(IsoSegment::new(right, bottom)),
                12 => segments.push(IsoSegment::new(left, right)),
                13 => segments.push(IsoSegment::new(top, right)),
                14 => segments.push(IsoSegment::new(left, top)),
                _ => unreachable!("marching-squares cell index is four bits"),
            }
        }
    }

    segments.sort();
    segments.dedup();
    segments
}

fn padded_scalar_sample(
    field: &ScalarField,
    raster_options: RasterOptions,
    x: usize,
    y: usize,
) -> f64 {
    if x == 0 || y == 0 || x > field.width() || y > field.height() {
        if raster_options.invert {
            0.0
        } else {
            255.0
        }
    } else {
        f64::from(field.sample(x - 1, y - 1))
    }
}

fn scalar_is_foreground(sample: f64, threshold: f64, raster_options: RasterOptions) -> bool {
    apply_invert(sample < threshold, raster_options)
}

fn scalar_edge_point(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    sample0: f64,
    sample1: f64,
    threshold: f64,
) -> IsoPoint {
    let delta = sample1 - sample0;
    let amount = if delta.abs() <= f64::EPSILON {
        0.5
    } else {
        ((threshold - sample0) / delta).clamp(0.0, 1.0)
    };

    IsoPoint::new(x0 + (x1 - x0) * amount, y0 + (y1 - y0) * amount)
}

fn subpixel_segments(bitmap: &Bitmap) -> Vec<SubSegment> {
    let mut segments = Vec::new();

    for y in 0..=bitmap.height() {
        for x in 0..=bitmap.width() {
            let top_left = padded_black_sample(bitmap, x, y);
            let top_right = padded_black_sample(bitmap, x + 1, y);
            let bottom_right = padded_black_sample(bitmap, x + 1, y + 1);
            let bottom_left = padded_black_sample(bitmap, x, y + 1);
            let cell = (top_left as u8)
                | ((top_right as u8) << 1)
                | ((bottom_right as u8) << 2)
                | ((bottom_left as u8) << 3);

            let top = SubPoint {
                x2: (x as i32) * 2,
                y2: (y as i32) * 2 - 1,
            };
            let right = SubPoint {
                x2: (x as i32) * 2 + 1,
                y2: (y as i32) * 2,
            };
            let bottom = SubPoint {
                x2: (x as i32) * 2,
                y2: (y as i32) * 2 + 1,
            };
            let left = SubPoint {
                x2: (x as i32) * 2 - 1,
                y2: (y as i32) * 2,
            };

            match cell {
                0 | 15 => {}
                1 => segments.push(SubSegment::new(top, left)),
                2 => segments.push(SubSegment::new(right, top)),
                3 => segments.push(SubSegment::new(right, left)),
                4 => segments.push(SubSegment::new(bottom, right)),
                5 => {
                    segments.push(SubSegment::new(top, left));
                    segments.push(SubSegment::new(bottom, right));
                }
                6 => segments.push(SubSegment::new(bottom, top)),
                7 => segments.push(SubSegment::new(bottom, left)),
                8 => segments.push(SubSegment::new(left, bottom)),
                9 => segments.push(SubSegment::new(top, bottom)),
                10 => {
                    segments.push(SubSegment::new(right, top));
                    segments.push(SubSegment::new(left, bottom));
                }
                11 => segments.push(SubSegment::new(right, bottom)),
                12 => segments.push(SubSegment::new(left, right)),
                13 => segments.push(SubSegment::new(top, right)),
                14 => segments.push(SubSegment::new(left, top)),
                _ => unreachable!("marching-squares cell index is four bits"),
            }
        }
    }

    segments.sort();
    segments.dedup();
    segments
}

fn padded_black_sample(bitmap: &Bitmap, x: usize, y: usize) -> bool {
    if x == 0 || y == 0 || x > bitmap.width() || y > bitmap.height() {
        false
    } else {
        bitmap.is_black(x - 1, y - 1)
    }
}

fn trace_subpixel_loops(segments: &[SubSegment]) -> Vec<Vec<SubPoint>> {
    let mut outgoing: BTreeMap<SubPoint, Vec<SubPoint>> = BTreeMap::new();

    for segment in segments {
        outgoing.entry(segment.start).or_default().push(segment.end);
        outgoing.entry(segment.end).or_default().push(segment.start);
    }

    for neighbors in outgoing.values_mut() {
        neighbors.sort();
    }

    let mut visited = HashSet::new();
    let mut loops = Vec::new();

    for segment in segments {
        if visited.contains(segment) {
            continue;
        }

        if let Some(points) = trace_subpixel_loop(*segment, &outgoing, &mut visited) {
            loops.push(points);
        }
    }

    loops
}

fn trace_subpixel_loop(
    start_segment: SubSegment,
    outgoing: &BTreeMap<SubPoint, Vec<SubPoint>>,
    visited: &mut HashSet<SubSegment>,
) -> Option<Vec<SubPoint>> {
    let start = start_segment.start;
    let mut previous = start_segment.start;
    let mut current = start_segment.end;
    let mut points = vec![start, current];
    visited.insert(start_segment);

    while current != start {
        let next = outgoing.get(&current)?.iter().copied().find(|candidate| {
            *candidate != previous && !visited.contains(&SubSegment::new(current, *candidate))
        })?;

        visited.insert(SubSegment::new(current, next));
        previous = current;
        current = next;

        if current != start {
            points.push(current);
        }
    }

    Some(points)
}

fn trace_iso_loops(segments: &[IsoSegment]) -> Vec<Vec<IsoPoint>> {
    let mut outgoing: BTreeMap<IsoPoint, Vec<IsoPoint>> = BTreeMap::new();

    for segment in segments {
        outgoing.entry(segment.start).or_default().push(segment.end);
        outgoing.entry(segment.end).or_default().push(segment.start);
    }

    for neighbors in outgoing.values_mut() {
        neighbors.sort();
    }

    let mut visited = HashSet::new();
    let mut loops = Vec::new();

    for segment in segments {
        if visited.contains(segment) {
            continue;
        }

        if let Some(points) = trace_iso_loop(*segment, &outgoing, &mut visited) {
            loops.push(points);
        }
    }

    loops
}

fn trace_iso_loop(
    start_segment: IsoSegment,
    outgoing: &BTreeMap<IsoPoint, Vec<IsoPoint>>,
    visited: &mut HashSet<IsoSegment>,
) -> Option<Vec<IsoPoint>> {
    let start = start_segment.start;
    let mut previous = start_segment.start;
    let mut current = start_segment.end;
    let mut points = vec![start, current];
    visited.insert(start_segment);

    while current != start {
        let next = outgoing.get(&current)?.iter().copied().find(|candidate| {
            *candidate != previous && !visited.contains(&IsoSegment::new(current, *candidate))
        })?;

        visited.insert(IsoSegment::new(current, next));
        previous = current;
        current = next;

        if current != start {
            points.push(current);
        }
    }

    Some(points)
}

fn simplify_subpixel_collinear(points: &[SubPoint]) -> Vec<SubPoint> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut simplified = Vec::new();

    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];

        let ab = (current.x2 - previous.x2, current.y2 - previous.y2);
        let bc = (next.x2 - current.x2, next.y2 - current.y2);

        if ab.0 * bc.1 - ab.1 * bc.0 != 0 {
            simplified.push(current);
        }
    }

    simplified
}

fn rotate_subpixel_loop_to_top(points: Vec<SubPoint>) -> Vec<SubPoint> {
    let Some((start_index, _)) = points
        .iter()
        .enumerate()
        .min_by_key(|(_, point)| (point.y2, point.x2))
    else {
        return points;
    };

    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

fn rotate_iso_loop_to_top(points: Vec<IsoPoint>) -> Vec<IsoPoint> {
    let Some((start_index, _)) = points
        .iter()
        .enumerate()
        .min_by_key(|(_, point)| (point.y, point.x))
    else {
        return points;
    };

    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

fn optimize_subpixel_path(points: &[SubPoint], tolerance: f64) -> Vec<SubPoint> {
    if tolerance <= 0.0 || points.len() <= 3 {
        return points.to_vec();
    }

    let start_index = farthest_subpixel_pair_start_index(points);
    let mut open_points = rotate_subpixel_points(points, start_index);
    open_points.push(open_points[0]);

    let mut keep = vec![false; open_points.len()];
    keep[0] = true;
    keep[open_points.len() - 1] = true;

    mark_subpixel_rdp_points(&open_points, 0, open_points.len() - 1, tolerance, &mut keep);

    let mut optimized = open_points
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, point)| keep[index].then_some(point))
        .collect::<Vec<_>>();

    if optimized.len() > 1 && optimized.first() == optimized.last() {
        optimized.pop();
    }

    if optimized.len() < 3 {
        points.to_vec()
    } else {
        optimized
    }
}

fn optimize_iso_path(points: &[IsoPoint], tolerance: f64) -> Vec<IsoPoint> {
    if tolerance <= 0.0 || points.len() <= 3 {
        return points.to_vec();
    }

    let start_index = farthest_iso_pair_start_index(points);
    let mut open_points = rotate_iso_points(points, start_index);
    open_points.push(open_points[0]);

    let mut keep = vec![false; open_points.len()];
    keep[0] = true;
    keep[open_points.len() - 1] = true;

    mark_iso_rdp_points(&open_points, 0, open_points.len() - 1, tolerance, &mut keep);

    let mut optimized = open_points
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, point)| keep[index].then_some(point))
        .collect::<Vec<_>>();

    if optimized.len() > 1 && optimized.first() == optimized.last() {
        optimized.pop();
    }

    if optimized.len() < 3 {
        points.to_vec()
    } else {
        optimized
    }
}

fn farthest_subpixel_pair_start_index(points: &[SubPoint]) -> usize {
    let mut best = (0usize, 0i64);

    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let distance = subpixel_distance_squared(points[i], points[j]);

            if distance > best.1 {
                best = (i, distance);
            }
        }
    }

    best.0
}

fn farthest_iso_pair_start_index(points: &[IsoPoint]) -> usize {
    let mut best = (0usize, 0i128);

    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let distance = iso_distance_squared(points[i], points[j]);

            if distance > best.1 {
                best = (i, distance);
            }
        }
    }

    best.0
}

fn rotate_subpixel_points(points: &[SubPoint], start_index: usize) -> Vec<SubPoint> {
    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

fn rotate_iso_points(points: &[IsoPoint], start_index: usize) -> Vec<IsoPoint> {
    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

fn mark_subpixel_rdp_points(
    points: &[SubPoint],
    start_index: usize,
    end_index: usize,
    tolerance: f64,
    keep: &mut [bool],
) {
    if end_index <= start_index + 1 {
        return;
    }

    let mut farthest_index = start_index;
    let mut farthest_distance = 0.0;

    for index in (start_index + 1)..end_index {
        let distance =
            subpixel_perpendicular_distance(points[index], points[start_index], points[end_index]);

        if distance > farthest_distance {
            farthest_distance = distance;
            farthest_index = index;
        }
    }

    if farthest_distance > tolerance {
        keep[farthest_index] = true;
        mark_subpixel_rdp_points(points, start_index, farthest_index, tolerance, keep);
        mark_subpixel_rdp_points(points, farthest_index, end_index, tolerance, keep);
    }
}

fn mark_iso_rdp_points(
    points: &[IsoPoint],
    start_index: usize,
    end_index: usize,
    tolerance: f64,
    keep: &mut [bool],
) {
    if end_index <= start_index + 1 {
        return;
    }

    let mut farthest_index = start_index;
    let mut farthest_distance = 0.0;

    for index in (start_index + 1)..end_index {
        let distance =
            iso_perpendicular_distance(points[index], points[start_index], points[end_index]);

        if distance > farthest_distance {
            farthest_distance = distance;
            farthest_index = index;
        }
    }

    if farthest_distance > tolerance {
        keep[farthest_index] = true;
        mark_iso_rdp_points(points, start_index, farthest_index, tolerance, keep);
        mark_iso_rdp_points(points, farthest_index, end_index, tolerance, keep);
    }
}

fn subpixel_perpendicular_distance(
    point: SubPoint,
    line_start: SubPoint,
    line_end: SubPoint,
) -> f64 {
    let dx = f64::from(line_end.x2 - line_start.x2);
    let dy = f64::from(line_end.y2 - line_start.y2);

    if dx == 0.0 && dy == 0.0 {
        return f64::from(point.x2 - line_start.x2).hypot(f64::from(point.y2 - line_start.y2))
            / 2.0;
    }

    let numerator =
        (dy * f64::from(point.x2 - line_start.x2) - dx * f64::from(point.y2 - line_start.y2)).abs();
    let denominator = dx.hypot(dy);

    numerator / denominator / 2.0
}

fn iso_perpendicular_distance(point: IsoPoint, line_start: IsoPoint, line_end: IsoPoint) -> f64 {
    let point = point.to_float();
    let line_start = line_start.to_float();
    let line_end = line_end.to_float();

    let dx = line_end.0 - line_start.0;
    let dy = line_end.1 - line_start.1;

    if dx == 0.0 && dy == 0.0 {
        return (point.0 - line_start.0).hypot(point.1 - line_start.1);
    }

    let numerator = (dy * (point.0 - line_start.0) - dx * (point.1 - line_start.1)).abs();
    let denominator = dx.hypot(dy);

    numerator / denominator
}

fn subpixel_distance_squared(a: SubPoint, b: SubPoint) -> i64 {
    let dx = i64::from(a.x2 - b.x2);
    let dy = i64::from(a.y2 - b.y2);

    dx * dx + dy * dy
}

fn iso_distance_squared(a: IsoPoint, b: IsoPoint) -> i128 {
    let dx = i128::from(a.x - b.x);
    let dy = i128::from(a.y - b.y);

    dx * dx + dy * dy
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: usize, y: usize) -> Self {
        Self {
            x: x as i32,
            y: y as i32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Edge {
    start: Point,
    end: Point,
}

impl Edge {
    fn direction(self) -> (i32, i32) {
        (
            (self.end.x - self.start.x).signum(),
            (self.end.y - self.start.y).signum(),
        )
    }
}

fn boundary_edges(bitmap: &Bitmap) -> Vec<Edge> {
    let mut edges = Vec::new();

    for y in 0..bitmap.height() {
        for x in 0..bitmap.width() {
            if !bitmap.is_black(x, y) {
                continue;
            }

            if y == 0 || !bitmap.is_black(x, y - 1) {
                edges.push(Edge {
                    start: Point::new(x, y),
                    end: Point::new(x + 1, y),
                });
            }

            if x + 1 == bitmap.width() || !bitmap.is_black(x + 1, y) {
                edges.push(Edge {
                    start: Point::new(x + 1, y),
                    end: Point::new(x + 1, y + 1),
                });
            }

            if y + 1 == bitmap.height() || !bitmap.is_black(x, y + 1) {
                edges.push(Edge {
                    start: Point::new(x + 1, y + 1),
                    end: Point::new(x, y + 1),
                });
            }

            if x == 0 || !bitmap.is_black(x - 1, y) {
                edges.push(Edge {
                    start: Point::new(x, y + 1),
                    end: Point::new(x, y),
                });
            }
        }
    }

    edges.sort();
    edges
}

fn trace_path(
    start_edge: Edge,
    outgoing: &BTreeMap<Point, Vec<Edge>>,
    visited: &mut HashSet<Edge>,
) -> Option<Vec<Point>> {
    let mut current = start_edge;
    let mut points = vec![start_edge.start];

    loop {
        if !visited.insert(current) {
            return None;
        }

        points.push(current.end);

        if current.end == start_edge.start {
            points.pop();
            return Some(points);
        }

        current = choose_next_edge(current, outgoing.get(&current.end)?, visited)?;
    }
}

fn choose_next_edge(current: Edge, candidates: &[Edge], visited: &HashSet<Edge>) -> Option<Edge> {
    let current_direction = current.direction();
    let preferred = [
        right_turn(current_direction),
        current_direction,
        left_turn(current_direction),
        (-current_direction.0, -current_direction.1),
    ];

    preferred.into_iter().find_map(|direction| {
        candidates
            .iter()
            .copied()
            .find(|edge| !visited.contains(edge) && edge.direction() == direction)
    })
}

fn right_turn((dx, dy): (i32, i32)) -> (i32, i32) {
    (-dy, dx)
}

fn left_turn((dx, dy): (i32, i32)) -> (i32, i32) {
    (dy, -dx)
}

fn simplify_collinear(points: &[Point]) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut simplified = Vec::new();

    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];

        let ab = (current.x - previous.x, current.y - previous.y);
        let bc = (next.x - current.x, next.y - current.y);

        if ab.0 * bc.1 - ab.1 * bc.0 != 0 {
            simplified.push(current);
        }
    }

    simplified
}

fn optimize_path(points: &[Point], tolerance: f64) -> Vec<Point> {
    if tolerance <= 0.0 || points.len() <= 3 {
        return points.to_vec();
    }

    let start_index = farthest_pair_start_index(points);
    let mut open_points = rotate_closed_points(points, start_index);
    open_points.push(open_points[0]);

    let mut keep = vec![false; open_points.len()];
    keep[0] = true;
    keep[open_points.len() - 1] = true;

    mark_rdp_points(&open_points, 0, open_points.len() - 1, tolerance, &mut keep);

    let mut optimized = open_points
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, point)| keep[index].then_some(point))
        .collect::<Vec<_>>();

    if optimized.len() > 1 && optimized.first() == optimized.last() {
        optimized.pop();
    }

    if optimized.len() < 3 {
        points.to_vec()
    } else {
        optimized
    }
}

fn farthest_pair_start_index(points: &[Point]) -> usize {
    let mut best = (0usize, 0i64);

    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let distance = distance_squared(points[i], points[j]);

            if distance > best.1 {
                best = (i, distance);
            }
        }
    }

    best.0
}

fn rotate_closed_points(points: &[Point], start_index: usize) -> Vec<Point> {
    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

fn mark_rdp_points(
    points: &[Point],
    start_index: usize,
    end_index: usize,
    tolerance: f64,
    keep: &mut [bool],
) {
    if end_index <= start_index + 1 {
        return;
    }

    let mut farthest_index = start_index;
    let mut farthest_distance = 0.0;

    for index in (start_index + 1)..end_index {
        let distance =
            perpendicular_distance(points[index], points[start_index], points[end_index]);

        if distance > farthest_distance {
            farthest_distance = distance;
            farthest_index = index;
        }
    }

    if farthest_distance > tolerance {
        keep[farthest_index] = true;
        mark_rdp_points(points, start_index, farthest_index, tolerance, keep);
        mark_rdp_points(points, farthest_index, end_index, tolerance, keep);
    }
}

fn perpendicular_distance(point: Point, line_start: Point, line_end: Point) -> f64 {
    let dx = f64::from(line_end.x - line_start.x);
    let dy = f64::from(line_end.y - line_start.y);

    if dx == 0.0 && dy == 0.0 {
        return f64::from(point.x - line_start.x).hypot(f64::from(point.y - line_start.y));
    }

    let numerator =
        (dy * f64::from(point.x - line_start.x) - dx * f64::from(point.y - line_start.y)).abs();
    let denominator = dx.hypot(dy);

    numerator / denominator
}

fn distance_squared(a: Point, b: Point) -> i64 {
    let dx = i64::from(a.x - b.x);
    let dy = i64::from(a.y - b.y);

    dx * dx + dy * dy
}

fn signed_area2(points: &[Point]) -> i64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| i64::from(a.x) * i64::from(b.y) - i64::from(b.x) * i64::from(a.y))
        .sum()
}

fn signed_subpixel_area(points: &[SubPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| {
            let a = a.to_float();
            let b = b.to_float();
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

fn signed_iso_area(points: &[IsoPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| {
            let a = a.to_float();
            let b = b.to_float();
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

fn is_below_turd_size(area2: i64, turd_size: usize) -> bool {
    if turd_size == 0 {
        return false;
    }

    area2.unsigned_abs() <= (turd_size as u64).saturating_mul(2)
}

fn is_below_turd_size_float(area: f64, turd_size: usize) -> bool {
    turd_size != 0 && area.abs() <= turd_size as f64
}

fn path_to_svg_data(
    path: &TracePath,
    options: SvgRenderOptions,
    canvas_size: Option<(usize, usize)>,
) -> Option<String> {
    match options.curve_mode {
        CurveMode::Polygon => path_to_polygon_svg_data(path),
        CurveMode::Smooth => path_to_smooth_svg_data(path),
        CurveMode::Spline => path_to_spline_svg_data(path),
        CurveMode::Fit => path_to_fit_svg_data(path),
        CurveMode::Potrace => path_to_potrace_svg_data(
            path,
            options.opt_tolerance.max(0.0),
            options.pixel_potrace,
            canvas_size,
        ),
    }
}

fn path_to_polygon_svg_data(path: &TracePath) -> Option<String> {
    let (first, rest) = path.points.split_first()?;
    let mut data = format!("M {} {}", format_float(first.0), format_float(first.1));

    for point in rest {
        data.push_str(&format!(
            " L {} {}",
            format_float(point.0),
            format_float(point.1)
        ));
    }

    data.push_str(" Z");
    Some(data)
}

fn path_to_smooth_svg_data(path: &TracePath) -> Option<String> {
    const CORNER_SMOOTHING: f64 = 0.25;

    if path.points.len() < 3 {
        return path_to_polygon_svg_data(path);
    }

    let first_exit = corner_exit(&path.points, 0, CORNER_SMOOTHING);
    let mut data = format!(
        "M {} {}",
        format_float(first_exit.0),
        format_float(first_exit.1)
    );

    for offset in 1..=path.points.len() {
        let vertex_index = offset % path.points.len();
        let vertex = path.points[vertex_index];
        let entry = corner_entry(&path.points, vertex_index, CORNER_SMOOTHING);
        let exit = corner_exit(&path.points, vertex_index, CORNER_SMOOTHING);
        let control1 = cubic_control_point(entry, vertex);
        let control2 = cubic_control_point(exit, vertex);

        data.push_str(&format!(
            " L {} {} C {} {}, {} {}, {} {}",
            format_float(entry.0),
            format_float(entry.1),
            format_float(control1.0),
            format_float(control1.1),
            format_float(control2.0),
            format_float(control2.1),
            format_float(exit.0),
            format_float(exit.1)
        ));
    }

    data.push_str(" Z");
    Some(data)
}

fn path_to_spline_svg_data(path: &TracePath) -> Option<String> {
    if path.points.len() < 3 {
        return path_to_polygon_svg_data(path);
    }

    let mut data = format!(
        "M {} {}",
        format_float(path.points[0].0),
        format_float(path.points[0].1)
    );

    for index in 0..path.points.len() {
        let (control1, control2, next) = catmull_rom_segment(path, index);

        data.push_str(&format!(
            " C {} {}, {} {}, {} {}",
            format_float(control1.0),
            format_float(control1.1),
            format_float(control2.0),
            format_float(control2.1),
            format_float(next.0),
            format_float(next.1)
        ));
    }

    data.push_str(" Z");
    Some(data)
}

fn path_to_fit_svg_data(path: &TracePath) -> Option<String> {
    const FIT_ERROR: f64 = 0.75;

    if path.points.len() < 3 {
        return path_to_polygon_svg_data(path);
    }

    let bounds = FloatBounds::from_points(&path.points)?;
    let segments = fit_closed_cubic_segments(&path.points, FIT_ERROR);
    let first = segments.first()?;
    let mut data = format!(
        "M {} {}",
        format_float(first.start.0),
        format_float(first.start.1)
    );

    for segment in segments {
        let control1 = bounds.clamp(segment.control1);
        let control2 = bounds.clamp(segment.control2);

        data.push_str(&format!(
            " C {} {}, {} {}, {} {}",
            format_float(control1.0),
            format_float(control1.1),
            format_float(control2.0),
            format_float(control2.1),
            format_float(segment.end.0),
            format_float(segment.end.1)
        ));
    }

    data.push_str(" Z");
    Some(data)
}

fn path_to_potrace_svg_data(
    path: &TracePath,
    opt_tolerance: f64,
    pixel_potrace: bool,
    canvas_size: Option<(usize, usize)>,
) -> Option<String> {
    if path.points.len() < 3 {
        return path_to_polygon_svg_data(path);
    }

    if pixel_potrace {
        let (start, segments) = choose_pixel_potrace_point_set(path, opt_tolerance, canvas_size)?;
        return Some(compact_svg_path_data_from_segments(start, &segments));
    }

    let polygon = legacy_potrace_polygon_indices(&path.points);
    let vertices = adjust_potrace_vertices(&path.points, &polygon, 1.0);
    let (mut start, mut segments) = smooth_potrace_vertices(&vertices)?;

    if segments
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
        && path.points.len() >= 12
        && !points_are_half_pixel_quantized(&path.points)
    {
        let fitted = fit_closed_smooth_potrace_segments(&path.points, false);
        if let Some(first) = fitted.first() {
            start = first.start();
            segments = fitted;
        }
    }

    let (start, segments) = optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        STRICT_POTRACE_LINEAR_DEVIATION,
    );
    Some(svg_path_data_from_segments(start, &segments))
}

fn choose_pixel_potrace_point_set(
    path: &TracePath,
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let mut best =
        pixel_potrace_segments_for_points(path, &path.points, opt_tolerance, canvas_size)?;
    let simplified = simplify_collinear_float_points(&path.points);

    if simplified.len() >= 3 && simplified.len() < path.points.len() {
        if let Some(candidate) =
            pixel_potrace_segments_for_points(path, &simplified, opt_tolerance, canvas_size)
        {
            if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                best = candidate;
            }
        }
    }

    Some(best)
}

fn pixel_potrace_segments_for_points(
    reference_path: &TracePath,
    points: &[(f64, f64)],
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    let polygon = optimal_potrace_polygon_indices(points);
    let vertices = adjust_potrace_vertices(points, &polygon, 0.5);
    let (start, segments) = smooth_potrace_vertices(&vertices)?;

    Some(choose_pixel_potrace_segments(
        reference_path,
        start,
        segments,
        opt_tolerance,
        canvas_size,
    ))
}

fn simplify_collinear_float_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    const EPSILON: f64 = 1.0e-9;

    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut simplified = Vec::new();
    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        let incoming = subtract(current, previous);
        let outgoing = subtract(next, current);

        if cross(incoming, outgoing).abs() > EPSILON {
            simplified.push(current);
        }
    }

    simplified
}

fn choose_pixel_potrace_segments(
    path: &TracePath,
    start: (f64, f64),
    segments: Vec<SvgPathSegment>,
    opt_tolerance: f64,
    canvas_size: Option<(usize, usize)>,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let mut best = optimize_potrace_segments(
        start,
        &segments,
        opt_tolerance,
        PIXEL_POTRACE_LINEAR_DEVIATION,
    );

    if segments
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
        && path.points.len() >= 12
    {
        let fitted = fit_closed_smooth_potrace_segments(&path.points, true);
        if let Some(first) = fitted.first() {
            let candidate = optimize_potrace_segments(
                first.start(),
                &fitted,
                opt_tolerance,
                PIXEL_POTRACE_LINEAR_DEVIATION,
            );
            if pixel_potrace_candidate_is_better(path, canvas_size, &candidate, &best) {
                best = candidate;
            }
        }
    }

    best
}

fn pixel_potrace_candidate_is_better(
    path: &TracePath,
    canvas_size: Option<(usize, usize)>,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    best: &((f64, f64), Vec<SvgPathSegment>),
) -> bool {
    if let Some((width, height)) = canvas_size {
        let candidate_error = pixel_potrace_candidate_mask_error(path, candidate, width, height);
        let best_error = pixel_potrace_candidate_mask_error(path, best, width, height);

        return candidate_error < best_error
            || (candidate_error == best_error
                && compact_svg_path_data_from_segments(candidate.0, &candidate.1).len()
                    < compact_svg_path_data_from_segments(best.0, &best.1).len());
    }

    compact_svg_path_data_from_segments(candidate.0, &candidate.1).len()
        < compact_svg_path_data_from_segments(best.0, &best.1).len()
}

fn pixel_potrace_candidate_mask_error(
    path: &TracePath,
    candidate: &((f64, f64), Vec<SvgPathSegment>),
    width: usize,
    height: usize,
) -> usize {
    let mut reference = vec![false; width.saturating_mul(height)];
    let mut candidate_pixels = vec![false; width.saturating_mul(height)];
    rasterize_path_evenodd(path, width, height, &mut reference);

    let candidate_path = TracePath {
        is_hole: path.is_hole,
        points: flattened_potrace_segments(candidate.0, &candidate.1),
    };
    rasterize_path_evenodd(&candidate_path, width, height, &mut candidate_pixels);

    reference
        .iter()
        .zip(candidate_pixels.iter())
        .filter(|(left, right)| left != right)
        .count()
}

fn flattened_potrace_segments(start: (f64, f64), segments: &[SvgPathSegment]) -> Vec<(f64, f64)> {
    const CUBIC_FLATTEN_STEPS: usize = 64;

    let mut points = Vec::new();
    points.push(start);

    for segment in segments {
        match segment {
            SvgPathSegment::Line { end, .. } => points.push(*end),
            SvgPathSegment::Cubic(cubic) => {
                for step in 1..=CUBIC_FLATTEN_STEPS {
                    points.push(cubic_point(
                        *cubic,
                        step as f64 / CUBIC_FLATTEN_STEPS as f64,
                    ));
                }
            }
        }
    }

    dedup_nearby_points(points)
}

fn optimal_potrace_polygon_indices(points: &[(f64, f64)]) -> Vec<usize> {
    if points.len() > 3 && distance_squared_float(points[0], points[points.len() - 1]) <= 1.0e-12 {
        return optimal_potrace_polygon_indices(&points[..points.len() - 1]);
    }

    if points.len() <= 8 {
        return (0..points.len()).collect();
    }

    if !points_are_half_pixel_quantized(points) {
        return legacy_potrace_polygon_indices(points);
    }

    let mut best: Option<PolygonCandidate> = None;
    for rotation in polygon_rotation_candidates(points) {
        let rotated = rotate_float_points(points, rotation);
        let Some(candidate) = best_polygon_for_rotated_points(&rotated) else {
            continue;
        };
        let indices = candidate
            .indices
            .iter()
            .map(|index| (index + rotation) % points.len())
            .collect::<Vec<_>>();
        let candidate = PolygonCandidate {
            indices,
            segments: candidate.segments,
            penalty: candidate.penalty,
        };

        if best
            .as_ref()
            .is_none_or(|current| polygon_candidate_is_better(&candidate, current))
        {
            best = Some(candidate);
        }
    }

    best.map(|candidate| candidate.indices)
        .filter(|indices| indices.len() >= 3)
        .unwrap_or_else(|| (0..points.len()).collect())
}

fn legacy_potrace_polygon_indices(points: &[(f64, f64)]) -> Vec<usize> {
    const POLYGON_TOLERANCE: f64 = 0.75;

    let n = points.len();
    let mut dp: Vec<Option<PolygonDpState>> = vec![None; n + 1];
    dp[0] = Some(PolygonDpState {
        previous: 0,
        segments: 0,
        penalty: 0.0,
    });

    for start in 0..n {
        let Some(state) = dp[start] else {
            continue;
        };

        let mut end = start + 1;
        while end <= n {
            if end == n && state.segments < 2 {
                end += 1;
                continue;
            }

            if !legacy_potrace_arc_is_straight(points, start, end, POLYGON_TOLERANCE) {
                if end == start + 1 {
                    end += 1;
                    continue;
                }
                break;
            }

            let penalty =
                state.penalty + legacy_potrace_polygon_segment_penalty(points, start, end);
            let candidate = PolygonDpState {
                previous: start,
                segments: state.segments + 1,
                penalty,
            };

            if dp[end].is_none_or(|best| polygon_dp_state_is_better(candidate, best)) {
                dp[end] = Some(candidate);
            }

            end += 1;
        }
    }

    let Some(_) = dp[n] else {
        return (0..points.len()).collect();
    };

    let mut indices = Vec::new();
    let mut cursor = n;

    while cursor != 0 {
        let state = dp[cursor].expect("legacy dp cursor should be reachable");
        indices.push(state.previous % n);
        cursor = state.previous;
    }

    indices.reverse();
    indices.dedup();

    if indices.len() < 3 {
        (0..points.len()).collect()
    } else {
        indices
    }
}

fn legacy_potrace_arc_is_straight(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
    tolerance: f64,
) -> bool {
    if end <= start + 1 {
        return true;
    }

    let start_point = closed_point(points, start);
    let end_point = closed_point(points, end);
    let tolerance_squared = tolerance * tolerance;

    for index in start + 1..end {
        let point = closed_point(points, index);
        if distance_squared_to_segment(point, start_point, end_point).0 > tolerance_squared {
            return false;
        }
    }

    true
}

fn legacy_potrace_polygon_segment_penalty(points: &[(f64, f64)], start: usize, end: usize) -> f64 {
    if end <= start + 1 {
        return 0.0;
    }

    let start_point = closed_point(points, start);
    let end_point = closed_point(points, end);

    (start + 1..end)
        .map(|index| {
            distance_squared_to_segment(closed_point(points, index), start_point, end_point).0
        })
        .sum()
}

#[derive(Debug, Clone)]
struct PolygonCandidate {
    indices: Vec<usize>,
    segments: usize,
    penalty: f64,
}

fn polygon_candidate_is_better(candidate: &PolygonCandidate, best: &PolygonCandidate) -> bool {
    candidate.segments < best.segments
        || (candidate.segments == best.segments && candidate.penalty < best.penalty)
}

fn polygon_rotation_candidates(points: &[(f64, f64)]) -> Vec<usize> {
    const MAX_ROTATIONS: usize = 24;

    if points.len() <= MAX_ROTATIONS {
        return (0..points.len()).collect();
    }

    let mut scored = (0..points.len())
        .map(|index| {
            let previous = points[(index + points.len() - 1) % points.len()];
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            let turn = vector_turn_angle(subtract(current, previous), subtract(next, current));
            (index, turn)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut candidates = Vec::new();
    candidates.push(0);
    for (index, turn) in scored {
        if turn <= 1.0e-6 {
            continue;
        }

        if !candidates.contains(&index) {
            candidates.push(index);
        }

        if candidates.len() >= MAX_ROTATIONS {
            break;
        }
    }

    let stride = (points.len() / MAX_ROTATIONS).max(1);
    for index in (0..points.len()).step_by(stride) {
        if candidates.len() >= MAX_ROTATIONS {
            break;
        }
        if !candidates.contains(&index) {
            candidates.push(index);
        }
    }

    candidates
}

fn rotate_float_points(points: &[(f64, f64)], start_index: usize) -> Vec<(f64, f64)> {
    points[start_index..]
        .iter()
        .chain(points[..start_index].iter())
        .copied()
        .collect()
}

fn best_polygon_for_rotated_points(points: &[(f64, f64)]) -> Option<PolygonCandidate> {
    let n = points.len();
    let sums = PathSums::for_closed_points(points);
    let mut dp: Vec<Option<PolygonDpState>> = vec![None; n + 1];
    dp[0] = Some(PolygonDpState {
        previous: 0,
        segments: 0,
        penalty: 0.0,
    });

    for start in 0..n {
        let Some(state) = dp[start] else {
            continue;
        };

        let mut end = start + 1;
        while end <= n {
            if end - start > n.saturating_sub(3) {
                break;
            }

            if !potrace_possible_segment_is_straight(points, start, end) {
                if end == start + 1 {
                    end += 1;
                    continue;
                }
                break;
            }

            let penalty =
                state.penalty + potrace_polygon_segment_penalty(points, &sums, start, end);
            let candidate = PolygonDpState {
                previous: start,
                segments: state.segments + 1,
                penalty,
            };

            if dp[end].is_none_or(|best| polygon_dp_state_is_better(candidate, best)) {
                dp[end] = Some(candidate);
            }

            end += 1;
        }
    }

    let final_state = dp[n]?;

    let mut indices = Vec::new();
    let mut cursor = n;

    while cursor != 0 {
        let state = dp[cursor].expect("dp cursor should be reachable");
        indices.push(state.previous % n);
        cursor = state.previous;
    }

    indices.reverse();
    indices.dedup();

    if indices.len() < 3 {
        None
    } else {
        Some(PolygonCandidate {
            indices,
            segments: final_state.segments,
            penalty: final_state.penalty,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct PolygonDpState {
    previous: usize,
    segments: usize,
    penalty: f64,
}

fn polygon_dp_state_is_better(candidate: PolygonDpState, best: PolygonDpState) -> bool {
    candidate.segments < best.segments
        || (candidate.segments == best.segments && candidate.penalty < best.penalty)
}

#[derive(Debug, Clone)]
struct PathSums {
    x: Vec<f64>,
    y: Vec<f64>,
    x2: Vec<f64>,
    xy: Vec<f64>,
    y2: Vec<f64>,
}

impl PathSums {
    fn for_closed_points(points: &[(f64, f64)]) -> Self {
        let count = points.len() * 2 + 1;
        let mut sums = Self {
            x: Vec::with_capacity(count + 1),
            y: Vec::with_capacity(count + 1),
            x2: Vec::with_capacity(count + 1),
            xy: Vec::with_capacity(count + 1),
            y2: Vec::with_capacity(count + 1),
        };
        sums.x.push(0.0);
        sums.y.push(0.0);
        sums.x2.push(0.0);
        sums.xy.push(0.0);
        sums.y2.push(0.0);

        for index in 0..count {
            let point = points[index % points.len()];
            sums.x.push(sums.x[index] + point.0);
            sums.y.push(sums.y[index] + point.1);
            sums.x2.push(sums.x2[index] + point.0 * point.0);
            sums.xy.push(sums.xy[index] + point.0 * point.1);
            sums.y2.push(sums.y2[index] + point.1 * point.1);
        }

        sums
    }

    fn range(&self, start: usize, end: usize) -> PathSumRange {
        let end = end + 1;
        PathSumRange {
            count: (end - start) as f64,
            x: self.x[end] - self.x[start],
            y: self.y[end] - self.y[start],
            x2: self.x2[end] - self.x2[start],
            xy: self.xy[end] - self.xy[start],
            y2: self.y2[end] - self.y2[start],
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PathSumRange {
    count: f64,
    x: f64,
    y: f64,
    x2: f64,
    xy: f64,
    y2: f64,
}

fn potrace_possible_segment_is_straight(points: &[(f64, f64)], start: usize, end: usize) -> bool {
    if end <= start + 1 {
        return true;
    }

    if end - start > points.len().saturating_sub(3) {
        return false;
    }

    potrace_subpath_is_straight(points, start as isize - 1, end as isize + 1)
}

fn potrace_subpath_is_straight(points: &[(f64, f64)], start: isize, end: isize) -> bool {
    const MAX_DISTANCE: f64 = 1.0;

    if end <= start + 2 {
        return true;
    }

    if potrace_subpath_uses_all_four_directions(points, start, end) {
        return false;
    }

    let start_point = cyclic_point(points, start);
    let end_point = cyclic_point(points, end);
    if distance_squared_float(start_point, end_point) <= f64::EPSILON {
        return false;
    }

    for index in (start + 1)..end {
        let point = cyclic_point(points, index);
        if max_distance_to_infinite_line(point, start_point, end_point) > MAX_DISTANCE {
            return false;
        }
    }

    true
}

fn potrace_subpath_uses_all_four_directions(
    points: &[(f64, f64)],
    start: isize,
    end: isize,
) -> bool {
    let mut mask = 0u8;

    for index in start..end {
        let from = cyclic_point(points, index);
        let to = cyclic_point(points, index + 1);
        mask |= cardinal_direction_mask(subtract(to, from));
        if mask == 0b1111 {
            return true;
        }
    }

    false
}

fn cardinal_direction_mask(vector: (f64, f64)) -> u8 {
    if vector.0.abs() <= f64::EPSILON && vector.1.abs() <= f64::EPSILON {
        return 0;
    }

    if vector.0.abs() >= vector.1.abs() {
        if vector.0 >= 0.0 {
            0b0001
        } else {
            0b0010
        }
    } else if vector.1 >= 0.0 {
        0b0100
    } else {
        0b1000
    }
}

fn max_distance_to_infinite_line(
    point: (f64, f64),
    line_start: (f64, f64),
    line_end: (f64, f64),
) -> f64 {
    let line = subtract(line_end, line_start);
    let length_squared = vector_length_squared(line);

    if length_squared <= f64::EPSILON {
        return (point.0 - line_start.0)
            .abs()
            .max((point.1 - line_start.1).abs());
    }

    let amount = dot(subtract(point, line_start), line) / length_squared;
    let projection = add(line_start, scale(line, amount));
    (point.0 - projection.0)
        .abs()
        .max((point.1 - projection.1).abs())
}

fn cyclic_point(points: &[(f64, f64)], index: isize) -> (f64, f64) {
    let len = points.len() as isize;
    let index = index.rem_euclid(len) as usize;
    points[index]
}

fn potrace_polygon_segment_penalty(
    points: &[(f64, f64)],
    sums: &PathSums,
    start: usize,
    end: usize,
) -> f64 {
    if end <= start + 1 {
        return 0.0;
    }

    let start_point = closed_point(points, start);
    let end_point = closed_point(points, end);
    let chord = subtract(end_point, start_point);
    let range = sums.range(start, end);
    let a = -chord.1;
    let b = chord.0;
    let c = chord.1 * start_point.0 - chord.0 * start_point.1;
    let squared_error = a * a * range.x2
        + 2.0 * a * b * range.xy
        + b * b * range.y2
        + 2.0 * a * c * range.x
        + 2.0 * b * c * range.y
        + range.count * c * c;

    (squared_error.max(0.0) / range.count).sqrt()
}

fn adjust_potrace_vertices(
    points: &[(f64, f64)],
    polygon: &[usize],
    max_vertex_adjustment: f64,
) -> Vec<(f64, f64)> {
    if polygon.len() < 3 {
        return polygon.iter().map(|index| points[*index]).collect();
    }

    let mut adjusted = Vec::with_capacity(polygon.len());

    for index in 0..polygon.len() {
        let previous = polygon[(index + polygon.len() - 1) % polygon.len()];
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        let incoming = best_fit_line_for_closed_arc(points, previous, current);
        let outgoing = best_fit_line_for_closed_arc(points, current, next);
        let vertex = line_intersection(incoming, outgoing)
            .map(|point| clamp_point_to_box(point, points[current], max_vertex_adjustment))
            .unwrap_or(points[current]);

        adjusted.push(vertex);
    }

    adjusted
}

#[derive(Debug, Clone, Copy)]
struct FitLine {
    point: (f64, f64),
    direction: (f64, f64),
}

fn best_fit_line_for_closed_arc(points: &[(f64, f64)], start: usize, end: usize) -> FitLine {
    let arc = closed_arc_points_by_index(points, start, end);

    if arc.len() <= 2 {
        return FitLine {
            point: arc[0],
            direction: unit_vector(subtract(*arc.last().unwrap_or(&arc[0]), arc[0])),
        };
    }

    let centroid = arc_centroid(&arc);
    let mut xx = 0.0;
    let mut xy = 0.0;
    let mut yy = 0.0;

    for point in &arc {
        let centered = subtract(*point, centroid);
        xx += centered.0 * centered.0;
        xy += centered.0 * centered.1;
        yy += centered.1 * centered.1;
    }

    let fallback = unit_vector(subtract(*arc.last().unwrap_or(&arc[0]), arc[0]));
    let direction = principal_axis_2x2(xx, xy, yy).unwrap_or(fallback);

    FitLine {
        point: centroid,
        direction,
    }
}

fn closed_arc_points_by_index(points: &[(f64, f64)], start: usize, end: usize) -> Vec<(f64, f64)> {
    let mut arc = Vec::new();
    let mut index = start;

    loop {
        arc.push(points[index]);

        if index == end {
            break;
        }

        index = (index + 1) % points.len();
    }

    arc
}

fn arc_centroid(points: &[(f64, f64)]) -> (f64, f64) {
    let sum = points.iter().copied().fold((0.0, 0.0), add);

    scale(sum, 1.0 / points.len() as f64)
}

fn largest_eigenvalue_2x2(xx: f64, xy: f64, yy: f64) -> f64 {
    let trace = xx + yy;
    let determinant = xx * yy - xy * xy;
    let discriminant = (trace * trace - 4.0 * determinant).max(0.0).sqrt();

    (trace + discriminant) / 2.0
}

fn principal_axis_2x2(xx: f64, xy: f64, yy: f64) -> Option<(f64, f64)> {
    if xx.abs() <= f64::EPSILON && xy.abs() <= f64::EPSILON && yy.abs() <= f64::EPSILON {
        return None;
    }

    let lambda = largest_eigenvalue_2x2(xx, xy, yy);
    let candidates = [(xy, lambda - xx), (lambda - yy, xy)];

    candidates
        .into_iter()
        .find(|candidate| vector_length_squared(*candidate) > f64::EPSILON)
        .map(unit_vector)
        .or({
            if xx >= yy {
                Some((1.0, 0.0))
            } else {
                Some((0.0, 1.0))
            }
        })
}

fn line_intersection(a: FitLine, b: FitLine) -> Option<(f64, f64)> {
    let denominator = cross(a.direction, b.direction);

    if denominator.abs() <= 1.0e-9 {
        return None;
    }

    let amount = cross(subtract(b.point, a.point), b.direction) / denominator;
    Some(add(a.point, scale(a.direction, amount)))
}

fn clamp_point_to_box(point: (f64, f64), center: (f64, f64), radius: f64) -> (f64, f64) {
    (
        point.0.clamp(center.0 - radius, center.0 + radius),
        point.1.clamp(center.1 - radius, center.1 + radius),
    )
}

fn smooth_potrace_vertices(points: &[(f64, f64)]) -> Option<((f64, f64), Vec<SvgPathSegment>)> {
    const ALPHA_MIN: f64 = 0.55;
    const ALPHA_MAX: f64 = 1.0;

    if points.len() < 3 {
        return None;
    }

    let first = edge_midpoint(points[points.len() - 1], points[0]);
    let mut segments = Vec::new();
    let mut start = first;

    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let vertex = points[index];
        let next = points[(index + 1) % points.len()];
        let entry = edge_midpoint(previous, vertex);
        let exit = edge_midpoint(vertex, next);
        let alpha = potrace_curve_alpha(previous, vertex, next);

        if alpha > ALPHA_MAX {
            segments.push(SvgPathSegment::Line { start, end: vertex });
            segments.push(SvgPathSegment::Line {
                start: vertex,
                end: exit,
            });
        } else {
            let alpha = alpha.clamp(ALPHA_MIN, ALPHA_MAX);
            segments.push(SvgPathSegment::Cubic(CubicSegment {
                start: entry,
                control1: interpolate(entry, vertex, alpha),
                control2: interpolate(exit, vertex, alpha),
                end: exit,
            }));
        }

        start = exit;
    }

    Some((first, segments))
}

fn closed_point(points: &[(f64, f64)], index: usize) -> (f64, f64) {
    points[index % points.len()]
}

#[derive(Debug, Clone, Copy)]
enum SvgPathSegment {
    Line { start: (f64, f64), end: (f64, f64) },
    Cubic(CubicSegment),
}

impl SvgPathSegment {
    fn start(self) -> (f64, f64) {
        match self {
            SvgPathSegment::Line { start, .. } => start,
            SvgPathSegment::Cubic(cubic) => cubic.start,
        }
    }

    fn end(self) -> (f64, f64) {
        match self {
            SvgPathSegment::Line { end, .. } => end,
            SvgPathSegment::Cubic(cubic) => cubic.end,
        }
    }
}

fn optimize_potrace_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
    opt_tolerance: f64,
    max_linear_deviation: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    if segments.len() < 3 {
        return (start, segments.to_vec());
    }

    if segments
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        let optimized = optimize_closed_potrace_curve_run(
            &segments
                .iter()
                .filter_map(|segment| match segment {
                    SvgPathSegment::Cubic(cubic) => Some(*cubic),
                    SvgPathSegment::Line { .. } => None,
                })
                .collect::<Vec<_>>(),
            opt_tolerance,
        );

        return finish_potrace_segments(start, optimized, opt_tolerance, max_linear_deviation);
    }

    let (start, optimized) = optimize_mixed_potrace_curve_runs_once(start, segments, opt_tolerance);
    finish_potrace_segments(start, optimized, opt_tolerance, max_linear_deviation)
}

fn finish_potrace_segments(
    start: (f64, f64),
    segments: Vec<SvgPathSegment>,
    opt_tolerance: f64,
    max_linear_deviation: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let optimized = cleanup_potrace_segments(segments, max_linear_deviation);
    if optimized
        .iter()
        .all(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
    {
        return (start, optimized);
    }

    let (start, optimized) =
        optimize_mixed_potrace_curve_runs_once(start, &optimized, opt_tolerance);
    (
        start,
        cleanup_potrace_segments(optimized, max_linear_deviation),
    )
}

fn optimize_mixed_potrace_curve_runs_once(
    start: (f64, f64),
    segments: &[SvgPathSegment],
    opt_tolerance: f64,
) -> ((f64, f64), Vec<SvgPathSegment>) {
    let rotated = rotate_potrace_segments_after_last_line(segments);
    let start = rotated.first().map_or(start, |segment| segment.start());
    let mut optimized = Vec::new();
    let mut curve_run = Vec::new();

    for segment in rotated {
        match segment {
            SvgPathSegment::Cubic(cubic) => curve_run.push(cubic),
            SvgPathSegment::Line { .. } => {
                flush_potrace_curve_run(&mut optimized, &mut curve_run, opt_tolerance);
                optimized.push(segment);
            }
        }
    }

    flush_potrace_curve_run(&mut optimized, &mut curve_run, opt_tolerance);
    (start, optimized)
}

fn cleanup_potrace_segments(
    segments: Vec<SvgPathSegment>,
    max_linear_deviation: f64,
) -> Vec<SvgPathSegment> {
    let optimized = prune_tiny_potrace_curve_segments(segments);
    let optimized = regularize_potrace_orthogonal_corners(optimized);
    let optimized = demote_nearly_linear_potrace_cubics(optimized, max_linear_deviation);
    merge_collinear_potrace_lines(optimized)
}

fn merge_collinear_potrace_lines(segments: Vec<SvgPathSegment>) -> Vec<SvgPathSegment> {
    if segments.len() < 2 {
        return segments;
    }

    let mut merged: Vec<SvgPathSegment> = Vec::with_capacity(segments.len());

    for segment in segments {
        if let Some(previous) = merged.last_mut() {
            if let Some(combined) = merge_collinear_potrace_line_pair(*previous, segment) {
                *previous = combined;
                continue;
            }
        }

        merged.push(segment);
    }

    merged
}

fn merge_collinear_potrace_line_pair(
    previous: SvgPathSegment,
    current: SvgPathSegment,
) -> Option<SvgPathSegment> {
    let (
        SvgPathSegment::Line { start, end: middle },
        SvgPathSegment::Line {
            start: current_start,
            end,
        },
    ) = (previous, current)
    else {
        return None;
    };

    if distance_squared_float(middle, current_start) > 1.0e-12 {
        return None;
    }

    let first = subtract(middle, start);
    let second = subtract(end, middle);
    if vector_length_squared(first) <= f64::EPSILON
        || vector_length_squared(second) <= f64::EPSILON
        || cross(first, second).abs() > 1.0e-9
        || dot(first, second) < 0.0
    {
        return None;
    }

    Some(SvgPathSegment::Line { start, end })
}

fn demote_nearly_linear_potrace_cubics(
    segments: Vec<SvgPathSegment>,
    max_linear_deviation: f64,
) -> Vec<SvgPathSegment> {
    segments
        .into_iter()
        .map(|segment| match segment {
            SvgPathSegment::Cubic(cubic)
                if potrace_cubic_is_nearly_linear(cubic, max_linear_deviation) =>
            {
                SvgPathSegment::Line {
                    start: cubic.start,
                    end: cubic.end,
                }
            }
            segment => segment,
        })
        .collect()
}

const STRICT_POTRACE_LINEAR_DEVIATION: f64 = 0.25;
const PIXEL_POTRACE_LINEAR_DEVIATION: f64 = 1.0;

fn potrace_cubic_is_nearly_linear(cubic: CubicSegment, max_linear_deviation: f64) -> bool {
    const MIN_LINEAR_LENGTH: f64 = 16.0;

    cubic_chord_length(cubic) >= MIN_LINEAR_LENGTH
        && cubic_chord_deviation(cubic) <= max_linear_deviation
}

fn prune_tiny_potrace_curve_segments(segments: Vec<SvgPathSegment>) -> Vec<SvgPathSegment> {
    if segments.len() < 5 {
        return segments;
    }

    let mut pruned = Vec::with_capacity(segments.len());
    for index in 0..segments.len() {
        if potrace_segment_is_tiny_spike(&segments, index) {
            continue;
        }

        pruned.push(segments[index]);
    }

    if pruned.len() >= 3 && pruned.len() < segments.len() {
        pruned
    } else {
        segments
    }
}

fn potrace_segment_is_tiny_spike(segments: &[SvgPathSegment], index: usize) -> bool {
    const TINY_CHORD_LENGTH: f64 = 2.1;
    const TINY_BOUNDS_DIAGONAL: f64 = 2.1;
    const MIN_NEIGHBOR_CHORD_LENGTH: f64 = 4.0;

    if index == 0 || index + 1 >= segments.len() {
        return false;
    }

    let (
        SvgPathSegment::Cubic(previous),
        SvgPathSegment::Cubic(current),
        SvgPathSegment::Cubic(next),
    ) = (segments[index - 1], segments[index], segments[index + 1])
    else {
        return false;
    };

    cubic_chord_length(current) <= TINY_CHORD_LENGTH
        && cubic_bounds_diagonal(current) <= TINY_BOUNDS_DIAGONAL
        && cubic_chord_length(previous) >= MIN_NEIGHBOR_CHORD_LENGTH
        && cubic_chord_length(next) >= MIN_NEIGHBOR_CHORD_LENGTH
        && potrace_segment_has_spike_turn(previous, current, next)
}

fn potrace_segment_has_spike_turn(
    previous: CubicSegment,
    current: CubicSegment,
    next: CubicSegment,
) -> bool {
    const MIN_SPIKE_TURN_RADIANS: f64 = 1.0;
    const MIN_BRIDGED_TURN_RADIANS: f64 = 0.35;

    let previous_vector = cubic_chord_vector(previous);
    let current_vector = cubic_chord_vector(current);
    let next_vector = cubic_chord_vector(next);
    let entry_turn = vector_turn_angle(previous_vector, current_vector);
    let exit_turn = vector_turn_angle(current_vector, next_vector);
    let bridged_turn = vector_turn_angle(previous_vector, next_vector);

    entry_turn.max(exit_turn) >= MIN_SPIKE_TURN_RADIANS
        && (bridged_turn >= MIN_BRIDGED_TURN_RADIANS
            || (entry_turn >= MIN_SPIKE_TURN_RADIANS && exit_turn >= MIN_SPIKE_TURN_RADIANS))
}

fn regularize_potrace_orthogonal_corners(segments: Vec<SvgPathSegment>) -> Vec<SvgPathSegment> {
    if segments.len() < 5 {
        return segments;
    }

    let mut regularized = Vec::with_capacity(segments.len());
    let mut index = 0usize;
    let mut changed = false;

    while index < segments.len() {
        if let Some(cubic) = regularized_potrace_corner_pair(&segments, index) {
            regularized.push(SvgPathSegment::Cubic(cubic));
            changed = true;
            index += 2;
            continue;
        }

        if let Some(cubic) = regularized_potrace_corner(&segments, index) {
            regularized.push(SvgPathSegment::Cubic(cubic));
            changed = true;
            index += 1;
            continue;
        }

        regularized.push(segments[index]);
        index += 1;
    }

    if changed && regularized.len() >= 3 {
        regularized
    } else {
        segments
    }
}

fn regularized_potrace_corner_pair(
    segments: &[SvgPathSegment],
    index: usize,
) -> Option<CubicSegment> {
    const MAX_LEAD_TURN_RADIANS: f64 = 0.35;

    if index == 0 || index + 2 >= segments.len() {
        return None;
    }

    let (
        SvgPathSegment::Cubic(previous),
        SvgPathSegment::Cubic(lead),
        SvgPathSegment::Cubic(turn),
        SvgPathSegment::Cubic(next),
    ) = (
        segments[index - 1],
        segments[index],
        segments[index + 1],
        segments[index + 2],
    )
    else {
        return None;
    };

    if !potrace_segment_is_straight_edge(previous)
        || !potrace_segment_is_straight_edge(next)
        || !potrace_segment_is_short_straight_lead(lead)
        || !potrace_segment_is_roundable_corner(turn)
    {
        return None;
    }

    let previous_vector = cubic_chord_vector(previous);
    let lead_vector = cubic_chord_vector(lead);
    let next_vector = cubic_chord_vector(next);
    if vector_turn_angle(previous_vector, lead_vector) > MAX_LEAD_TURN_RADIANS
        || !vectors_are_roughly_orthogonal(previous_vector, next_vector)
    {
        return None;
    }

    let candidate = tangent_corner_cubic(lead.start, turn.end, previous_vector, next_vector)?;
    potrace_regularized_corner_is_close(&[lead, turn], candidate, 5.0).then_some(candidate)
}

fn regularized_potrace_corner(segments: &[SvgPathSegment], index: usize) -> Option<CubicSegment> {
    if index == 0 || index + 1 >= segments.len() {
        return None;
    }

    let (
        SvgPathSegment::Cubic(previous),
        SvgPathSegment::Cubic(current),
        SvgPathSegment::Cubic(next),
    ) = (segments[index - 1], segments[index], segments[index + 1])
    else {
        return None;
    };

    if !potrace_segment_is_straight_edge(previous)
        || !potrace_segment_is_straight_edge(next)
        || !potrace_segment_is_roundable_corner(current)
        || !vectors_are_roughly_orthogonal(cubic_chord_vector(previous), cubic_chord_vector(next))
    {
        return None;
    }

    let candidate = tangent_corner_cubic(
        current.start,
        current.end,
        cubic_chord_vector(previous),
        cubic_chord_vector(next),
    )?;
    potrace_regularized_corner_is_close(&[current], candidate, 3.5).then_some(candidate)
}

fn potrace_segment_is_straight_edge(cubic: CubicSegment) -> bool {
    const MIN_STRAIGHT_LENGTH: f64 = 40.0;
    const MAX_STRAIGHT_DEVIATION: f64 = 1.5;

    cubic_chord_length(cubic) >= MIN_STRAIGHT_LENGTH
        && cubic_chord_deviation(cubic) <= MAX_STRAIGHT_DEVIATION
}

fn potrace_segment_is_short_straight_lead(cubic: CubicSegment) -> bool {
    const MIN_LEAD_LENGTH: f64 = 4.0;
    const MAX_LEAD_LENGTH: f64 = 32.0;
    const MAX_LEAD_DEVIATION: f64 = 1.5;

    let length = cubic_chord_length(cubic);
    (MIN_LEAD_LENGTH..=MAX_LEAD_LENGTH).contains(&length)
        && cubic_chord_deviation(cubic) <= MAX_LEAD_DEVIATION
}

fn potrace_segment_is_roundable_corner(cubic: CubicSegment) -> bool {
    const MIN_CORNER_LENGTH: f64 = 6.0;
    const MAX_CORNER_LENGTH: f64 = 36.0;
    const MIN_CORNER_DEVIATION: f64 = 1.5;

    let length = cubic_chord_length(cubic);
    (MIN_CORNER_LENGTH..=MAX_CORNER_LENGTH).contains(&length)
        && cubic_chord_deviation(cubic) >= MIN_CORNER_DEVIATION
}

fn vectors_are_roughly_orthogonal(a: (f64, f64), b: (f64, f64)) -> bool {
    const MIN_ORTHOGONAL_TURN: f64 = 1.0;
    const MAX_ORTHOGONAL_TURN: f64 = 2.15;

    let turn = vector_turn_angle(a, b);
    (MIN_ORTHOGONAL_TURN..=MAX_ORTHOGONAL_TURN).contains(&turn)
}

fn tangent_corner_cubic(
    start: (f64, f64),
    end: (f64, f64),
    incoming: (f64, f64),
    outgoing: (f64, f64),
) -> Option<CubicSegment> {
    const CIRCLE_ARC_KAPPA: f64 = 0.552_284_749_830_793_6;
    const MIN_HANDLE_LENGTH: f64 = 2.0;

    let incoming = unit_vector(incoming);
    let outgoing = unit_vector(outgoing);
    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return None;
    }

    let delta = subtract(end, start);
    let incoming_projection = dot(delta, incoming);
    let outgoing_projection = dot(delta, outgoing);
    if incoming_projection <= 0.0 || outgoing_projection <= 0.0 {
        return None;
    }

    let handle = incoming_projection.min(outgoing_projection) * CIRCLE_ARC_KAPPA;
    if handle < MIN_HANDLE_LENGTH {
        return None;
    }

    Some(CubicSegment {
        start,
        control1: add(start, scale(incoming, handle)),
        control2: subtract(end, scale(outgoing, handle)),
        end,
    })
}

fn potrace_regularized_corner_is_close(
    source: &[CubicSegment],
    candidate: CubicSegment,
    tolerance: f64,
) -> bool {
    let samples = sample_cubic_run(source);
    cubic_runs_are_close(&samples, &[candidate], tolerance)
}

fn rotate_potrace_segments_after_last_line(segments: &[SvgPathSegment]) -> Vec<SvgPathSegment> {
    let Some(line_index) = segments
        .iter()
        .rposition(|segment| matches!(segment, SvgPathSegment::Line { .. }))
    else {
        return segments.to_vec();
    };

    let start = (line_index + 1) % segments.len();
    segments[start..]
        .iter()
        .chain(segments[..start].iter())
        .copied()
        .collect()
}

fn optimize_closed_potrace_curve_run(
    run: &[CubicSegment],
    opt_tolerance: f64,
) -> Vec<SvgPathSegment> {
    const CLOSED_SPLITS: usize = 4;

    if run.len() < CLOSED_SPLITS * 2 {
        return run.iter().copied().map(SvgPathSegment::Cubic).collect();
    }

    let mut optimized = Vec::new();

    for split in 0..CLOSED_SPLITS {
        let start = split * run.len() / CLOSED_SPLITS;
        let end = (split + 1) * run.len() / CLOSED_SPLITS;
        append_optimized_potrace_curve_run(&mut optimized, &run[start..end], opt_tolerance);
    }

    optimized
}

fn flush_potrace_curve_run(
    output: &mut Vec<SvgPathSegment>,
    run: &mut Vec<CubicSegment>,
    opt_tolerance: f64,
) {
    append_optimized_potrace_curve_run(output, run, opt_tolerance);
    run.clear();
}

fn append_optimized_potrace_curve_run(
    output: &mut Vec<SvgPathSegment>,
    run: &[CubicSegment],
    opt_tolerance: f64,
) {
    if run.is_empty() {
        return;
    }

    if run.len() <= 1 {
        output.extend(run.iter().copied().map(SvgPathSegment::Cubic));
        return;
    }

    output.extend(
        optimize_potrace_curve_run_graph(run, opt_tolerance)
            .into_iter()
            .map(SvgPathSegment::Cubic),
    );
}

fn optimize_potrace_curve_run_graph(run: &[CubicSegment], opt_tolerance: f64) -> Vec<CubicSegment> {
    let mut dp: Vec<Option<OpticurveState>> = vec![None; run.len() + 1];
    let mut edges: Vec<Vec<OpticurveEdge>> = vec![Vec::new(); run.len()];
    dp[0] = Some(OpticurveState {
        previous: 0,
        edge_index: 0,
        segments: 0,
        penalty: 0.0,
    });

    for start in 0..run.len() {
        let Some(state) = dp[start] else {
            continue;
        };

        let mut end = start + 1;
        while end <= run.len() {
            let Some(edge) = opticurve_edge(run, start, end, opt_tolerance) else {
                end += 1;
                continue;
            };
            let edge_index = edges[start].len();
            edges[start].push(edge);
            let candidate = OpticurveState {
                previous: start,
                edge_index,
                segments: state.segments + 1,
                penalty: state.penalty + edge.penalty,
            };

            if dp[end].is_none_or(|best| opticurve_state_is_better(candidate, best)) {
                dp[end] = Some(candidate);
            }

            end += 1;
        }
    }

    let Some(_) = dp[run.len()] else {
        return run.to_vec();
    };

    let mut merged = Vec::new();
    let mut cursor = run.len();

    while cursor != 0 {
        let state = dp[cursor].expect("opticurve cursor should be reachable");
        let edge = edges[state.previous][state.edge_index];
        merged.push(edge.cubic);
        cursor = state.previous;
    }

    merged.reverse();

    if merged.len() <= run.len() {
        merged
    } else {
        run.to_vec()
    }
}

#[derive(Debug, Clone, Copy)]
struct OpticurveState {
    previous: usize,
    edge_index: usize,
    segments: usize,
    penalty: f64,
}

#[derive(Debug, Clone, Copy)]
struct OpticurveEdge {
    cubic: CubicSegment,
    penalty: f64,
}

fn opticurve_state_is_better(candidate: OpticurveState, best: OpticurveState) -> bool {
    candidate.segments < best.segments
        || (candidate.segments == best.segments && candidate.penalty < best.penalty)
}

fn opticurve_edge(
    run: &[CubicSegment],
    start: usize,
    end: usize,
    opt_tolerance: f64,
) -> Option<OpticurveEdge> {
    let opt_tolerance = opt_tolerance.max(0.0);
    if end <= start {
        return None;
    }

    if end == start + 1 {
        return Some(OpticurveEdge {
            cubic: run[start],
            penalty: 0.0,
        });
    }

    if !cubic_run_has_consistent_convexity(&run[start..end]) {
        return None;
    }

    if let Some(edge) = potrace_area_opticurve_edge(run, start, end, opt_tolerance) {
        return Some(edge);
    }

    let samples = sample_cubic_run(&run[start..end]);
    let mut fitted = Vec::new();
    fit_open_cubic_segments_raw(&samples, opt_tolerance * opt_tolerance, &mut fitted);

    if fitted.len() != 1 || !cubic_runs_are_close(&samples, &fitted, opt_tolerance) {
        return None;
    }

    Some(OpticurveEdge {
        cubic: fitted[0],
        penalty: cubic_run_fit_penalty(&samples, fitted[0]),
    })
}

fn cubic_run_has_consistent_convexity(run: &[CubicSegment]) -> bool {
    let mut sign = 0.0_f64;

    for cubic in run {
        let start_tangent = subtract(cubic.control1, cubic.start);
        let end_tangent = subtract(cubic.end, cubic.control2);
        let turn = cross(start_tangent, end_tangent);

        if turn.abs() <= 1.0e-9 {
            continue;
        }

        if sign == 0.0 {
            sign = turn.signum();
        } else if turn.signum() != sign {
            return false;
        }
    }

    true
}

fn cubic_run_fit_penalty(samples: &[(f64, f64)], cubic: CubicSegment) -> f64 {
    samples
        .iter()
        .map(|sample| distance_squared_to_cubic_segments(*sample, &[cubic]))
        .sum()
}

struct ReconstructedPotraceRun {
    vertices: Vec<(f64, f64)>,
    alphas: Vec<f64>,
}

impl ReconstructedPotraceRun {
    fn from_cubics(run: &[CubicSegment]) -> Option<Self> {
        let mut vertices = Vec::with_capacity(run.len());
        let mut alphas = Vec::with_capacity(run.len());

        for cubic in run {
            let vertex = potrace_cubic_vertex(*cubic)?;
            let alpha = potrace_cubic_alpha(*cubic, vertex)?;
            vertices.push(vertex);
            alphas.push(alpha);
        }

        Some(Self { vertices, alphas })
    }
}

fn potrace_area_opticurve_edge(
    run: &[CubicSegment],
    start: usize,
    end: usize,
    opt_tolerance: f64,
) -> Option<OpticurveEdge> {
    if end <= start + 1 {
        return None;
    }

    let reconstructed = ReconstructedPotraceRun::from_cubics(run)?;
    let p0 = run[start].start;
    let p1 = reconstructed.vertices[start];
    let p2 = reconstructed.vertices[end - 1];
    let p3 = run[end - 1].end;
    let area = reconstructed_potrace_curve_area(&reconstructed, run, start, end);
    let a1 = signed_area_twice(p0, p1, p2);
    let a2 = signed_area_twice(p0, p1, p3);
    let a3 = signed_area_twice(p0, p2, p3);
    let a4 = a1 + a3 - a2;
    let t_denominator = a3 - a4;
    let s_denominator = a2 - a1;
    if t_denominator.abs() <= f64::EPSILON || s_denominator.abs() <= f64::EPSILON {
        return None;
    }

    let t = a3 / t_denominator;
    let s = a2 / s_denominator;
    let triangle_area = a2 * t / 2.0;
    if triangle_area.abs() <= f64::EPSILON {
        return None;
    }

    let radicand = 4.0 - area / triangle_area / 0.3;
    if radicand < 0.0 {
        return None;
    }

    let alpha = 2.0 - radicand.sqrt();
    if !alpha.is_finite() {
        return None;
    }

    let candidate = CubicSegment {
        start: p0,
        control1: interpolate(p0, p1, t * alpha),
        control2: interpolate(p3, p2, s * alpha),
        end: p3,
    };
    let penalty =
        potrace_area_opticurve_penalty(&reconstructed, run, start, end, candidate, opt_tolerance)?;

    Some(OpticurveEdge {
        cubic: candidate,
        penalty,
    })
}

fn reconstructed_potrace_curve_area(
    reconstructed: &ReconstructedPotraceRun,
    run: &[CubicSegment],
    start: usize,
    end: usize,
) -> f64 {
    let reference = reconstructed.vertices[0];
    let edge_start = run[start].start;
    let edge_end = run[end - 1].end;
    let mut area = 0.0;

    for index in start..end {
        let previous_end = if index == start {
            edge_start
        } else {
            run[index - 1].end
        };
        let end_point = run[index].end;
        let vertex = reconstructed.vertices[index];
        let alpha = reconstructed.alphas[index];
        area +=
            0.3 * alpha * (4.0 - alpha) * signed_area_twice(previous_end, vertex, end_point) / 2.0;
        area += signed_area_twice(reference, previous_end, end_point) / 2.0;
    }

    area - signed_area_twice(reference, edge_start, edge_end) / 2.0
}

fn potrace_area_opticurve_penalty(
    reconstructed: &ReconstructedPotraceRun,
    run: &[CubicSegment],
    start: usize,
    end: usize,
    candidate: CubicSegment,
    opt_tolerance: f64,
) -> Option<f64> {
    let mut penalty = 0.0;

    for index in start..end - 1 {
        let from = reconstructed.vertices[index];
        let to = reconstructed.vertices[index + 1];
        let parameter = bezier_tangent_parameter(candidate, from, to)?;
        let point = cubic_point(candidate, parameter);
        let length = distance(from, to);
        if length <= f64::EPSILON {
            return None;
        }

        let signed_distance = signed_area_twice(from, to, point) / length;
        if signed_distance.abs() > opt_tolerance {
            return None;
        }
        if dot(subtract(to, from), subtract(point, from)) < 0.0
            || dot(subtract(from, to), subtract(point, to)) < 0.0
        {
            return None;
        }

        penalty += signed_distance * signed_distance;
    }

    let edge_start = run[start].start;
    for index in start..end {
        let previous_end = if index == start {
            edge_start
        } else {
            run[index - 1].end
        };
        let end_point = run[index].end;
        let parameter = bezier_tangent_parameter(candidate, previous_end, end_point)?;
        let point = cubic_point(candidate, parameter);
        let length = distance(previous_end, end_point);
        if length <= f64::EPSILON {
            return None;
        }

        let mut signed_distance = signed_area_twice(previous_end, end_point, point) / length;
        let mut corner_distance =
            signed_area_twice(previous_end, end_point, reconstructed.vertices[index]) / length;
        corner_distance *= 0.75 * reconstructed.alphas[index];
        if corner_distance < 0.0 {
            signed_distance = -signed_distance;
            corner_distance = -corner_distance;
        }

        if signed_distance < corner_distance - opt_tolerance {
            return None;
        }
        if signed_distance < corner_distance {
            let delta = signed_distance - corner_distance;
            penalty += delta * delta;
        }
    }

    Some(penalty)
}

fn potrace_cubic_vertex(cubic: CubicSegment) -> Option<(f64, f64)> {
    let incoming = subtract(cubic.control1, cubic.start);
    let outgoing = subtract(cubic.control2, cubic.end);
    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return None;
    }

    line_intersection(
        FitLine {
            point: cubic.start,
            direction: incoming,
        },
        FitLine {
            point: cubic.end,
            direction: outgoing,
        },
    )
}

fn potrace_cubic_alpha(cubic: CubicSegment, vertex: (f64, f64)) -> Option<f64> {
    let entry_alpha = projected_fraction(cubic.start, vertex, cubic.control1)?;
    let exit_alpha = projected_fraction(cubic.end, vertex, cubic.control2)?;
    let alpha = (entry_alpha + exit_alpha) / 2.0;

    (alpha.is_finite() && alpha > 0.0 && alpha <= 2.0).then_some(alpha)
}

fn projected_fraction(start: (f64, f64), end: (f64, f64), point: (f64, f64)) -> Option<f64> {
    let vector = subtract(end, start);
    let length_squared = vector_length_squared(vector);
    if length_squared <= f64::EPSILON {
        return None;
    }

    Some(dot(subtract(point, start), vector) / length_squared)
}

fn bezier_tangent_parameter(
    cubic: CubicSegment,
    line_start: (f64, f64),
    line_end: (f64, f64),
) -> Option<f64> {
    let a = cross_lines(cubic.start, cubic.control1, line_start, line_end);
    let b = cross_lines(cubic.control1, cubic.control2, line_start, line_end);
    let c = cross_lines(cubic.control2, cubic.end, line_start, line_end);
    let quadratic_a = a - 2.0 * b + c;
    let quadratic_b = -2.0 * a + 2.0 * b;
    let quadratic_c = a;
    let discriminant = quadratic_b * quadratic_b - 4.0 * quadratic_a * quadratic_c;

    if quadratic_a.abs() <= f64::EPSILON {
        if quadratic_b.abs() <= f64::EPSILON {
            return None;
        }

        let linear = -quadratic_c / quadratic_b;
        return (0.0..=1.0).contains(&linear).then_some(linear);
    }

    if discriminant < 0.0 {
        return None;
    }

    let root = discriminant.sqrt();
    let first = (-quadratic_b + root) / (2.0 * quadratic_a);
    let second = (-quadratic_b - root) / (2.0 * quadratic_a);

    if (0.0..=1.0).contains(&first) {
        Some(first)
    } else if (0.0..=1.0).contains(&second) {
        Some(second)
    } else {
        None
    }
}

fn cross_lines(
    first_start: (f64, f64),
    first_end: (f64, f64),
    second_start: (f64, f64),
    second_end: (f64, f64),
) -> f64 {
    cross(
        subtract(first_end, first_start),
        subtract(second_end, second_start),
    )
}

fn signed_area_twice(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    cross(subtract(b, a), subtract(c, a))
}

fn fit_closed_smooth_potrace_segments(
    points: &[(f64, f64)],
    allow_ellipse_primitive: bool,
) -> Vec<SvgPathSegment> {
    const SMOOTH_FIT_ERROR: f64 = 1.1;

    if allow_ellipse_primitive {
        if let Some(capsule) = fit_closed_capsule_potrace_segments(points) {
            return capsule;
        }
        if let Some(ellipse) = fit_closed_ellipse_potrace_segments(points) {
            return ellipse;
        }
    }

    let breakpoints = even_fit_breakpoints(points.len());
    let mut segments = Vec::new();

    for index in 0..breakpoints.len() {
        let start = breakpoints[index];
        let end = breakpoints[(index + 1) % breakpoints.len()];
        let arc = closed_arc_points(points, start, end);
        fit_open_cubic_segments(&arc, SMOOTH_FIT_ERROR * SMOOTH_FIT_ERROR, &mut segments);
    }

    segments.into_iter().map(SvgPathSegment::Cubic).collect()
}

fn fit_closed_capsule_potrace_segments(points: &[(f64, f64)]) -> Option<Vec<SvgPathSegment>> {
    const MIN_RADIUS: f64 = 8.0;
    const MIN_ASPECT_RATIO: f64 = 1.2;

    let bounds = FloatBounds::from_points(points)?;
    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    if width >= height * MIN_ASPECT_RATIO {
        let radius = height / 2.0;
        if radius < MIN_RADIUS {
            return None;
        }

        let center_y = (bounds.min_y + bounds.max_y) / 2.0;
        let start = (bounds.min_x + radius, center_y);
        let end = (bounds.max_x - radius, center_y);
        capsule_boundary_is_close(points, start, end, radius)
            .then(|| horizontal_capsule_segments(bounds, radius))
    } else if height >= width * MIN_ASPECT_RATIO {
        let radius = width / 2.0;
        if radius < MIN_RADIUS {
            return None;
        }

        let center_x = (bounds.min_x + bounds.max_x) / 2.0;
        let start = (center_x, bounds.min_y + radius);
        let end = (center_x, bounds.max_y - radius);
        capsule_boundary_is_close(points, start, end, radius)
            .then(|| vertical_capsule_segments(bounds, radius))
    } else {
        None
    }
}

fn capsule_boundary_is_close(
    points: &[(f64, f64)],
    start: (f64, f64),
    end: (f64, f64),
    radius: f64,
) -> bool {
    const MAX_RADIAL_ERROR: f64 = 0.075;
    const MAX_MEAN_RADIAL_ERROR: f64 = 0.03;

    let mut max_error = 0.0_f64;
    let mut total_error = 0.0_f64;

    for point in points {
        let distance = distance_squared_to_segment(*point, start, end).0.sqrt();
        let error = ((distance - radius) / radius).abs();
        max_error = max_error.max(error);
        total_error += error;
    }

    max_error <= MAX_RADIAL_ERROR && total_error / points.len() as f64 <= MAX_MEAN_RADIAL_ERROR
}

// Pixel-traced capsules match Potrace more closely with slightly flatter arcs
// than the mathematical circle kappa.
const PIXEL_CAPSULE_ARC_KAPPA: f64 = 13.0 / 24.0;

fn horizontal_capsule_segments(bounds: FloatBounds, radius: f64) -> Vec<SvgPathSegment> {
    let center_y = (bounds.min_y + bounds.max_y) / 2.0;
    let left = (bounds.min_x, center_y);
    let top_left = (bounds.min_x + radius, bounds.min_y);
    let top_right = (bounds.max_x - radius, bounds.min_y);
    let right = (bounds.max_x, center_y);
    let bottom_right = (bounds.max_x - radius, bounds.max_y);
    let bottom_left = (bounds.min_x + radius, bounds.max_y);
    let handle = radius * PIXEL_CAPSULE_ARC_KAPPA;

    cubics_to_segments([
        CubicSegment {
            start: left,
            control1: (left.0, left.1 - handle),
            control2: (top_left.0 - handle, top_left.1),
            end: top_left,
        },
        line_as_cubic(top_left, top_right),
        CubicSegment {
            start: top_right,
            control1: (top_right.0 + handle, top_right.1),
            control2: (right.0, right.1 - handle),
            end: right,
        },
        CubicSegment {
            start: right,
            control1: (right.0, right.1 + handle),
            control2: (bottom_right.0 + handle, bottom_right.1),
            end: bottom_right,
        },
        line_as_cubic(bottom_right, bottom_left),
        CubicSegment {
            start: bottom_left,
            control1: (bottom_left.0 - handle, bottom_left.1),
            control2: (left.0, left.1 + handle),
            end: left,
        },
    ])
}

fn vertical_capsule_segments(bounds: FloatBounds, radius: f64) -> Vec<SvgPathSegment> {
    let center_x = (bounds.min_x + bounds.max_x) / 2.0;
    let top = (center_x, bounds.min_y);
    let right_top = (bounds.max_x, bounds.min_y + radius);
    let right_bottom = (bounds.max_x, bounds.max_y - radius);
    let bottom = (center_x, bounds.max_y);
    let left_bottom = (bounds.min_x, bounds.max_y - radius);
    let left_top = (bounds.min_x, bounds.min_y + radius);
    let handle = radius * PIXEL_CAPSULE_ARC_KAPPA;

    cubics_to_segments([
        CubicSegment {
            start: top,
            control1: (top.0 + handle, top.1),
            control2: (right_top.0, right_top.1 - handle),
            end: right_top,
        },
        line_as_cubic(right_top, right_bottom),
        CubicSegment {
            start: right_bottom,
            control1: (right_bottom.0, right_bottom.1 + handle),
            control2: (bottom.0 + handle, bottom.1),
            end: bottom,
        },
        CubicSegment {
            start: bottom,
            control1: (bottom.0 - handle, bottom.1),
            control2: (left_bottom.0, left_bottom.1 + handle),
            end: left_bottom,
        },
        line_as_cubic(left_bottom, left_top),
        CubicSegment {
            start: left_top,
            control1: (left_top.0, left_top.1 - handle),
            control2: (top.0 - handle, top.1),
            end: top,
        },
    ])
}

fn line_as_cubic(start: (f64, f64), end: (f64, f64)) -> CubicSegment {
    CubicSegment {
        start,
        control1: interpolate(start, end, 1.0 / 3.0),
        control2: interpolate(start, end, 2.0 / 3.0),
        end,
    }
}

fn cubics_to_segments(cubics: [CubicSegment; 6]) -> Vec<SvgPathSegment> {
    cubics.into_iter().map(SvgPathSegment::Cubic).collect()
}

fn fit_closed_ellipse_potrace_segments(points: &[(f64, f64)]) -> Option<Vec<SvgPathSegment>> {
    const MIN_AXIS: f64 = 8.0;
    const MAX_RADIAL_ERROR: f64 = 0.075;
    const MAX_MEAN_RADIAL_ERROR: f64 = 0.03;
    let bounds = FloatBounds::from_points(points)?;
    let rx = (bounds.max_x - bounds.min_x) / 2.0;
    let ry = (bounds.max_y - bounds.min_y) / 2.0;
    if rx < MIN_AXIS || ry < MIN_AXIS {
        return None;
    }

    let center = (
        (bounds.min_x + bounds.max_x) / 2.0,
        (bounds.min_y + bounds.max_y) / 2.0,
    );
    let mut max_error = 0.0_f64;
    let mut total_error = 0.0_f64;

    for point in points {
        let nx = (point.0 - center.0) / rx;
        let ny = (point.1 - center.1) / ry;
        let error = ((nx * nx + ny * ny).sqrt() - 1.0).abs();
        max_error = max_error.max(error);
        total_error += error;
    }

    let mean_error = total_error / points.len() as f64;
    if max_error > MAX_RADIAL_ERROR || mean_error > MAX_MEAN_RADIAL_ERROR {
        return None;
    }

    // Pixel circles from Potrace tend to use five cubic arcs rather than a
    // mathematically minimal four-arc ellipse.
    Some(ellipse_arc_segments(center, rx, ry, 5))
}

fn ellipse_arc_segments(
    center: (f64, f64),
    rx: f64,
    ry: f64,
    segment_count: usize,
) -> Vec<SvgPathSegment> {
    let step = 2.0 * std::f64::consts::PI / segment_count as f64;
    let handle = (4.0 / 3.0) * (step / 4.0).tan();

    (0..segment_count)
        .map(|index| {
            let start_angle = std::f64::consts::PI + step * index as f64;
            let end_angle = start_angle + step;
            let start = ellipse_point(center, rx, ry, start_angle);
            let end = ellipse_point(center, rx, ry, end_angle);
            let start_tangent = ellipse_tangent(rx, ry, start_angle);
            let end_tangent = ellipse_tangent(rx, ry, end_angle);

            SvgPathSegment::Cubic(CubicSegment {
                start,
                control1: add(start, scale(start_tangent, handle)),
                control2: subtract(end, scale(end_tangent, handle)),
                end,
            })
        })
        .collect()
}

fn ellipse_point(center: (f64, f64), rx: f64, ry: f64, angle: f64) -> (f64, f64) {
    (center.0 + rx * angle.cos(), center.1 + ry * angle.sin())
}

fn ellipse_tangent(rx: f64, ry: f64, angle: f64) -> (f64, f64) {
    (-rx * angle.sin(), ry * angle.cos())
}

fn points_are_half_pixel_quantized(points: &[(f64, f64)]) -> bool {
    points
        .iter()
        .all(|point| is_half_pixel_quantized(point.0) && is_half_pixel_quantized(point.1))
}

fn is_half_pixel_quantized(value: f64) -> bool {
    let doubled = value * 2.0;
    (doubled - doubled.round()).abs() <= 1.0e-6
}

fn sample_cubic_run(run: &[CubicSegment]) -> Vec<(f64, f64)> {
    const SAMPLES_PER_SEGMENT: usize = 4;

    let mut samples = Vec::with_capacity(run.len() * SAMPLES_PER_SEGMENT + 1);
    samples.push(run[0].start);

    for cubic in run {
        for step in 1..=SAMPLES_PER_SEGMENT {
            let parameter = step as f64 / SAMPLES_PER_SEGMENT as f64;
            samples.push(cubic_point(*cubic, parameter));
        }
    }

    dedup_nearby_points(samples)
}

fn cubic_runs_are_close(
    source_samples: &[(f64, f64)],
    fitted: &[CubicSegment],
    tolerance: f64,
) -> bool {
    let tolerance_squared = tolerance * tolerance;

    source_samples
        .iter()
        .all(|sample| distance_squared_to_cubic_segments(*sample, fitted) <= tolerance_squared)
        && fitted.iter().all(|cubic| {
            (1..16).all(|step| {
                let point = cubic_point(*cubic, step as f64 / 16.0);
                distance_squared_to_polyline(point, source_samples).0 <= tolerance_squared
            })
        })
}

fn distance_squared_to_cubic_segments(point: (f64, f64), segments: &[CubicSegment]) -> f64 {
    let mut best = f64::INFINITY;

    for segment in segments {
        for step in 0..=32 {
            let candidate = cubic_point(*segment, step as f64 / 32.0);
            best = best.min(distance_squared_float(point, candidate));
        }
    }

    best
}

fn dedup_nearby_points(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut deduped = Vec::with_capacity(points.len());

    for point in points {
        if deduped
            .last()
            .is_none_or(|previous| distance_squared_float(*previous, point) > 1.0e-12)
        {
            deduped.push(point);
        }
    }

    deduped
}

fn svg_path_data_from_segments(start: (f64, f64), segments: &[SvgPathSegment]) -> String {
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

fn compact_svg_path_data_from_segments(start: (f64, f64), segments: &[SvgPathSegment]) -> String {
    let mut best = compact_svg_path_data_for_order(start, segments);

    if compact_segments_are_closed(start, segments) {
        for offset in 1..segments.len() {
            let rotated = rotate_segments_at(segments, offset);
            let candidate = compact_svg_path_data_for_order(rotated[0].start(), &rotated);
            if candidate.len() < best.len() {
                best = candidate;
            }
        }
    }

    best
}

fn compact_svg_path_data_for_order(start: (f64, f64), segments: &[SvgPathSegment]) -> String {
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
    let arc = compact_circle_arc_svg_path_data_from_segments(segments)
        .map(|data| minify_compact_svg_path_data(&data));
    let axis_smooth = minify_compact_svg_path_data(
        &compact_axis_smooth_relative_svg_path_data_from_segments(start, segments),
    );

    let mut candidates = vec![absolute, relative, smooth, quadratic];
    if let Some(arc) = arc {
        candidates.push(arc);
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

fn compact_segments_without_redundant_closing_line(
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

fn closing_line_continues_last_line(
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

fn compact_circle_arc_svg_path_data_from_segments(segments: &[SvgPathSegment]) -> Option<String> {
    const MIN_CIRCLE_SEGMENTS: usize = 5;
    const MIN_AXIS: f64 = 8.0;
    const RADIUS_X_INSET: f64 = 0.15;
    const RADIUS_Y_INSET: f64 = 0.1;
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
    if radius < MIN_AXIS {
        return None;
    }

    for point in endpoints {
        if ((distance(point, center) - radius) / radius).abs() > MAX_RADIUS_ERROR {
            return None;
        }
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

fn compact_segments_are_closed(start: (f64, f64), segments: &[SvgPathSegment]) -> bool {
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

fn rotate_segments_at(segments: &[SvgPathSegment], offset: usize) -> Vec<SvgPathSegment> {
    segments[offset..]
        .iter()
        .chain(segments[..offset].iter())
        .copied()
        .collect()
}

fn minify_compact_svg_path_data(data: &str) -> String {
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

fn scaled_integer_svg_path_data(data: &str, scale_factor: f64) -> Option<String> {
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

fn one_decimal_svg_path_data(data: &str) -> Option<String> {
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

fn path_data_has_quadratic_commands(path_data: &str) -> bool {
    path_data.bytes().any(|byte| matches!(byte, b'Q' | b'q'))
}

fn format_one_decimal_half_away_from_zero(value: f64) -> String {
    let scaled = value * 10.0;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        (scaled - 0.5).ceil()
    };
    format_compact_float_with_precision(rounded / 10.0, 1)
}

fn svg_number_token_end(data: &str, start: usize) -> Option<usize> {
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

fn minify_svg_path_tokens(tokens: &[String]) -> String {
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

fn compact_path_tokens_need_separator(previous: &str, next: &str) -> bool {
    if compact_path_token_is_command(previous) || compact_path_token_is_command(next) {
        return false;
    }

    if next.starts_with('-') || next.starts_with('+') {
        return false;
    }

    !(next.starts_with('.') && previous.contains('.'))
}

fn compact_path_token_is_command(token: &str) -> bool {
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

fn compact_path_command_count(data: &str) -> usize {
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

fn compact_relative_line_command(start: (f64, f64), end: (f64, f64)) -> char {
    if line_axis_delta_is_zero(start.1, end.1) {
        'h'
    } else if line_axis_delta_is_zero(start.0, end.0) {
        'v'
    } else {
        'l'
    }
}

fn compact_relative_line_coordinates(start: (f64, f64), end: (f64, f64)) -> String {
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

fn line_axis_delta_is_zero(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1.0e-9
}

fn compact_absolute_svg_path_data_from_segments(
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

fn compact_relative_svg_path_data_from_segments(
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

fn compact_smooth_relative_svg_path_data_from_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    compact_smooth_relative_svg_path_data_with_line_mode(start, segments, false)
}

fn compact_quadratic_relative_svg_path_data_from_segments(
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

fn compact_axis_smooth_relative_svg_path_data_from_segments(
    start: (f64, f64),
    segments: &[SvgPathSegment],
) -> String {
    compact_smooth_relative_svg_path_data_with_line_mode(start, segments, true)
}

fn compact_smooth_relative_svg_path_data_with_line_mode(
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

fn cubic_control_reflection_is_close(
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

fn quadratic_approximation_for_tiny_cubic(cubic: CubicSegment) -> Option<(f64, f64)> {
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

fn quadratic_point(
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

fn fit_open_cubic_segments_raw(
    points: &[(f64, f64)],
    max_error_squared: f64,
    segments: &mut Vec<CubicSegment>,
) {
    if points.len() < 2 {
        return;
    }

    let left_tangent = unit_vector(subtract(points[1], points[0]));
    let right_tangent = unit_vector(subtract(points[points.len() - 2], points[points.len() - 1]));

    fit_cubic_recursive(
        points,
        left_tangent,
        right_tangent,
        max_error_squared,
        segments,
    );
}

fn edge_midpoint(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

fn potrace_curve_alpha(previous: (f64, f64), vertex: (f64, f64), next: (f64, f64)) -> f64 {
    let incoming_segment = subtract(vertex, previous);
    let outgoing_segment = subtract(next, vertex);
    let incoming_length = distance(vertex, previous);
    let outgoing_length = distance(next, vertex);
    let incoming = unit_vector(incoming_segment);
    let outgoing = unit_vector(outgoing_segment);

    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return 0.0;
    }

    let turn = dot(incoming, outgoing).clamp(-1.0, 1.0).acos();
    let base_alpha = (4.0 / 3.0) * (turn / 4.0).tan();
    base_alpha * incoming_length.min(outgoing_length).sqrt()
}

#[derive(Debug, Clone, Copy)]
struct CubicSegment {
    start: (f64, f64),
    control1: (f64, f64),
    control2: (f64, f64),
    end: (f64, f64),
}

fn cubic_chord_length(cubic: CubicSegment) -> f64 {
    (cubic.end.0 - cubic.start.0).hypot(cubic.end.1 - cubic.start.1)
}

fn cubic_chord_vector(cubic: CubicSegment) -> (f64, f64) {
    (cubic.end.0 - cubic.start.0, cubic.end.1 - cubic.start.1)
}

fn cubic_bounds_diagonal(cubic: CubicSegment) -> f64 {
    let min_x = cubic
        .start
        .0
        .min(cubic.control1.0)
        .min(cubic.control2.0)
        .min(cubic.end.0);
    let max_x = cubic
        .start
        .0
        .max(cubic.control1.0)
        .max(cubic.control2.0)
        .max(cubic.end.0);
    let min_y = cubic
        .start
        .1
        .min(cubic.control1.1)
        .min(cubic.control2.1)
        .min(cubic.end.1);
    let max_y = cubic
        .start
        .1
        .max(cubic.control1.1)
        .max(cubic.control2.1)
        .max(cubic.end.1);

    (max_x - min_x).hypot(max_y - min_y)
}

fn cubic_chord_deviation(cubic: CubicSegment) -> f64 {
    distance_squared_to_segment(cubic.control1, cubic.start, cubic.end)
        .0
        .max(distance_squared_to_segment(cubic.control2, cubic.start, cubic.end).0)
        .sqrt()
}

fn vector_turn_angle(a: (f64, f64), b: (f64, f64)) -> f64 {
    let a = unit_vector(a);
    let b = unit_vector(b);

    if vector_length_squared(a) <= f64::EPSILON || vector_length_squared(b) <= f64::EPSILON {
        0.0
    } else {
        dot(a, b).clamp(-1.0, 1.0).acos()
    }
}

#[derive(Debug, Clone, Copy)]
struct FloatBounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl FloatBounds {
    fn from_points(points: &[(f64, f64)]) -> Option<Self> {
        let (first, rest) = points.split_first()?;
        let mut bounds = Self {
            min_x: first.0,
            max_x: first.0,
            min_y: first.1,
            max_y: first.1,
        };

        for point in rest {
            bounds.min_x = bounds.min_x.min(point.0);
            bounds.max_x = bounds.max_x.max(point.0);
            bounds.min_y = bounds.min_y.min(point.1);
            bounds.max_y = bounds.max_y.max(point.1);
        }

        Some(bounds)
    }

    fn clamp(self, point: (f64, f64)) -> (f64, f64) {
        (
            point.0.clamp(self.min_x, self.max_x),
            point.1.clamp(self.min_y, self.max_y),
        )
    }
}

fn fit_closed_cubic_segments(points: &[(f64, f64)], error: f64) -> Vec<CubicSegment> {
    if points.len() < 2 {
        return Vec::new();
    }

    let breakpoints = fit_breakpoints(points);
    let mut segments = Vec::new();

    for index in 0..breakpoints.len() {
        let start = breakpoints[index];
        let end = breakpoints[(index + 1) % breakpoints.len()];
        let arc = closed_arc_points(points, start, end);
        fit_open_cubic_segments(&arc, error * error, &mut segments);
    }

    segments
}

fn fit_breakpoints(points: &[(f64, f64)]) -> Vec<usize> {
    let mut breakpoints = vec![0];

    for index in 1..points.len() {
        if is_sharp_fit_corner(points, index) {
            breakpoints.push(index);
        }
    }

    if breakpoints.len() < 2 {
        return even_fit_breakpoints(points.len());
    }

    breakpoints
}

fn even_fit_breakpoints(point_count: usize) -> Vec<usize> {
    let breakpoint_count = point_count.min(4);
    let mut breakpoints = Vec::with_capacity(breakpoint_count);

    for index in 0..breakpoint_count {
        let breakpoint = index * point_count / breakpoint_count;
        if breakpoints.last() != Some(&breakpoint) {
            breakpoints.push(breakpoint);
        }
    }

    breakpoints
}

fn is_sharp_fit_corner(points: &[(f64, f64)], index: usize) -> bool {
    // Smooth contours often contain small alternating turns from raster sampling.
    // Average a few neighboring segments before deciding whether a point is a
    // structural corner that should split a cubic fitting run.
    const CORNER_COSINE_THRESHOLD: f64 = 0.0;

    let steps = fit_corner_tangent_steps(points.len());
    let incoming = averaged_fit_tangent(points, index, FitTangentDirection::Incoming, steps);
    let outgoing = averaged_fit_tangent(points, index, FitTangentDirection::Outgoing, steps);

    if vector_length_squared(incoming) <= f64::EPSILON
        || vector_length_squared(outgoing) <= f64::EPSILON
    {
        return false;
    }

    let incoming = unit_vector(incoming);
    let outgoing = unit_vector(outgoing);

    dot(incoming, outgoing) <= CORNER_COSINE_THRESHOLD
}

#[derive(Debug, Clone, Copy)]
enum FitTangentDirection {
    Incoming,
    Outgoing,
}

fn fit_corner_tangent_steps(point_count: usize) -> usize {
    match point_count {
        0..=7 => 1,
        8..=17 => 2,
        _ => 3,
    }
}

fn averaged_fit_tangent(
    points: &[(f64, f64)],
    index: usize,
    direction: FitTangentDirection,
    steps: usize,
) -> (f64, f64) {
    let mut vector = (0.0, 0.0);
    let mut current = index;

    for _ in 0..steps {
        match direction {
            FitTangentDirection::Incoming => {
                let previous = (current + points.len() - 1) % points.len();
                vector = add(vector, subtract(points[current], points[previous]));
                current = previous;
            }
            FitTangentDirection::Outgoing => {
                let next = (current + 1) % points.len();
                vector = add(vector, subtract(points[next], points[current]));
                current = next;
            }
        }
    }

    vector
}

fn closed_arc_points(points: &[(f64, f64)], start: usize, end: usize) -> Vec<(f64, f64)> {
    let mut arc = Vec::new();
    let mut index = start;

    loop {
        arc.push(points[index]);

        if index == end {
            break;
        }

        index = (index + 1) % points.len();
    }

    arc
}

fn fit_open_cubic_segments(
    points: &[(f64, f64)],
    max_error_squared: f64,
    segments: &mut Vec<CubicSegment>,
) {
    if points.len() < 2 {
        return;
    }

    let points = smooth_open_fit_points(points);
    let left_tangent = unit_vector(subtract(points[1], points[0]));
    let right_tangent = unit_vector(subtract(points[points.len() - 2], points[points.len() - 1]));

    fit_cubic_recursive(
        &points,
        left_tangent,
        right_tangent,
        max_error_squared,
        segments,
    );
}

fn smooth_open_fit_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() < 9 {
        return points.to_vec();
    }

    let mut smoothed = points.to_vec();
    smooth_open_laplacian(&mut smoothed, 0.25);
    smooth_open_laplacian(&mut smoothed, -0.265);
    smoothed
}

fn smooth_open_laplacian(points: &mut [(f64, f64)], amount: f64) {
    let original = points.to_vec();

    for index in 1..points.len() - 1 {
        let midpoint = interpolate(original[index - 1], original[index + 1], 0.5);
        points[index] = add(
            original[index],
            scale(subtract(midpoint, original[index]), amount),
        );
    }
}

fn fit_cubic_recursive(
    points: &[(f64, f64)],
    left_tangent: (f64, f64),
    right_tangent: (f64, f64),
    max_error_squared: f64,
    segments: &mut Vec<CubicSegment>,
) {
    let bounds = FloatBounds::from_points(points).expect("fit segment should contain points");

    if points.len() == 2 {
        segments.push(clamp_cubic(
            linear_cubic(points[0], points[1], left_tangent, right_tangent),
            bounds,
        ));
        return;
    }

    let parameters = chord_length_parameters(points);
    let mut cubic = clamp_cubic(
        generate_cubic(points, &parameters, left_tangent, right_tangent),
        bounds,
    );
    let (mut error_squared, mut split_index) = max_cubic_error(points, &parameters, cubic);

    if error_squared <= max_error_squared {
        segments.push(cubic);
        return;
    }

    let mut refined_parameters = parameters;

    for _ in 0..4 {
        let Some(next_parameters) = reparameterize(points, &refined_parameters, cubic) else {
            break;
        };

        refined_parameters = next_parameters;
        let refined_cubic = clamp_cubic(
            generate_cubic(points, &refined_parameters, left_tangent, right_tangent),
            bounds,
        );
        let (refined_error, refined_split_index) =
            max_cubic_error(points, &refined_parameters, refined_cubic);

        if refined_error < error_squared {
            cubic = refined_cubic;
            error_squared = refined_error;
            split_index = refined_split_index;
        }

        if refined_error <= max_error_squared {
            segments.push(refined_cubic);
            return;
        }
    }

    if split_index == 0 || split_index + 1 >= points.len() {
        segments.push(cubic);
        return;
    }

    let center_tangent = center_tangent(points, split_index);
    fit_cubic_recursive(
        &points[..=split_index],
        left_tangent,
        center_tangent,
        max_error_squared,
        segments,
    );
    fit_cubic_recursive(
        &points[split_index..],
        scale(center_tangent, -1.0),
        right_tangent,
        max_error_squared,
        segments,
    );
}

fn clamp_cubic(cubic: CubicSegment, bounds: FloatBounds) -> CubicSegment {
    CubicSegment {
        start: cubic.start,
        control1: bounds.clamp(cubic.control1),
        control2: bounds.clamp(cubic.control2),
        end: cubic.end,
    }
}

fn linear_cubic(
    start: (f64, f64),
    end: (f64, f64),
    left_tangent: (f64, f64),
    right_tangent: (f64, f64),
) -> CubicSegment {
    let distance = distance(start, end) / 3.0;

    CubicSegment {
        start,
        control1: add(start, scale(left_tangent, distance)),
        control2: add(end, scale(right_tangent, distance)),
        end,
    }
}

fn chord_length_parameters(points: &[(f64, f64)]) -> Vec<f64> {
    let mut parameters = Vec::with_capacity(points.len());
    parameters.push(0.0);

    for index in 1..points.len() {
        parameters.push(parameters[index - 1] + distance(points[index - 1], points[index]));
    }

    let total = *parameters.last().unwrap_or(&0.0);
    if total <= f64::EPSILON {
        return (0..points.len())
            .map(|index| index as f64 / (points.len().saturating_sub(1)).max(1) as f64)
            .collect();
    }

    parameters
        .iter()
        .map(|parameter| parameter / total)
        .collect()
}

fn generate_cubic(
    points: &[(f64, f64)],
    parameters: &[f64],
    left_tangent: (f64, f64),
    right_tangent: (f64, f64),
) -> CubicSegment {
    let start = points[0];
    let end = points[points.len() - 1];
    let mut c00: f64 = 0.0;
    let mut c01: f64 = 0.0;
    let mut c11: f64 = 0.0;
    let mut x0: f64 = 0.0;
    let mut x1: f64 = 0.0;

    for (point, parameter) in points.iter().zip(parameters) {
        let (b0, b1, b2, b3) = bernstein3(*parameter);
        let a1 = scale(left_tangent, b1);
        let a2 = scale(right_tangent, b2);
        let endpoint_blend = add(scale(start, b0 + b1), scale(end, b2 + b3));
        let target = subtract(*point, endpoint_blend);

        c00 += dot(a1, a1);
        c01 += dot(a1, a2);
        c11 += dot(a2, a2);
        x0 += dot(a1, target);
        x1 += dot(a2, target);
    }

    let determinant = c00 * c11 - c01 * c01;
    let segment_length = distance(start, end);
    let epsilon = 1.0e-6 * segment_length;

    let (alpha1, alpha2) = if determinant.abs() > f64::EPSILON {
        (
            (x0 * c11 - x1 * c01) / determinant,
            (c00 * x1 - c01 * x0) / determinant,
        )
    } else {
        (segment_length / 3.0, segment_length / 3.0)
    };

    if alpha1 <= epsilon || alpha2 <= epsilon {
        return linear_cubic(start, end, left_tangent, right_tangent);
    }

    CubicSegment {
        start,
        control1: add(start, scale(left_tangent, alpha1)),
        control2: add(end, scale(right_tangent, alpha2)),
        end,
    }
}

fn max_cubic_error(points: &[(f64, f64)], parameters: &[f64], cubic: CubicSegment) -> (f64, usize) {
    let mut max_error = 0.0;
    let mut split_index = points.len() / 2;

    for index in 1..points.len() - 1 {
        let point = cubic_point(cubic, parameters[index]);
        let error = distance_squared_float(point, points[index]);

        if error > max_error {
            max_error = error;
            split_index = index;
        }
    }

    let sample_count = ((points.len() - 1) * 4).clamp(8, 32);
    for sample in 1..sample_count {
        let parameter = sample as f64 / sample_count as f64;
        let point = cubic_point(cubic, parameter);
        let (error, candidate_split_index) = distance_squared_to_polyline(point, points);

        if error > max_error {
            max_error = error;
            split_index = candidate_split_index;
        }
    }

    (max_error, split_index)
}

fn distance_squared_to_polyline(point: (f64, f64), points: &[(f64, f64)]) -> (f64, usize) {
    let mut min_error = f64::INFINITY;
    let mut split_index = points.len() / 2;

    for index in 0..points.len() - 1 {
        let (error, projection) =
            distance_squared_to_segment(point, points[index], points[index + 1]);

        if error < min_error {
            min_error = error;
            split_index = if projection < 0.5 { index } else { index + 1 };
        }
    }

    (min_error, split_index.clamp(1, points.len() - 2))
}

fn distance_squared_to_segment(
    point: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
) -> (f64, f64) {
    let segment = subtract(end, start);
    let length_squared = dot(segment, segment);

    if length_squared <= f64::EPSILON {
        return (distance_squared_float(point, start), 0.0);
    }

    let projection = (dot(subtract(point, start), segment) / length_squared).clamp(0.0, 1.0);
    let closest = add(start, scale(segment, projection));

    (distance_squared_float(point, closest), projection)
}

fn reparameterize(
    points: &[(f64, f64)],
    parameters: &[f64],
    cubic: CubicSegment,
) -> Option<Vec<f64>> {
    let mut refined_parameters = Vec::with_capacity(parameters.len());

    for (point, parameter) in points.iter().zip(parameters) {
        refined_parameters.push(newton_raphson_root_find(cubic, *point, *parameter));
    }

    if refined_parameters
        .windows(2)
        .all(|window| window[0] < window[1])
    {
        Some(refined_parameters)
    } else {
        None
    }
}

fn newton_raphson_root_find(cubic: CubicSegment, point: (f64, f64), parameter: f64) -> f64 {
    let curve_point = cubic_point(cubic, parameter);
    let first_derivative = cubic_derivative_point(cubic, parameter);
    let second_derivative = cubic_second_derivative_point(cubic, parameter);
    let difference = subtract(curve_point, point);
    let numerator = dot(difference, first_derivative);
    let denominator = dot(first_derivative, first_derivative) + dot(difference, second_derivative);

    if denominator.abs() <= f64::EPSILON {
        return parameter;
    }

    (parameter - numerator / denominator).clamp(0.0, 1.0)
}

fn center_tangent(points: &[(f64, f64)], index: usize) -> (f64, f64) {
    let previous = points[index - 1];
    let next = points[index + 1];
    unit_vector(subtract(previous, next))
}

fn cubic_point(cubic: CubicSegment, parameter: f64) -> (f64, f64) {
    let (b0, b1, b2, b3) = bernstein3(parameter);

    add(
        add(scale(cubic.start, b0), scale(cubic.control1, b1)),
        add(scale(cubic.control2, b2), scale(cubic.end, b3)),
    )
}

fn cubic_derivative_point(cubic: CubicSegment, parameter: f64) -> (f64, f64) {
    let inverse = 1.0 - parameter;
    let b0 = 3.0 * inverse * inverse;
    let b1 = 6.0 * inverse * parameter;
    let b2 = 3.0 * parameter * parameter;

    add(
        add(
            scale(subtract(cubic.control1, cubic.start), b0),
            scale(subtract(cubic.control2, cubic.control1), b1),
        ),
        scale(subtract(cubic.end, cubic.control2), b2),
    )
}

fn cubic_second_derivative_point(cubic: CubicSegment, parameter: f64) -> (f64, f64) {
    let inverse = 1.0 - parameter;
    let b0 = 6.0 * inverse;
    let b1 = 6.0 * parameter;

    add(
        scale(
            add(
                subtract(cubic.control2, scale(cubic.control1, 2.0)),
                cubic.start,
            ),
            b0,
        ),
        scale(
            add(
                subtract(cubic.end, scale(cubic.control2, 2.0)),
                cubic.control1,
            ),
            b1,
        ),
    )
}

fn bernstein3(parameter: f64) -> (f64, f64, f64, f64) {
    let inverse = 1.0 - parameter;

    (
        inverse * inverse * inverse,
        3.0 * parameter * inverse * inverse,
        3.0 * parameter * parameter * inverse,
        parameter * parameter * parameter,
    )
}

fn add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

fn subtract(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 - b.0, a.1 - b.1)
}

fn scale(vector: (f64, f64), scalar: f64) -> (f64, f64) {
    (vector.0 * scalar, vector.1 * scalar)
}

fn dot(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    distance_squared_float(a, b).sqrt()
}

fn distance_squared_float(a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;

    dx * dx + dy * dy
}

fn vector_length_squared(vector: (f64, f64)) -> f64 {
    vector.0 * vector.0 + vector.1 * vector.1
}

fn unit_vector(vector: (f64, f64)) -> (f64, f64) {
    let length = vector.0.hypot(vector.1);

    if length <= f64::EPSILON {
        (0.0, 0.0)
    } else {
        (vector.0 / length, vector.1 / length)
    }
}

fn catmull_rom_segment(path: &TracePath, index: usize) -> ((f64, f64), (f64, f64), (f64, f64)) {
    let previous = path.points[(index + path.points.len() - 1) % path.points.len()];
    let current = path.points[index];
    let next = path.points[(index + 1) % path.points.len()];
    let after_next = path.points[(index + 2) % path.points.len()];

    let control1 = (
        current.0 + (next.0 - previous.0) / 6.0,
        current.1 + (next.1 - previous.1) / 6.0,
    );
    let control2 = (
        next.0 - (after_next.0 - current.0) / 6.0,
        next.1 - (after_next.1 - current.1) / 6.0,
    );

    (control1, control2, next)
}

fn corner_entry(points: &[(f64, f64)], index: usize, amount: f64) -> (f64, f64) {
    let previous = points[(index + points.len() - 1) % points.len()];
    interpolate(points[index], previous, amount)
}

fn corner_exit(points: &[(f64, f64)], index: usize, amount: f64) -> (f64, f64) {
    let next = points[(index + 1) % points.len()];
    interpolate(points[index], next, amount)
}

fn interpolate(from: (f64, f64), to: (f64, f64), amount: f64) -> (f64, f64) {
    (
        from.0 + (to.0 - from.0) * amount,
        from.1 + (to.1 - from.1) * amount,
    )
}

fn cubic_control_point(endpoint: (f64, f64), corner: (f64, f64)) -> (f64, f64) {
    (
        endpoint.0 + (corner.0 - endpoint.0) * 2.0 / 3.0,
        endpoint.1 + (corner.1 - endpoint.1) * 2.0 / 3.0,
    )
}

fn format_float(value: f64) -> String {
    format_float_with_precision(value, 6)
}

fn format_compact_float(value: f64) -> String {
    format_compact_float_with_precision(value, 2)
}

fn format_compact_float_with_precision(value: f64, precision: usize) -> String {
    let mut formatted = format_float_with_precision(value, precision);
    if formatted.starts_with("0.") {
        formatted.remove(0);
    } else if formatted.starts_with("-0.") {
        formatted.remove(1);
    }
    formatted
}

fn format_float_with_precision(value: f64, precision: usize) -> String {
    let epsilon = 0.5 * 10.0_f64.powi(-(precision as i32));
    let value = if value.abs() < epsilon { 0.0 } else { value };
    let mut formatted = format!("{value:.precision$}");

    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }

    if formatted.ends_with('.') {
        formatted.pop();
    }

    formatted
}

struct PnmParser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> PnmParser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }

    fn parse(&mut self, options: RasterOptions) -> Result<Bitmap, PnmError> {
        let magic = self.next_string()?;

        match magic.as_str() {
            "P1" => self.parse_ascii_pbm(options),
            "P2" => self.parse_ascii_gray(options),
            "P3" => self.parse_ascii_rgb(options),
            "P4" => self.parse_binary_pbm(options),
            "P5" => self.parse_binary_gray(options),
            "P6" => self.parse_binary_rgb(options),
            _ => Err(PnmError::UnsupportedFormat(magic)),
        }
    }

    fn parse_ascii_pbm(&mut self, options: RasterOptions) -> Result<Bitmap, PnmError> {
        let (width, height) = self.read_dimensions()?;
        let len = checked_pixel_len(width, height, 1)?;
        let mut pixels = Vec::with_capacity(len);

        for _ in 0..len {
            let token = self.next_string()?;
            let black = match token.as_str() {
                "0" => false,
                "1" => true,
                _ => return Err(PnmError::InvalidToken(token)),
            };
            pixels.push(apply_invert(black, options));
        }

        Bitmap::from_rows(width, height, &pixels)
            .map_err(|_| PnmError::InvalidDimensions { width, height })
    }

    fn parse_ascii_gray(&mut self, options: RasterOptions) -> Result<Bitmap, PnmError> {
        let (width, height) = self.read_dimensions()?;
        let max_value = self.read_max_value()?;
        let len = checked_pixel_len(width, height, 1)?;
        let mut samples = Vec::with_capacity(len);

        for _ in 0..len {
            let sample = self.next_u32()?;
            samples.push(self.sample_to_luma(sample, max_value)?);
        }

        Bitmap::from_rows(width, height, &samples_to_pixels(&samples, options))
            .map_err(|_| PnmError::InvalidDimensions { width, height })
    }

    fn parse_ascii_rgb(&mut self, options: RasterOptions) -> Result<Bitmap, PnmError> {
        let (width, height) = self.read_dimensions()?;
        let max_value = self.read_max_value()?;
        let len = checked_pixel_len(width, height, 1)?;
        let mut samples = Vec::with_capacity(len);

        for _ in 0..len {
            let red = self.next_u32()?;
            let green = self.next_u32()?;
            let blue = self.next_u32()?;
            let luma = rgb_luma(red, green, blue);
            samples.push(self.sample_to_luma(luma, max_value)?);
        }

        Bitmap::from_rows(width, height, &samples_to_pixels(&samples, options))
            .map_err(|_| PnmError::InvalidDimensions { width, height })
    }

    fn parse_binary_pbm(&mut self, options: RasterOptions) -> Result<Bitmap, PnmError> {
        let (width, height) = self.read_dimensions()?;
        self.consume_binary_separator()?;

        let bytes_per_row = width.div_ceil(8);
        let raster_len = checked_pixel_len(bytes_per_row, height, 1)?;
        let raster = self.read_bytes(raster_len)?;
        let mut pixels = Vec::with_capacity(checked_pixel_len(width, height, 1)?);

        for y in 0..height {
            for x in 0..width {
                let byte = raster[y * bytes_per_row + x / 8];
                let mask = 1 << (7 - (x % 8));
                pixels.push(apply_invert(byte & mask != 0, options));
            }
        }

        Bitmap::from_rows(width, height, &pixels)
            .map_err(|_| PnmError::InvalidDimensions { width, height })
    }

    fn parse_binary_gray(&mut self, options: RasterOptions) -> Result<Bitmap, PnmError> {
        let (width, height) = self.read_dimensions()?;
        let max_value = self.read_max_value()?;
        self.consume_binary_separator()?;

        let len = checked_pixel_len(width, height, 1)?;
        let raster = self.read_bytes(len)?;
        let mut samples = Vec::with_capacity(len);

        for sample in raster {
            samples.push(self.sample_to_luma((*sample).into(), max_value)?);
        }

        Bitmap::from_rows(width, height, &samples_to_pixels(&samples, options))
            .map_err(|_| PnmError::InvalidDimensions { width, height })
    }

    fn parse_binary_rgb(&mut self, options: RasterOptions) -> Result<Bitmap, PnmError> {
        let (width, height) = self.read_dimensions()?;
        let max_value = self.read_max_value()?;
        self.consume_binary_separator()?;

        let pixel_count = checked_pixel_len(width, height, 1)?;
        let raster = self.read_bytes(checked_pixel_len(pixel_count, 3, 1)?)?;
        let mut samples = Vec::with_capacity(pixel_count);

        for rgb in raster.chunks_exact(3) {
            let luma = rgb_luma(rgb[0].into(), rgb[1].into(), rgb[2].into());
            samples.push(self.sample_to_luma(luma, max_value)?);
        }

        Bitmap::from_rows(width, height, &samples_to_pixels(&samples, options))
            .map_err(|_| PnmError::InvalidDimensions { width, height })
    }

    fn read_dimensions(&mut self) -> Result<(usize, usize), PnmError> {
        let width = self.next_usize()?;
        let height = self.next_usize()?;

        if width == 0 || height == 0 {
            return Err(PnmError::InvalidDimensions { width, height });
        }

        Ok((width, height))
    }

    fn read_max_value(&mut self) -> Result<u32, PnmError> {
        let max_value = self.next_u32()?;

        if !(1..=255).contains(&max_value) {
            return Err(PnmError::UnsupportedMaxValue(max_value));
        }

        Ok(max_value)
    }

    fn sample_to_luma(&self, sample: u32, max_value: u32) -> Result<u8, PnmError> {
        if sample > max_value {
            return Err(PnmError::InvalidToken(sample.to_string()));
        }

        let scaled = sample * 255 / max_value;
        Ok(scaled as u8)
    }

    fn next_string(&mut self) -> Result<String, PnmError> {
        let token = self.next_token()?;
        std::str::from_utf8(token)
            .map(|token| token.to_owned())
            .map_err(|_| PnmError::InvalidToken(String::from_utf8_lossy(token).into_owned()))
    }

    fn next_usize(&mut self) -> Result<usize, PnmError> {
        let token = self.next_string()?;
        token.parse().map_err(|_| PnmError::InvalidToken(token))
    }

    fn next_u32(&mut self) -> Result<u32, PnmError> {
        let token = self.next_string()?;
        token.parse().map_err(|_| PnmError::InvalidToken(token))
    }

    fn next_token(&mut self) -> Result<&'a [u8], PnmError> {
        self.skip_whitespace_and_comments();

        if self.index >= self.bytes.len() {
            return Err(PnmError::UnexpectedEof);
        }

        let start = self.index;

        while self.index < self.bytes.len() && !self.bytes[self.index].is_ascii_whitespace() {
            self.index += 1;
        }

        Ok(&self.bytes[start..self.index])
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.index < self.bytes.len() && self.bytes[self.index].is_ascii_whitespace() {
                self.index += 1;
            }

            if self.index >= self.bytes.len() || self.bytes[self.index] != b'#' {
                break;
            }

            while self.index < self.bytes.len() && self.bytes[self.index] != b'\n' {
                self.index += 1;
            }
        }
    }

    fn consume_binary_separator(&mut self) -> Result<(), PnmError> {
        if self.index >= self.bytes.len() {
            return Err(PnmError::UnexpectedEof);
        }

        if !self.bytes[self.index].is_ascii_whitespace() {
            return Err(PnmError::InvalidToken(
                String::from_utf8_lossy(&self.bytes[self.index..self.index + 1]).into_owned(),
            ));
        }

        self.index += 1;
        Ok(())
    }

    fn read_bytes(&mut self, count: usize) -> Result<&'a [u8], PnmError> {
        if self.index + count > self.bytes.len() {
            return Err(PnmError::UnexpectedEof);
        }

        let start = self.index;
        self.index += count;
        Ok(&self.bytes[start..self.index])
    }
}

fn checked_pixel_len(width: usize, height: usize, channels: usize) -> Result<usize, PnmError> {
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(channels))
        .ok_or(PnmError::InvalidDimensions { width, height })
}

fn apply_invert(black: bool, options: RasterOptions) -> bool {
    if options.invert {
        !black
    } else {
        black
    }
}

fn rgb_luma(red: u32, green: u32, blue: u32) -> u32 {
    (red * 299 + green * 587 + blue * 114) / 1000
}

fn samples_to_pixels(samples: &[u8], options: RasterOptions) -> Vec<bool> {
    let threshold = options.threshold.resolve(samples);

    samples
        .iter()
        .map(|sample| apply_invert(*sample < threshold, options))
        .collect()
}

fn otsu_threshold(samples: &[u8]) -> u8 {
    if samples.is_empty() {
        return 128;
    }

    let min = samples.iter().copied().min().unwrap_or(0);
    let max = samples.iter().copied().max().unwrap_or(255);

    if min == max {
        return if min < 128 { 255 } else { 0 };
    }

    let mut histogram = [0u64; 256];
    for sample in samples {
        histogram[usize::from(*sample)] += 1;
    }

    let total = samples.len() as f64;
    let total_sum = histogram
        .iter()
        .enumerate()
        .map(|(value, count)| value as f64 * *count as f64)
        .sum::<f64>();

    let mut background_weight = 0u64;
    let mut background_sum = 0f64;
    let mut best_threshold = 0u8;
    let mut best_variance = f64::NEG_INFINITY;

    for (threshold, count) in histogram.iter().enumerate().take(255) {
        background_weight += *count;

        if background_weight == 0 {
            continue;
        }

        let foreground_weight = samples.len() as u64 - background_weight;
        if foreground_weight == 0 {
            break;
        }

        background_sum += threshold as f64 * *count as f64;
        let background_mean = background_sum / background_weight as f64;
        let foreground_mean = (total_sum - background_sum) / foreground_weight as f64;
        let weight_product = background_weight as f64 * foreground_weight as f64;
        let variance = weight_product * (background_mean - foreground_mean).powi(2) / total;

        if variance > best_variance {
            best_variance = variance;
            best_threshold = threshold.saturating_add(1) as u8;
        }
    }

    best_threshold
}

fn parse_png(bytes: &[u8], options: RasterOptions) -> Result<Bitmap, PngError> {
    let image = parse_png_rgba_image(bytes)?;
    image
        .to_bitmap(options)
        .map_err(|_| PngError::InvalidDimensions {
            width: image.width as u32,
            height: image.height as u32,
        })
}

fn parse_png_rgba_image(bytes: &[u8]) -> Result<RgbaImage, PngError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);

    let mut reader = decoder
        .read_info()
        .map_err(|error| PngError::Decode(error.to_string()))?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let output = reader
        .next_frame(&mut buffer)
        .map_err(|error| PngError::Decode(error.to_string()))?;
    let raster = &buffer[..output.buffer_size()];

    let width = output.width as usize;
    let height = output.height as usize;

    if width == 0 || height == 0 {
        return Err(PngError::InvalidDimensions {
            width: output.width,
            height: output.height,
        });
    }

    let pixels = match output.color_type {
        png::ColorType::Rgb => png_rgb_to_rgba_pixels(raster),
        png::ColorType::Rgba => png_rgba_to_rgba_pixels(raster),
        png::ColorType::Grayscale => png_gray_to_rgba_pixels(raster),
        png::ColorType::GrayscaleAlpha => png_gray_alpha_to_rgba_pixels(raster),
        png::ColorType::Indexed => {
            return Err(PngError::UnsupportedColorType("indexed".to_owned()));
        }
    };

    RgbaImage::from_rows(width, height, &pixels).map_err(|_| PngError::InvalidDimensions {
        width: output.width,
        height: output.height,
    })
}

fn png_rgb_to_rgba_pixels(raster: &[u8]) -> Vec<Rgba8> {
    raster
        .chunks_exact(3)
        .map(|pixel| Rgba8 {
            red: pixel[0],
            green: pixel[1],
            blue: pixel[2],
            alpha: 255,
        })
        .collect()
}

fn png_rgba_to_rgba_pixels(raster: &[u8]) -> Vec<Rgba8> {
    raster
        .chunks_exact(4)
        .map(|pixel| Rgba8 {
            red: pixel[0],
            green: pixel[1],
            blue: pixel[2],
            alpha: pixel[3],
        })
        .collect()
}

fn png_gray_to_rgba_pixels(raster: &[u8]) -> Vec<Rgba8> {
    raster
        .iter()
        .map(|gray| Rgba8 {
            red: *gray,
            green: *gray,
            blue: *gray,
            alpha: 255,
        })
        .collect()
}

fn png_gray_alpha_to_rgba_pixels(raster: &[u8]) -> Vec<Rgba8> {
    raster
        .chunks_exact(2)
        .map(|pixel| Rgba8 {
            red: pixel[0],
            green: pixel[0],
            blue: pixel[0],
            alpha: pixel[1],
        })
        .collect()
}

fn rgba_pixels_to_luma_samples(pixels: &[Rgba8], alpha_background: AlphaBackground) -> Vec<u8> {
    pixels
        .iter()
        .map(|pixel| {
            let red = composite_over_background(pixel.red, pixel.alpha, alpha_background);
            let green = composite_over_background(pixel.green, pixel.alpha, alpha_background);
            let blue = composite_over_background(pixel.blue, pixel.alpha, alpha_background);
            rgb_luma(red, green, blue) as u8
        })
        .collect()
}

fn rgba_pixels_to_binary_pixels(pixels: &[Rgba8], options: RasterOptions) -> Vec<bool> {
    let samples = rgba_pixels_to_luma_samples(pixels, options.alpha_background);

    samples_to_pixels(&samples, options)
}

fn composite_over_background(color: u8, alpha: u8, background: AlphaBackground) -> u32 {
    let background = match background {
        AlphaBackground::Black => 0,
        AlphaBackground::White => 255,
    };

    (u32::from(color) * u32::from(alpha) + background * u32::from(255 - alpha)) / 255
}

fn parse_jpeg(bytes: &[u8], options: RasterOptions) -> Result<Bitmap, JpegError> {
    let image = parse_jpeg_rgba_image(bytes)?;
    image
        .to_bitmap(options)
        .map_err(|_| JpegError::InvalidDimensions {
            width: image.width as u16,
            height: image.height as u16,
        })
}

fn parse_jpeg_rgba_image(bytes: &[u8]) -> Result<RgbaImage, JpegError> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    let raster = decoder
        .decode()
        .map_err(|error| JpegError::Decode(error.to_string()))?;
    let info = decoder.info().ok_or(JpegError::MissingInfo)?;

    if info.width == 0 || info.height == 0 {
        return Err(JpegError::InvalidDimensions {
            width: info.width,
            height: info.height,
        });
    }

    let pixels = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => raster
            .into_iter()
            .map(|gray| Rgba8 {
                red: gray,
                green: gray,
                blue: gray,
                alpha: 255,
            })
            .collect::<Vec<_>>(),
        jpeg_decoder::PixelFormat::L16 => raster
            .chunks_exact(2)
            .map(|sample| Rgba8 {
                red: sample[0],
                green: sample[0],
                blue: sample[0],
                alpha: 255,
            })
            .collect::<Vec<_>>(),
        jpeg_decoder::PixelFormat::RGB24 => raster
            .chunks_exact(3)
            .map(|pixel| Rgba8 {
                red: pixel[0],
                green: pixel[1],
                blue: pixel[2],
                alpha: 255,
            })
            .collect::<Vec<_>>(),
        jpeg_decoder::PixelFormat::CMYK32 => {
            return Err(JpegError::UnsupportedPixelFormat("cmyk32".to_owned()));
        }
    };

    RgbaImage::from_rows(usize::from(info.width), usize::from(info.height), &pixels).map_err(|_| {
        JpegError::InvalidDimensions {
            width: info.width,
            height: info.height,
        }
    })
}

struct BmpParser<'a> {
    bytes: &'a [u8],
}

impl<'a> BmpParser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn parse(&self, options: RasterOptions) -> Result<Bitmap, BmpError> {
        if self.bytes.get(0..2) != Some(b"BM") {
            return Err(BmpError::InvalidSignature);
        }

        let pixel_offset = self.read_u32(10)? as usize;
        let dib_header_size = self.read_u32(14)?;

        if dib_header_size < 40 {
            return Err(BmpError::UnsupportedDibHeader(dib_header_size));
        }

        let width = self.read_i32(18)?;
        let height = self.read_i32(22)?;

        if width <= 0 || height == 0 {
            return Err(BmpError::InvalidDimensions { width, height });
        }

        let planes = self.read_u16(26)?;
        if planes != 1 {
            return Err(BmpError::UnsupportedPlanes(planes));
        }

        let bits_per_pixel = self.read_u16(28)?;
        if !matches!(bits_per_pixel, 24 | 32) {
            return Err(BmpError::UnsupportedBitsPerPixel(bits_per_pixel));
        }

        let compression = self.read_u32(30)?;
        if compression != 0 {
            return Err(BmpError::UnsupportedCompression(compression));
        }

        let width = width as usize;
        let height_abs = height.unsigned_abs() as usize;
        let bytes_per_pixel = usize::from(bits_per_pixel / 8);
        let row_stride = bmp_row_stride(width, bits_per_pixel);
        let raster_len = row_stride
            .checked_mul(height_abs)
            .ok_or(BmpError::InvalidDimensions {
                width: width as i32,
                height,
            })?;
        let raster_end = pixel_offset
            .checked_add(raster_len)
            .ok_or(BmpError::InvalidPixelOffset(pixel_offset))?;

        if raster_end > self.bytes.len() {
            return Err(BmpError::UnexpectedEof);
        }

        let top_down = height < 0;
        let mut samples = Vec::with_capacity(width * height_abs);

        for y in 0..height_abs {
            let source_y = if top_down { y } else { height_abs - 1 - y };
            let row_start = pixel_offset + source_y * row_stride;

            for x in 0..width {
                let pixel_start = row_start + x * bytes_per_pixel;
                let blue = u32::from(self.bytes[pixel_start]);
                let green = u32::from(self.bytes[pixel_start + 1]);
                let red = u32::from(self.bytes[pixel_start + 2]);
                samples.push(rgb_luma(red, green, blue) as u8);
            }
        }

        Bitmap::from_rows(width, height_abs, &samples_to_pixels(&samples, options)).map_err(|_| {
            BmpError::InvalidDimensions {
                width: width as i32,
                height,
            }
        })
    }

    fn read_u16(&self, offset: usize) -> Result<u16, BmpError> {
        let bytes = self
            .bytes
            .get(offset..offset + 2)
            .ok_or(BmpError::UnexpectedEof)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&self, offset: usize) -> Result<u32, BmpError> {
        let bytes = self
            .bytes
            .get(offset..offset + 4)
            .ok_or(BmpError::UnexpectedEof)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_i32(&self, offset: usize) -> Result<i32, BmpError> {
        let bytes = self
            .bytes
            .get(offset..offset + 4)
            .ok_or(BmpError::UnexpectedEof)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

fn bmp_row_stride(width: usize, bits_per_pixel: u16) -> usize {
    (width * usize::from(bits_per_pixel)).div_ceil(32) * 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optimal_potrace_polygon_reduces_nearly_straight_stair_steps() {
        let mut points = (0..=12)
            .map(|x| (x as f64, if x % 2 == 0 { 0.0 } else { 0.2 }))
            .collect::<Vec<_>>();
        points.extend([(12.0, 6.0), (0.0, 6.0), (0.0, 0.0)]);

        let polygon = optimal_potrace_polygon_indices(&points);

        assert!(polygon.len() < points.len() / 2, "{polygon:?}");
    }

    #[test]
    fn vertex_adjustment_moves_corner_toward_fitted_line_intersection() {
        let points = vec![
            (0.0, 0.0),
            (1.0, 0.0),
            (2.0, 0.2),
            (2.0, 1.0),
            (2.0, 2.0),
            (0.0, 2.0),
        ];
        let adjusted = adjust_potrace_vertices(&points, &[0, 2, 4, 5], 1.0);

        assert!(
            adjusted[1].1 < points[2].1,
            "corner did not move toward the fitted intersection: {adjusted:?}"
        );
    }

    #[test]
    fn graph_opticurve_merges_compatible_adjacent_curves() {
        let run = vec![
            CubicSegment {
                start: (0.0, 0.0),
                control1: (0.33, 0.0),
                control2: (0.66, 0.0),
                end: (1.0, 0.0),
            },
            CubicSegment {
                start: (1.0, 0.0),
                control1: (1.33, 0.0),
                control2: (1.66, 0.0),
                end: (2.0, 0.0),
            },
        ];

        let optimized = optimize_potrace_curve_run_graph(&run, 0.2);

        assert_eq!(optimized.len(), 1, "{optimized:?}");
    }

    #[test]
    fn compact_path_data_uses_relative_segments_when_shorter() {
        let segments = vec![
            SvgPathSegment::Line {
                start: (10.0, 10.0),
                end: (15.0, 9.0),
            },
            SvgPathSegment::Cubic(CubicSegment {
                start: (15.0, 9.0),
                control1: (17.0, 9.0),
                control2: (19.0, 12.0),
                end: (21.0, 15.0),
            }),
        ];

        let data = compact_svg_path_data_from_segments((10.0, 10.0), &segments);

        assert_eq!(data, "M10 10l5-1c2 0 4 3 6 6Z");
    }

    #[test]
    fn compact_path_data_keeps_absolute_segments_when_shorter() {
        let segments = vec![
            SvgPathSegment::Line {
                start: (1000.0, 1000.0),
                end: (0.0, 0.0),
            },
            SvgPathSegment::Cubic(CubicSegment {
                start: (0.0, 0.0),
                control1: (0.0, 0.0),
                control2: (0.0, 0.0),
                end: (0.0, 0.0),
            }),
        ];

        let data = compact_svg_path_data_from_segments((1000.0, 1000.0), &segments);

        assert_eq!(data, "M1000 1000L0 0C0 0 0 0 0 0Z");
    }

    #[test]
    fn compact_path_data_limits_fractional_precision() {
        let segments = vec![SvgPathSegment::Line {
            start: (10.12345, 20.98765),
            end: (11.55555, 22.44444),
        }];

        let data = compact_svg_path_data_from_segments((10.12345, 20.98765), &segments);

        assert_eq!(data, "M10.12 20.99l1.43 1.46Z");
    }

    #[test]
    fn compact_path_data_omits_fractional_leading_zeroes() {
        let segments = vec![SvgPathSegment::Line {
            start: (0.25, -0.25),
            end: (0.75, -0.75),
        }];

        let data = compact_svg_path_data_from_segments((0.25, -0.25), &segments);

        assert_eq!(data, "M.25-.25l.5-.5Z");
    }

    #[test]
    fn compact_path_data_omits_separator_before_fraction_after_decimal() {
        let segments = vec![SvgPathSegment::Line {
            start: (1.5, 0.25),
            end: (2.5, 0.75),
        }];

        let data = compact_svg_path_data_from_segments((1.5, 0.25), &segments);

        assert_eq!(data, "M1.5.25l1 .5Z");
    }

    #[test]
    fn compact_path_data_uses_axis_line_shorthand() {
        let segments = vec![SvgPathSegment::Line {
            start: (0.0, 0.0),
            end: (10.0, 0.0),
        }];

        let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

        assert_eq!(data, "M0 0h10Z");
    }

    #[test]
    fn compact_path_data_omits_redundant_closing_line() {
        let segments = vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (10.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 10.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 0.0),
            },
        ];

        let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

        assert_eq!(data, "M0 0l10 0 0 10-10 0Z");
    }

    #[test]
    fn compact_path_data_omits_collinear_line_before_close() {
        let segments = vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (10.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 10.0),
                end: (0.0, 10.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 10.0),
                end: (0.0, 5.0),
            },
        ];

        let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

        assert_eq!(data, "M0 0l10 0 0 10-10 0Z");
    }

    #[test]
    fn compact_path_data_rotates_closed_segments_to_shorter_start() {
        let segments = vec![
            SvgPathSegment::Line {
                start: (1000.0, 1000.0),
                end: (0.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (1.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (1.0, 0.0),
                end: (1000.0, 1000.0),
            },
        ];

        let data = compact_svg_path_data_from_segments((1000.0, 1000.0), &segments);

        assert!(data.starts_with("M0 0"), "{data}");
    }

    #[test]
    fn compact_path_data_uses_smooth_cubic_shorthand() {
        let segments = vec![
            SvgPathSegment::Cubic(CubicSegment {
                start: (0.0, 0.0),
                control1: (0.0, 10.0),
                control2: (10.0, 10.0),
                end: (10.0, 0.0),
            }),
            SvgPathSegment::Cubic(CubicSegment {
                start: (10.0, 0.0),
                control1: (10.0, -10.0),
                control2: (20.0, -10.0),
                end: (20.0, 0.0),
            }),
        ];

        let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

        assert_eq!(data, "M0 0c0 10 10 10 10 0s10-10 10 0Z");
    }

    #[test]
    fn compact_path_data_uses_quadratic_for_tiny_cubic() {
        let segments = vec![SvgPathSegment::Cubic(CubicSegment {
            start: (0.0, 0.0),
            control1: (0.19, -0.75),
            control2: (1.81, -0.75),
            end: (2.0, 0.0),
        })];

        let data = compact_svg_path_data_from_segments((0.0, 0.0), &segments);

        assert!(data.contains('q'), "{data}");
        assert!(!data.contains('c'), "{data}");
    }

    #[test]
    fn compact_path_data_uses_arc_for_circle_primitive() {
        let segments = ellipse_arc_segments((128.0, 128.0), 76.0, 76.0, 5);
        let data = compact_svg_path_data_from_segments(segments[0].start(), &segments);

        assert!(data.contains('a'), "{data}");
        assert!(data.contains("75.85 75.9"), "{data}");
        assert!(data.len() <= 61, "{data}");
    }

    #[test]
    fn scaled_integer_path_data_preserves_numeric_separators() {
        let data = scaled_integer_svg_path_data(
            "M52 128c0-32.93 21.2-62.11 52.51-72.28s65.62.97 84.97 27.61Z",
            10.0,
        )
        .expect("compact path data should parse");

        assert!(data.contains("656 10"), "{data}");
        assert!(!data.contains("65610"), "{data}");
    }

    #[test]
    fn scaled_potrace_path_element_is_used_only_when_shorter() {
        let diagonal = "M92.5 183c-11 9.62-21.5 18.18-23.32 19-6.29 2.84-15.93 1.27-19.72-3.2-4.94-5.83-6.24-13.59-3.43-20.53.84-2.07 23.11-22.67 49.5-45.77l67.98-59.5c11-9.62 21.5-18.17 23.32-19 6.29-2.84 15.93-1.27 19.72 3.2 4.94 5.83 6.24 13.59 3.43 20.53-.84 2.07-23.11 22.67-49.5 45.77l-67.98 59.5Z";
        let square = "M72 72l112 0 0 112-112 0 0-56 0-56Z";

        let diagonal_path = svg_path_element(diagonal, true);
        let square_path = svg_path_element(square, true);

        assert!(!diagonal_path.contains("transform="), "{diagonal_path}");
        assert!(diagonal_path.contains("9.6"), "{diagonal_path}");
        assert!(diagonal_path.len() < svg_path_element(diagonal, false).len());
        assert!(!square_path.contains("transform="), "{square_path}");
    }

    #[test]
    fn one_decimal_path_element_uses_half_away_rounding_for_quadratics() {
        let triangle = "M84.5 129l42.5-85.25q1-1.12 2 0l42.5 85.25 42.5 84.75-86 .25-86-.25Z";

        let path = svg_path_element(triangle, true);

        assert!(path.contains("-85.3"), "{path}");
        assert!(path.contains("q1-1.1"), "{path}");
        assert!(path.len() < svg_path_element(triangle, false).len());
    }

    #[test]
    fn one_decimal_path_element_skips_arc_commands() {
        let circle = "M52.15 128a75.85 75.9 0 1 0 151.7 0a75.85 75.9 0 1 0-151.7 0Z";

        let path = svg_path_element(circle, true);

        assert!(path.contains("75.85"), "{path}");
        assert!(!path.contains("75.9 75.9"), "{path}");
    }

    #[test]
    fn pixel_potrace_candidate_selection_rejects_shorter_mask_regression() {
        let path = TracePath {
            is_hole: false,
            points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        };
        let best = (
            (0.0, 0.0),
            vec![
                SvgPathSegment::Line {
                    start: (0.0, 0.0),
                    end: (10.0, 0.0),
                },
                SvgPathSegment::Line {
                    start: (10.0, 0.0),
                    end: (10.0, 10.0),
                },
                SvgPathSegment::Line {
                    start: (10.0, 10.0),
                    end: (0.0, 10.0),
                },
                SvgPathSegment::Line {
                    start: (0.0, 10.0),
                    end: (0.0, 0.0),
                },
            ],
        );
        let shorter_wrong = (
            (0.0, 0.0),
            vec![
                SvgPathSegment::Line {
                    start: (0.0, 0.0),
                    end: (10.0, 0.0),
                },
                SvgPathSegment::Line {
                    start: (10.0, 0.0),
                    end: (0.0, 10.0),
                },
                SvgPathSegment::Line {
                    start: (0.0, 10.0),
                    end: (0.0, 0.0),
                },
            ],
        );

        assert!(pixel_potrace_candidate_is_better(
            &path,
            None,
            &shorter_wrong,
            &best
        ));
        assert!(!pixel_potrace_candidate_is_better(
            &path,
            Some((12, 12)),
            &shorter_wrong,
            &best
        ));
    }

    #[test]
    fn pixel_trace_can_preserve_collinear_boundary_points() {
        let bitmap =
            Bitmap::from_rows(3, 1, &[true, true, true]).expect("bitmap dimensions should match");
        let simplified = trace_bitmap(
            &bitmap,
            TraceOptions {
                turd_size: 0,
                opt_tolerance: 0.0,
                contour_mode: ContourMode::Pixel,
                preserve_collinear: false,
            },
        );
        let preserved = trace_bitmap(
            &bitmap,
            TraceOptions {
                turd_size: 0,
                opt_tolerance: 0.0,
                contour_mode: ContourMode::Pixel,
                preserve_collinear: true,
            },
        );

        assert_eq!(simplified.paths[0].points.len(), 4);
        assert_eq!(preserved.paths[0].points.len(), 8);
    }

    #[test]
    fn closed_ellipse_potrace_fit_uses_five_cubics() {
        let points = (0..64)
            .map(|index| {
                let angle = index as f64 * std::f64::consts::TAU / 64.0;
                (40.0 + angle.cos() * 20.0, 30.0 + angle.sin() * 12.0)
            })
            .collect::<Vec<_>>();

        let segments = fit_closed_ellipse_potrace_segments(&points)
            .expect("ellipse-like points should fit the primitive");

        assert_eq!(segments.len(), 5);
        assert!(segments
            .iter()
            .all(|segment| matches!(segment, SvgPathSegment::Cubic(_))));
    }

    #[test]
    fn closed_capsule_potrace_fit_uses_six_cubics() {
        let center_y = 40.0;
        let radius = 20.0;
        let left_center = (30.0, center_y);
        let right_center = (70.0, center_y);
        let mut points = Vec::new();

        for index in 0..=8 {
            points.push((30.0 + index as f64 * 5.0, center_y - radius));
        }
        for index in 1..=16 {
            let angle = -std::f64::consts::FRAC_PI_2 + index as f64 * std::f64::consts::PI / 16.0;
            points.push((
                right_center.0 + angle.cos() * radius,
                right_center.1 + angle.sin() * radius,
            ));
        }
        for index in 1..=8 {
            points.push((70.0 - index as f64 * 5.0, center_y + radius));
        }
        for index in 1..=16 {
            let angle = std::f64::consts::FRAC_PI_2 + index as f64 * std::f64::consts::PI / 16.0;
            points.push((
                left_center.0 + angle.cos() * radius,
                left_center.1 + angle.sin() * radius,
            ));
        }

        let segments = fit_closed_capsule_potrace_segments(&points)
            .expect("capsule-like points should fit the primitive");

        assert_eq!(segments.len(), 6);
        assert!(segments
            .iter()
            .all(|segment| matches!(segment, SvgPathSegment::Cubic(_))));
    }

    #[test]
    fn pixel_capsule_primitive_uses_compact_integer_handles() {
        let bounds = FloatBounds {
            min_x: 40.0,
            max_x: 216.0,
            min_y: 80.0,
            max_y: 176.0,
        };
        let segments = cleanup_potrace_segments(
            horizontal_capsule_segments(bounds, 48.0),
            PIXEL_POTRACE_LINEAR_DEVIATION,
        );
        let path_data = compact_svg_path_data_from_segments(segments[0].start(), &segments);

        assert_eq!(
            path_data,
            "M88 176c-26 0-48-22-48-48s22-48 48-48h80c26 0 48 22 48 48s-22 48-48 48Z"
        );
    }

    #[test]
    fn icon_candidate_selection_uses_global_fit_band() {
        let candidates = vec![
            test_icon_candidate(0.0, 10.0, 100, 100),
            test_icon_candidate(0.0015, 8.0, 80, 80),
            test_icon_candidate(0.003, 1.0, 10, 10),
        ];

        let best_index = best_icon_candidate_index(&candidates).expect("candidates should exist");

        assert_eq!(best_index, 1);
    }

    #[test]
    fn potrace_segment_cleanup_removes_tiny_spike_between_long_curves() {
        let segments = vec![
            SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (10.0, 0.0))),
            SvgPathSegment::Cubic(test_cubic((10.0, 0.0), (20.0, 0.0))),
            SvgPathSegment::Cubic(CubicSegment {
                start: (20.0, 0.0),
                control1: (19.9, 0.0),
                control2: (18.6, -0.9),
                end: (18.4, -1.2),
            }),
            SvgPathSegment::Cubic(test_cubic((18.4, -1.2), (30.0, 0.0))),
            SvgPathSegment::Cubic(test_cubic((30.0, 0.0), (40.0, 0.0))),
        ];

        let pruned = prune_tiny_potrace_curve_segments(segments);

        assert_eq!(pruned.len(), 4);
    }

    #[test]
    fn potrace_segment_cleanup_demotes_nearly_linear_cubics() {
        let segments = [
            SvgPathSegment::Cubic(CubicSegment {
                start: (0.0, 0.0),
                control1: (33.0, 0.8),
                control2: (66.0, -0.8),
                end: (100.0, 0.0),
            }),
            SvgPathSegment::Cubic(CubicSegment {
                start: (100.0, 0.0),
                control1: (100.0, 40.0),
                control2: (140.0, 40.0),
                end: (140.0, 0.0),
            }),
        ];

        let strict_cleaned =
            demote_nearly_linear_potrace_cubics(segments.to_vec(), STRICT_POTRACE_LINEAR_DEVIATION);
        let pixel_cleaned =
            demote_nearly_linear_potrace_cubics(segments.to_vec(), PIXEL_POTRACE_LINEAR_DEVIATION);

        assert!(matches!(strict_cleaned[0], SvgPathSegment::Cubic(_)));
        assert!(matches!(pixel_cleaned[0], SvgPathSegment::Line { .. }));
        assert!(matches!(pixel_cleaned[1], SvgPathSegment::Cubic(_)));
    }

    #[test]
    fn potrace_segment_cleanup_merges_adjacent_collinear_lines() {
        let segments = vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (20.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (20.0, 0.0),
                end: (20.0, 10.0),
            },
        ];

        let merged = merge_collinear_potrace_lines(segments);

        assert_eq!(merged.len(), 2, "{merged:?}");
        assert!(matches!(
            merged[0],
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (20.0, 0.0)
            }
        ));
    }

    #[test]
    fn potrace_segment_cleanup_keeps_reversing_collinear_lines() {
        let segments = vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (10.0, 0.0),
            },
            SvgPathSegment::Line {
                start: (10.0, 0.0),
                end: (0.0, 0.0),
            },
        ];

        let merged = merge_collinear_potrace_lines(segments);

        assert_eq!(merged.len(), 2, "{merged:?}");
    }

    #[test]
    fn potrace_segment_cleanup_reruns_curve_optimization_after_linear_demotion() {
        let segments = vec![
            SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (1.0, 0.0))),
            SvgPathSegment::Cubic(test_cubic((1.0, 0.0), (2.0, 0.0))),
            SvgPathSegment::Cubic(line_as_cubic((2.0, 0.0), (30.0, 0.0))),
            SvgPathSegment::Cubic(test_cubic((30.0, 0.0), (31.0, 0.0))),
            SvgPathSegment::Cubic(test_cubic((31.0, 0.0), (32.0, 0.0))),
            SvgPathSegment::Cubic(line_as_cubic((32.0, 0.0), (0.0, 0.0))),
        ];

        let (_, optimized) =
            finish_potrace_segments((0.0, 0.0), segments, 0.2, STRICT_POTRACE_LINEAR_DEVIATION);
        let cubic_count = optimized
            .iter()
            .filter(|segment| matches!(segment, SvgPathSegment::Cubic(_)))
            .count();
        let line_count = optimized
            .iter()
            .filter(|segment| matches!(segment, SvgPathSegment::Line { .. }))
            .count();

        assert_eq!(cubic_count, 2, "{optimized:?}");
        assert_eq!(line_count, 2, "{optimized:?}");
    }

    #[test]
    fn bezier_tangent_parameter_handles_linear_degenerate_case() {
        let cubic = CubicSegment {
            start: (0.0, 0.0),
            control1: (1.0, 1.0),
            control2: (2.0, 1.0),
            end: (3.0, 0.0),
        };

        let parameter = bezier_tangent_parameter(cubic, (0.0, 0.0), (1.0, 0.0))
            .expect("linear tangent equation should have an in-range solution");

        assert!((parameter - 0.5).abs() <= 1.0e-9);
    }

    #[test]
    fn regularize_potrace_orthogonal_corner_uses_tangent_controls() {
        let segments = vec![
            SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (100.0, 0.0))),
            SvgPathSegment::Cubic(CubicSegment {
                start: (100.0, 0.0),
                control1: (104.0, 0.2),
                control2: (109.8, 5.5),
                end: (110.0, 10.0),
            }),
            SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
            SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (40.0, 90.0))),
            SvgPathSegment::Cubic(test_cubic((40.0, 90.0), (0.0, 0.0))),
        ];

        let regularized = regularize_potrace_orthogonal_corners(segments);
        let SvgPathSegment::Cubic(corner) = regularized[1] else {
            panic!("corner should remain cubic: {regularized:?}");
        };

        assert_eq!(regularized.len(), 5);
        assert!(
            (corner.control1.1 - corner.start.1).abs() <= 1.0e-6,
            "{corner:?}"
        );
        assert!(
            (corner.control2.0 - corner.end.0).abs() <= 1.0e-6,
            "{corner:?}"
        );
    }

    #[test]
    fn regularize_potrace_orthogonal_corner_merges_straight_lead_in() {
        let segments = vec![
            SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (100.0, 0.0))),
            SvgPathSegment::Cubic(test_cubic((100.0, 0.0), (120.0, 0.0))),
            SvgPathSegment::Cubic(CubicSegment {
                start: (120.0, 0.0),
                control1: (124.0, 0.5),
                control2: (130.0, 6.0),
                end: (130.0, 12.0),
            }),
            SvgPathSegment::Cubic(test_cubic((130.0, 12.0), (130.0, 92.0))),
            SvgPathSegment::Cubic(test_cubic((130.0, 92.0), (0.0, 92.0))),
        ];

        let regularized = regularize_potrace_orthogonal_corners(segments);
        let SvgPathSegment::Cubic(corner) = regularized[1] else {
            panic!("merged corner should be cubic: {regularized:?}");
        };

        assert_eq!(regularized.len(), 4);
        assert_eq!(corner.start, (100.0, 0.0));
        assert_eq!(corner.end, (130.0, 12.0));
    }

    #[test]
    fn regularize_potrace_orthogonal_corner_rejects_beveled_turn() {
        let bevel = test_cubic((100.0, 0.0), (110.0, 10.0));
        let segments = vec![
            SvgPathSegment::Cubic(test_cubic((0.0, 0.0), (100.0, 0.0))),
            SvgPathSegment::Cubic(bevel),
            SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
            SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (40.0, 90.0))),
            SvgPathSegment::Cubic(test_cubic((40.0, 90.0), (0.0, 0.0))),
        ];

        let regularized = regularize_potrace_orthogonal_corners(segments);
        let SvgPathSegment::Cubic(unchanged) = regularized[1] else {
            panic!("bevel should remain cubic: {regularized:?}");
        };

        assert_eq!(regularized.len(), 5);
        assert_eq!(unchanged.control1, bevel.control1);
        assert_eq!(unchanged.control2, bevel.control2);
    }

    #[test]
    fn regularize_potrace_orthogonal_corner_ignores_mixed_line_boundaries() {
        let corner = CubicSegment {
            start: (100.0, 0.0),
            control1: (104.0, 0.2),
            control2: (109.8, 5.5),
            end: (110.0, 10.0),
        };
        let segments = vec![
            SvgPathSegment::Line {
                start: (0.0, 0.0),
                end: (100.0, 0.0),
            },
            SvgPathSegment::Cubic(corner),
            SvgPathSegment::Cubic(test_cubic((110.0, 10.0), (110.0, 90.0))),
            SvgPathSegment::Cubic(test_cubic((110.0, 90.0), (40.0, 90.0))),
            SvgPathSegment::Cubic(test_cubic((40.0, 90.0), (0.0, 0.0))),
        ];

        let regularized = regularize_potrace_orthogonal_corners(segments);
        let SvgPathSegment::Cubic(unchanged) = regularized[1] else {
            panic!("mixed line boundary should keep the corner cubic: {regularized:?}");
        };

        assert_eq!(regularized.len(), 5);
        assert_eq!(unchanged.control1, corner.control1);
        assert_eq!(unchanged.control2, corner.control2);
    }

    fn test_cubic(start: (f64, f64), end: (f64, f64)) -> CubicSegment {
        CubicSegment {
            start,
            control1: (
                start.0 + (end.0 - start.0) / 3.0,
                start.1 + (end.1 - start.1) / 3.0,
            ),
            control2: (
                start.0 + (end.0 - start.0) * 2.0 / 3.0,
                start.1 + (end.1 - start.1) * 2.0 / 3.0,
            ),
            end,
        }
    }

    fn test_icon_candidate(
        foreground_error_ratio: f64,
        score: f64,
        point_count: usize,
        svg_command_count: usize,
    ) -> IconOptimizationCandidate {
        IconOptimizationCandidate {
            trace_options: TraceOptions::default(),
            metrics: IconDiffMetrics {
                total_pixels: 1000,
                target_foreground_pixels: 1000,
                candidate_foreground_pixels: 1000,
                true_positive_pixels: 1000,
                false_positive_pixels: 0,
                false_negative_pixels: 0,
                xor_pixels: 0,
                xor_ratio: foreground_error_ratio,
                foreground_error_ratio,
                false_positive_ratio: 0.0,
                false_negative_ratio: 0.0,
                precision: 1.0,
                recall: 1.0,
                iou: 1.0,
            },
            score,
            path_count: 1,
            point_count,
            svg_command_count,
        }
    }
}
