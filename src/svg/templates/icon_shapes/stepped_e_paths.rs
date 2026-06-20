use super::path_specs::segments_from_specs;
use super::*;

mod down;
mod left;
mod right;
mod thin;
mod up;
mod wide;

pub(crate) fn stepped_e_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, right::SEGMENTS)
}

pub(crate) fn stepped_e_left_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, left::SEGMENTS)
}

pub(crate) fn stepped_e_down_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, down::SEGMENTS)
}

pub(crate) fn stepped_e_up_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, up::SEGMENTS)
}

pub(crate) fn stepped_e_wide_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, wide::SEGMENTS)
}

pub(crate) fn stepped_e_thin_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, thin::SEGMENTS)
}
