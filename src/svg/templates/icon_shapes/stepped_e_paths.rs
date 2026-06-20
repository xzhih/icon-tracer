use super::path_specs::segments_from_specs;
use super::*;

mod down;
mod left;
mod right;
mod up;

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
