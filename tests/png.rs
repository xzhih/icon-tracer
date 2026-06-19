use icon_tracer::{AlphaBackground, Bitmap, RasterOptions, Rgba8, RgbaImage, ThresholdMode};

#[test]
fn parses_rgb_png() {
    let bitmap =
        Bitmap::from_bytes(png_2x1_rgb(), RasterOptions::default()).expect("PNG should parse");

    assert_eq!(bitmap.width(), 2);
    assert_eq!(bitmap.height(), 1);
    assert!(bitmap.is_black(0, 0));
    assert!(!bitmap.is_black(1, 0));
}

#[test]
fn composites_png_alpha_over_white_before_thresholding() {
    let bitmap =
        Bitmap::from_bytes(png_2x1_rgba(), RasterOptions::default()).expect("PNG should parse");

    assert_eq!(bitmap.width(), 2);
    assert_eq!(bitmap.height(), 1);
    assert!(!bitmap.is_black(0, 0));
    assert!(bitmap.is_black(1, 0));
}

#[test]
fn composites_png_alpha_over_black_when_requested() {
    let bitmap = Bitmap::from_bytes(
        png_2x1_rgba(),
        RasterOptions {
            threshold: ThresholdMode::Fixed(128),
            alpha_background: AlphaBackground::Black,
            ..RasterOptions::default()
        },
    )
    .expect("PNG should parse");

    assert!(bitmap.is_black(0, 0));
}

#[test]
fn raw_png_decode_preserves_rgba_pixels() {
    let png = png_2x1_color_rgba();
    let image = RgbaImage::from_png(&png).expect("PNG should parse as raw RGBA");

    assert_eq!(image.width(), 2);
    assert_eq!(image.height(), 1);
    assert_eq!(
        image.pixels(),
        &[
            Rgba8 {
                red: 12,
                green: 34,
                blue: 56,
                alpha: 78,
            },
            Rgba8 {
                red: 200,
                green: 150,
                blue: 100,
                alpha: 255,
            },
        ]
    );
}

fn png_2x1_color_rgba() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header should write");
        writer
            .write_image_data(&[12, 34, 56, 78, 200, 150, 100, 255])
            .expect("PNG data should write");
    }
    bytes
}

fn png_2x1_rgb() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x7b,
        0x40, 0xe8, 0xdd, 0x00, 0x00, 0x00, 0x0f, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60,
        0x60, 0x60, 0xf8, 0xff, 0xff, 0x3f, 0x00, 0x06, 0x01, 0x02, 0xfe, 0x02, 0xb2, 0x39, 0xae,
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

fn png_2x1_rgba() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0xf4,
        0x22, 0x7f, 0x8a, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60,
        0x80, 0x80, 0xff, 0x00, 0x01, 0x08, 0x01, 0x00, 0x4d, 0x19, 0x8f, 0x39, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}
