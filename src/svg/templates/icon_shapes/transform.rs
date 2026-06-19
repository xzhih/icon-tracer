use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) enum TemplateTransform {
    Identity,
    MirrorX,
    MirrorY,
    Rotate90,
    Rotate180,
    Rotate270,
}

pub(super) const ORTHOGONAL_TEMPLATE_TRANSFORMS: [TemplateTransform; 6] = [
    TemplateTransform::Identity,
    TemplateTransform::MirrorX,
    TemplateTransform::MirrorY,
    TemplateTransform::Rotate90,
    TemplateTransform::Rotate180,
    TemplateTransform::Rotate270,
];

pub(super) fn fit_closed_template_variants(
    points: &[(f64, f64)],
    min_axis: f64,
    min_aspect_ratio: f64,
    max_aspect_ratio: f64,
    max_template_boundary_error: f64,
    template: fn(FloatBounds) -> Vec<SvgPathSegment>,
    transforms: &[TemplateTransform],
) -> Option<Vec<SvgPathSegment>> {
    let bounds = FloatBounds::from_points(points)?;
    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;
    if width < min_axis || height < min_axis {
        return None;
    }

    let aspect = width / height;
    if !(min_aspect_ratio..=max_aspect_ratio).contains(&aspect) {
        return None;
    }

    let path = TracePath {
        points: points.to_vec(),
        is_hole: false,
    };
    for transform in transforms {
        let segments = transform_template_segments(template(bounds), bounds, *transform);
        let candidate = (segments[0].start(), segments);
        let candidate_boundary_error =
            pixel_potrace_candidate_boundary_rms_error(&path, &candidate);
        if candidate_boundary_error <= max_template_boundary_error {
            return Some(candidate.1);
        }
    }

    None
}

pub(super) fn transform_template_segments(
    segments: Vec<SvgPathSegment>,
    bounds: FloatBounds,
    transform: TemplateTransform,
) -> Vec<SvgPathSegment> {
    segments
        .into_iter()
        .map(|segment| transform_template_segment(segment, bounds, transform))
        .collect()
}

pub(super) fn transform_template_segment(
    segment: SvgPathSegment,
    bounds: FloatBounds,
    transform: TemplateTransform,
) -> SvgPathSegment {
    match segment {
        SvgPathSegment::Line { start, end } => SvgPathSegment::Line {
            start: transform_template_point(start, bounds, transform),
            end: transform_template_point(end, bounds, transform),
        },
        SvgPathSegment::Cubic(cubic) => SvgPathSegment::Cubic(CubicSegment {
            start: transform_template_point(cubic.start, bounds, transform),
            control1: transform_template_point(cubic.control1, bounds, transform),
            control2: transform_template_point(cubic.control2, bounds, transform),
            end: transform_template_point(cubic.end, bounds, transform),
        }),
    }
}

pub(super) fn transform_template_point(
    point: (f64, f64),
    bounds: FloatBounds,
    transform: TemplateTransform,
) -> (f64, f64) {
    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;
    let x = (point.0 - bounds.min_x) / width;
    let y = (point.1 - bounds.min_y) / height;
    let (x, y) = match transform {
        TemplateTransform::Identity => (x, y),
        TemplateTransform::MirrorX => (1.0 - x, y),
        TemplateTransform::MirrorY => (x, 1.0 - y),
        TemplateTransform::Rotate90 => (1.0 - y, x),
        TemplateTransform::Rotate180 => (1.0 - x, 1.0 - y),
        TemplateTransform::Rotate270 => (y, 1.0 - x),
    };

    normalized_rect_point(bounds, (x, y))
}
