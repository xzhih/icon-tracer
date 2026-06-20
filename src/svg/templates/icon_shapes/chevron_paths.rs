use super::path_specs::segments_from_specs;
use super::*;

mod down;
mod left;
mod right;
mod up;

pub(crate) fn chevron_down_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, down::SEGMENTS)
}

pub(crate) fn chevron_up_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, up::SEGMENTS)
}

pub(crate) fn chevron_right_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, right::SEGMENTS)
}

pub(crate) fn chevron_left_potrace_segments(bounds: FloatBounds) -> Vec<SvgPathSegment> {
    segments_from_specs(bounds, left::SEGMENTS)
}
