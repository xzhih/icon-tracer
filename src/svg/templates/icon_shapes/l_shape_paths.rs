use super::path_specs::segments_from_specs;
use super::*;

mod base;
mod fat;
mod mx;
mod my;
mod r180;
mod r270;
mod r90;
mod thin;
mod wide;

pub(crate) fn l_base_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, base::SEGMENTS)
}

pub(crate) fn l_mx_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, mx::SEGMENTS)
}

pub(crate) fn l_my_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, my::SEGMENTS)
}

pub(crate) fn l_r90_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, r90::SEGMENTS)
}

pub(crate) fn l_r180_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, r180::SEGMENTS)
}

pub(crate) fn l_r270_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, r270::SEGMENTS)
}

pub(crate) fn l_wide_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, wide::SEGMENTS)
}

pub(crate) fn l_fat_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, fat::SEGMENTS)
}

pub(crate) fn l_thin_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, thin::SEGMENTS)
}
