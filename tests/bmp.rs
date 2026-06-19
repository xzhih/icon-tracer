use icon_tracer::{Bitmap, RasterOptions};

#[test]
fn parses_bottom_up_24_bit_bmp() {
    let bitmap =
        Bitmap::from_bytes(&bmp_24_bit_2x2(), RasterOptions::default()).expect("BMP should parse");

    assert_eq!(bitmap.width(), 2);
    assert_eq!(bitmap.height(), 2);
    assert!(bitmap.is_black(0, 0));
    assert!(!bitmap.is_black(1, 0));
    assert!(!bitmap.is_black(0, 1));
    assert!(!bitmap.is_black(1, 1));
}

#[test]
fn parses_top_down_32_bit_bmp() {
    let bitmap = Bitmap::from_bytes(&bmp_32_bit_top_down_2x1(), RasterOptions::default())
        .expect("BMP should parse");

    assert_eq!(bitmap.width(), 2);
    assert_eq!(bitmap.height(), 1);
    assert!(bitmap.is_black(0, 0));
    assert!(!bitmap.is_black(1, 0));
}

fn bmp_24_bit_2x2() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&70u32.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&54u32.to_le_bytes());

    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&2i32.to_le_bytes());
    bytes.extend_from_slice(&2i32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&24u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());

    bytes.extend_from_slice(&[255, 255, 255, 255, 255, 255, 0, 0]);
    bytes.extend_from_slice(&[0, 0, 0, 255, 255, 255, 0, 0]);
    bytes
}

fn bmp_32_bit_top_down_2x1() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&62u32.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&54u32.to_le_bytes());

    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&2i32.to_le_bytes());
    bytes.extend_from_slice(&(-1i32).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());

    bytes.extend_from_slice(&[0, 0, 0, 255, 255, 255, 255, 255]);
    bytes
}
