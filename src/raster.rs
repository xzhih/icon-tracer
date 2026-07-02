use std::fmt;
use std::io::Cursor;

use crate::BinaryMask;

mod bmp;
mod pnm;

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
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) samples: Vec<u8>,
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
        pnm::parse_pnm(bytes, options)
    }

    pub fn from_bmp(bytes: &[u8], options: RasterOptions) -> Result<Self, BmpError> {
        bmp::parse_bmp(bytes, options)
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
    pub(crate) fn resolve(self, samples: &[u8]) -> u8 {
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

pub(crate) fn apply_invert(black: bool, options: RasterOptions) -> bool {
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

pub(crate) fn otsu_threshold(samples: &[u8]) -> u8 {
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

pub(crate) fn composite_over_background(color: u8, alpha: u8, background: AlphaBackground) -> u32 {
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
