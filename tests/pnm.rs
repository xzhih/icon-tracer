use icon_tracer::{Bitmap, RasterOptions, ThresholdMode};

#[test]
fn parses_ascii_pbm_with_comments() {
    let bitmap = Bitmap::from_pnm(
        b"P1\n# one black pixel in the middle\n3 3\n0 0 0\n0 1 0\n0 0 0\n",
        RasterOptions::default(),
    )
    .expect("PBM should parse");

    assert_eq!(bitmap.width(), 3);
    assert_eq!(bitmap.height(), 3);
    assert!(!bitmap.is_black(0, 0));
    assert!(bitmap.is_black(1, 1));
}

#[test]
fn parses_binary_pgm_with_threshold_and_invert() {
    let bitmap = Bitmap::from_pnm(
        b"P5\n2 1\n255\n\x00\xff",
        RasterOptions {
            threshold: ThresholdMode::Fixed(128),
            invert: true,
            ..RasterOptions::default()
        },
    )
    .expect("PGM should parse");

    assert!(!bitmap.is_black(0, 0));
    assert!(bitmap.is_black(1, 0));
}

#[test]
fn auto_threshold_classifies_dark_and_light_gray_groups() {
    let bitmap = Bitmap::from_pnm(b"P2\n4 1\n255\n10 20 220 230\n", RasterOptions::default())
        .expect("PGM should parse");

    assert!(bitmap.is_black(0, 0));
    assert!(bitmap.is_black(1, 0));
    assert!(!bitmap.is_black(2, 0));
    assert!(!bitmap.is_black(3, 0));
}

#[test]
fn auto_threshold_handles_uniform_black_and_white_images() {
    let black = Bitmap::from_pnm(b"P2\n1 1\n255\n0\n", RasterOptions::default())
        .expect("black PGM should parse");
    let white = Bitmap::from_pnm(b"P2\n1 1\n255\n255\n", RasterOptions::default())
        .expect("white PGM should parse");

    assert!(black.is_black(0, 0));
    assert!(!white.is_black(0, 0));
}

#[test]
fn parses_binary_pbm_bits_by_row() {
    let bitmap =
        Bitmap::from_pnm(b"P4\n3 2\n\xa0\x40", RasterOptions::default()).expect("PBM should parse");

    assert!(bitmap.is_black(0, 0));
    assert!(!bitmap.is_black(1, 0));
    assert!(bitmap.is_black(2, 0));
    assert!(!bitmap.is_black(0, 1));
    assert!(bitmap.is_black(1, 1));
    assert!(!bitmap.is_black(2, 1));
}

#[test]
fn parses_binary_ppm_by_luma_threshold() {
    let bitmap = Bitmap::from_pnm(
        b"P6\n2 1\n255\n\x00\x00\x00\xff\xff\xff",
        RasterOptions::default(),
    )
    .expect("PPM should parse");

    assert!(bitmap.is_black(0, 0));
    assert!(!bitmap.is_black(1, 0));
}
