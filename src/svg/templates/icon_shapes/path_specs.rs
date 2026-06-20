use super::*;

#[derive(Clone, Copy)]
pub(super) enum SegmentSpec {
    Line((f64, f64), (f64, f64)),
    Cubic((f64, f64), (f64, f64), (f64, f64), (f64, f64)),
}

pub(super) fn segments_from_specs(
    bounds: FloatBounds,
    specs: &[SegmentSpec],
) -> Vec<SvgPathSegment> {
    specs
        .iter()
        .map(|spec| match *spec {
            SegmentSpec::Line(start, end) => normalized_rect_line(bounds, start, end),
            SegmentSpec::Cubic(start, control1, control2, end) => {
                normalized_rect_cubic(bounds, start, control1, control2, end)
            }
        })
        .collect()
}
