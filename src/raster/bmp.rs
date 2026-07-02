use super::*;

pub(super) fn parse_bmp(bytes: &[u8], options: RasterOptions) -> Result<Bitmap, BmpError> {
    BmpParser::new(bytes).parse(options)
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
