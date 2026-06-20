use super::path_specs::segments_from_specs;
use super::*;

mod base;
mod mx;
mod my;
mod r180;
mod r270;
mod r90;

pub(crate) fn h_base_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, base::SEGMENTS)
}

pub(crate) fn h_mx_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, mx::SEGMENTS)
}

pub(crate) fn h_my_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, my::SEGMENTS)
}

pub(crate) fn h_r90_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, r90::SEGMENTS)
}

pub(crate) fn h_r180_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, r180::SEGMENTS)
}

pub(crate) fn h_r270_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, r270::SEGMENTS)
}
