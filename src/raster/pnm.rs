use super::*;

pub(super) fn parse_pnm(bytes: &[u8], options: RasterOptions) -> Result<Bitmap, PnmError> {
    PnmParser::new(bytes).parse(options)
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
