use std::fmt;
use std::io::Cursor;

use crate::BinaryMask;

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
