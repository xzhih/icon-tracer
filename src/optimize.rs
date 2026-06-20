use crate::components::{connected_components, RawComponent};
use crate::raster::{composite_over_background, otsu_threshold};
use crate::svg::path_to_svg_data_with_context;
use crate::{
    trace_bitmap, trace_scalar_field, AlphaBackground, BinaryMask, BitmapError, ContourMode,
    IconDiffMetrics, IconOptimizationCandidate, IconOptimizationResult, IconOptimizeOptions,
    RasterOptions, Rgba8, RgbaImage, ScalarField, SvgOptions, SvgRenderOptions, ThresholdMode,
    TraceOptions, TracedBitmap,
};

pub fn compare_icon_masks(
    target: &BinaryMask,
    candidate: &BinaryMask,
) -> Result<IconDiffMetrics, BitmapError> {
    if target.width != candidate.width || target.height != candidate.height {
        return Err(BitmapError::DimensionMismatch {
            width: target.width,
            height: target.height,
            pixels: candidate.pixels.len(),
        });
    }

    let mut target_foreground_pixels = 0usize;
    let mut candidate_foreground_pixels = 0usize;
    let mut true_positive_pixels = 0usize;
    let mut false_positive_pixels = 0usize;
    let mut false_negative_pixels = 0usize;

    for (target_pixel, candidate_pixel) in target.pixels.iter().zip(&candidate.pixels) {
        if *target_pixel {
            target_foreground_pixels += 1;
        }

        if *candidate_pixel {
            candidate_foreground_pixels += 1;
        }

        match (*target_pixel, *candidate_pixel) {
            (true, true) => true_positive_pixels += 1,
            (false, true) => false_positive_pixels += 1,
            (true, false) => false_negative_pixels += 1,
            (false, false) => {}
        }
    }

    let total_pixels = target.pixels.len();
    let xor_pixels = false_positive_pixels + false_negative_pixels;
    let union_pixels = true_positive_pixels + false_positive_pixels + false_negative_pixels;
    let predicted_pixels = true_positive_pixels + false_positive_pixels;
    let target_positive_pixels = true_positive_pixels + false_negative_pixels;

    Ok(IconDiffMetrics {
        total_pixels,
        target_foreground_pixels,
        candidate_foreground_pixels,
        true_positive_pixels,
        false_positive_pixels,
        false_negative_pixels,
        xor_pixels,
        xor_ratio: ratio(xor_pixels, total_pixels),
        foreground_error_ratio: ratio(xor_pixels, target_foreground_pixels.max(1)),
        false_positive_ratio: ratio(false_positive_pixels, total_pixels),
        false_negative_ratio: ratio(false_negative_pixels, target_foreground_pixels.max(1)),
        precision: if predicted_pixels == 0 {
            1.0
        } else {
            ratio(true_positive_pixels, predicted_pixels)
        },
        recall: if target_positive_pixels == 0 {
            1.0
        } else {
            ratio(true_positive_pixels, target_positive_pixels)
        },
        iou: if union_pixels == 0 {
            1.0
        } else {
            ratio(true_positive_pixels, union_pixels)
        },
    })
}

pub fn optimize_icon_trace(
    image: &RgbaImage,
    options: IconOptimizeOptions,
) -> Result<IconOptimizationResult, BitmapError> {
    let source_scalar_field = image.to_scalar_field(options.raster_options.alpha_background);
    let source_threshold = options
        .raster_options
        .threshold
        .resolve(source_scalar_field.samples());
    let source_bitmap = image.to_bitmap(options.raster_options)?;
    let source_mask = source_bitmap.as_mask();
    let target_mask = if options.isolate_foreground {
        isolate_icon_foreground_mask(image, options.raster_options, &source_mask)
    } else {
        source_mask.clone()
    };
    let target_bitmap = target_mask.to_bitmap();
    let foreground_isolated = target_mask != source_mask;
    let scalar_field = if foreground_isolated {
        masked_scalar_field(&source_scalar_field, &target_mask, options.raster_options)
    } else {
        source_scalar_field.clone()
    };
    let scalar_raster_options = if foreground_isolated {
        RasterOptions {
            threshold: ThresholdMode::Fixed(source_threshold),
            ..options.raster_options
        }
    } else {
        options.raster_options
    };
    let contour_modes = if options.contour_modes.is_empty() {
        vec![options.trace_options.contour_mode]
    } else {
        options.contour_modes.clone()
    };
    let opt_tolerances = if options.opt_tolerances.is_empty() {
        vec![options.trace_options.opt_tolerance]
    } else {
        options.opt_tolerances.clone()
    };

    let mut evaluated_candidates = Vec::new();

    for contour_mode in contour_modes {
        for opt_tolerance in &opt_tolerances {
            let trace_options = TraceOptions {
                contour_mode,
                opt_tolerance: opt_tolerance.max(0.0),
                ..options.trace_options
            };
            let traced = if contour_mode == ContourMode::Scalar {
                trace_scalar_field(&scalar_field, scalar_raster_options, trace_options)?
            } else {
                trace_bitmap(&target_bitmap, trace_options)
            };
            let candidate_mask = traced.to_mask();
            let metrics = compare_icon_masks(&target_mask, &candidate_mask)?;
            let path_count = traced.paths.len();
            let point_count = traced_point_count(&traced);
            let svg_command_count = traced_svg_command_count(&traced, options.svg_options);
            let score = icon_candidate_score(
                metrics,
                point_count,
                svg_command_count,
                options.complexity_weight,
            );
            let candidate = IconOptimizationCandidate {
                trace_options,
                metrics,
                score,
                path_count,
                point_count,
                svg_command_count,
            };

            evaluated_candidates.push((candidate, traced));
        }
    }

    let candidates = evaluated_candidates
        .iter()
        .map(|(candidate, _)| candidate.clone())
        .collect::<Vec<_>>();
    let best_index = best_icon_candidate_index(&candidates)
        .expect("optimizer should evaluate at least one candidate");
    let best_candidate = candidates[best_index].clone();
    let best_traced = evaluated_candidates[best_index].1.clone();

    Ok(IconOptimizationResult {
        traced: best_traced,
        svg_options: options.svg_options,
        best_candidate,
        candidates,
    })
}

impl IconOptimizationResult {
    pub fn to_svg(&self) -> String {
        self.traced.to_svg_with_options(self.svg_options)
    }
}

fn isolate_icon_foreground_mask(
    image: &RgbaImage,
    raster_options: RasterOptions,
    fallback: &BinaryMask,
) -> BinaryMask {
    let edge_pruned_fallback = prune_edge_touching_foreground(fallback);
    let pruned_fallback = prune_peripheral_foreground_noise(&edge_pruned_fallback, fallback);
    let fallback_area = mask_area(fallback);
    let pruned_fallback_area = mask_area(&pruned_fallback);
    let fallback_ratio = foreground_ratio(fallback_area, fallback.pixels.len());

    if mask_is_plausible_foreground(&pruned_fallback, fallback)
        && (fallback_ratio > 0.35
            || pruned_fallback_area * 10 < fallback_area * 9
            || edge_pruned_foreground_drop_is_useful(
                fallback_area,
                pruned_fallback_area,
                fallback.pixels.len(),
            ))
    {
        return pruned_fallback;
    }

    let contrast = background_contrast_mask(image, raster_options.alpha_background);
    let edge_pruned_contrast = prune_edge_touching_foreground(&contrast);
    let pruned_contrast = prune_peripheral_foreground_noise(&edge_pruned_contrast, &contrast);
    let contrast_area = mask_area(&pruned_contrast);
    if mask_is_plausible_foreground(&pruned_contrast, fallback)
        && (fallback_ratio > 0.35
            || contrast_area * 4 < fallback_area.saturating_mul(3)
            || !component_area_is_plausible(fallback_area, fallback.pixels.len()))
    {
        return pruned_contrast;
    }

    fallback.clone()
}

fn background_contrast_mask(image: &RgbaImage, alpha_background: AlphaBackground) -> BinaryMask {
    let background = estimate_border_rgb(image, alpha_background);
    let distances = image
        .pixels()
        .iter()
        .map(|pixel| {
            let color = composited_rgb(pixel, alpha_background);
            rgb_distance_sample(color, background)
        })
        .collect::<Vec<_>>();
    let threshold = otsu_threshold(&distances).max(18);
    let pixels = distances
        .iter()
        .map(|distance| *distance > threshold)
        .collect::<Vec<_>>();

    BinaryMask {
        width: image.width(),
        height: image.height(),
        pixels,
    }
}

fn estimate_border_rgb(image: &RgbaImage, alpha_background: AlphaBackground) -> (f64, f64, f64) {
    let mut count = 0usize;
    let mut sum = (0.0, 0.0, 0.0);

    for y in 0..image.height() {
        for x in 0..image.width() {
            if x != 0 && y != 0 && x + 1 != image.width() && y + 1 != image.height() {
                continue;
            }

            let color = composited_rgb(&image.pixel(x, y), alpha_background);
            sum.0 += color.0;
            sum.1 += color.1;
            sum.2 += color.2;
            count += 1;
        }
    }

    if count == 0 {
        (0.0, 0.0, 0.0)
    } else {
        (
            sum.0 / count as f64,
            sum.1 / count as f64,
            sum.2 / count as f64,
        )
    }
}

fn composited_rgb(pixel: &Rgba8, alpha_background: AlphaBackground) -> (f64, f64, f64) {
    (
        f64::from(composite_over_background(
            pixel.red,
            pixel.alpha,
            alpha_background,
        )),
        f64::from(composite_over_background(
            pixel.green,
            pixel.alpha,
            alpha_background,
        )),
        f64::from(composite_over_background(
            pixel.blue,
            pixel.alpha,
            alpha_background,
        )),
    )
}

fn rgb_distance_sample(color: (f64, f64, f64), background: (f64, f64, f64)) -> u8 {
    let distance = ((color.0 - background.0).powi(2)
        + (color.1 - background.1).powi(2)
        + (color.2 - background.2).powi(2))
    .sqrt();

    ((distance / (3.0_f64.sqrt() * 255.0)) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn prune_edge_touching_foreground(mask: &BinaryMask) -> BinaryMask {
    let components = connected_components(mask, 1);
    let interior_area = components
        .iter()
        .filter(|component| !raw_component_touches_edge(component, mask.width, mask.height))
        .map(|component| component.pixels.len())
        .sum::<usize>();

    if !component_area_is_plausible(interior_area, mask.pixels.len()) {
        return mask.clone();
    }

    let mut pixels = vec![false; mask.pixels.len()];
    for component in components {
        if raw_component_touches_edge(&component, mask.width, mask.height) {
            continue;
        }

        for index in component.pixels {
            pixels[index] = true;
        }
    }

    BinaryMask {
        width: mask.width,
        height: mask.height,
        pixels,
    }
}

fn prune_peripheral_foreground_noise(mask: &BinaryMask, original: &BinaryMask) -> BinaryMask {
    let components = connected_components(mask, 1);
    if components.len() <= 1 {
        return mask.clone();
    }

    let original_area = mask_area(original);
    let mask_area = mask_area(mask);
    if mask_area >= original_area {
        return mask.clone();
    }

    let largest_area = components[0].pixels.len();
    if !component_area_is_plausible(largest_area, mask.pixels.len()) {
        return mask.clone();
    }

    let corner_band = (mask.width.min(mask.height) / 10).max(2);
    let canvas_noise_cap = ((mask.pixels.len() as f64 * 0.0001).round() as usize).clamp(4, 128);
    let foreground_noise_cap = (largest_area / 500).max(4);
    let max_noise_area = canvas_noise_cap.min(foreground_noise_cap);
    let mut pixels = vec![false; mask.pixels.len()];
    let mut kept_area = 0usize;

    for component in components {
        if component.pixels.len() <= max_noise_area {
            if let Some(corner) =
                raw_component_corner_band(&component, mask.width, mask.height, corner_band)
            {
                if original_has_edge_residue_in_corner(original, corner_band, corner) {
                    continue;
                }
            }
        }

        kept_area += component.pixels.len();
        for index in component.pixels {
            pixels[index] = true;
        }
    }

    if !component_area_is_plausible(kept_area, mask.pixels.len()) {
        return mask.clone();
    }

    BinaryMask {
        width: mask.width,
        height: mask.height,
        pixels,
    }
}

fn raw_component_touches_edge(component: &RawComponent, width: usize, height: usize) -> bool {
    component.min_x == 0
        || component.min_y == 0
        || component.max_x + 1 == width
        || component.max_y + 1 == height
}

fn raw_component_corner_band(
    component: &RawComponent,
    width: usize,
    height: usize,
    band: usize,
) -> Option<(bool, bool)> {
    let near_left = component.max_x < band;
    let near_right = component.min_x.saturating_add(band) >= width;
    let near_top = component.max_y < band;
    let near_bottom = component.min_y.saturating_add(band) >= height;

    match (near_left, near_right, near_top, near_bottom) {
        (true, false, true, false) => Some((true, true)),
        (true, false, false, true) => Some((true, false)),
        (false, true, true, false) => Some((false, true)),
        (false, true, false, true) => Some((false, false)),
        _ => None,
    }
}

fn original_has_edge_residue_in_corner(
    original: &BinaryMask,
    band: usize,
    corner: (bool, bool),
) -> bool {
    connected_components(original, 1).iter().any(|component| {
        raw_component_touches_edge(component, original.width, original.height)
            && raw_component_intersects_corner_band(
                component,
                original.width,
                original.height,
                band,
                corner,
            )
    })
}

fn raw_component_intersects_corner_band(
    component: &RawComponent,
    width: usize,
    height: usize,
    band: usize,
    corner: (bool, bool),
) -> bool {
    let horizontal = if corner.0 {
        component.min_x < band
    } else {
        component.max_x.saturating_add(band) >= width
    };
    let vertical = if corner.1 {
        component.min_y < band
    } else {
        component.max_y.saturating_add(band) >= height
    };

    horizontal && vertical
}

fn mask_is_plausible_foreground(mask: &BinaryMask, fallback: &BinaryMask) -> bool {
    let area = mask_area(mask);
    let fallback_area = mask_area(fallback);

    component_area_is_plausible(area, mask.pixels.len())
        && (area <= fallback_area.saturating_mul(2).max(1)
            || foreground_ratio(fallback_area, mask.pixels.len()) > 0.35)
}

fn edge_pruned_foreground_drop_is_useful(
    fallback_area: usize,
    pruned_area: usize,
    total_pixels: usize,
) -> bool {
    if pruned_area >= fallback_area {
        return false;
    }

    let removed_area = fallback_area - pruned_area;
    let min_removed_from_canvas = ((total_pixels as f64 * 0.001).round() as usize).max(4);
    let min_removed_from_foreground = ((fallback_area as f64 * 0.02).round() as usize).max(4);
    let max_removed_from_foreground = ((fallback_area as f64 * 0.25).round() as usize).max(4);

    removed_area >= min_removed_from_canvas.max(min_removed_from_foreground)
        && removed_area <= max_removed_from_foreground
}

fn component_area_is_plausible(area: usize, total: usize) -> bool {
    if total == 0 || area == 0 {
        return false;
    }

    let ratio = foreground_ratio(area, total);
    area >= ((total as f64 * 0.005).round() as usize).max(4) && ratio <= 0.70
}

fn foreground_ratio(area: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        area as f64 / total as f64
    }
}

fn mask_area(mask: &BinaryMask) -> usize {
    mask.pixels.iter().filter(|pixel| **pixel).count()
}

fn masked_scalar_field(
    field: &ScalarField,
    mask: &BinaryMask,
    raster_options: RasterOptions,
) -> ScalarField {
    let background_sample = if raster_options.invert { 0 } else { 255 };
    let samples = field
        .samples()
        .iter()
        .zip(mask.pixels())
        .map(|(sample, keep)| if *keep { *sample } else { background_sample })
        .collect::<Vec<_>>();

    ScalarField {
        width: field.width(),
        height: field.height(),
        samples,
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn traced_point_count(traced: &TracedBitmap) -> usize {
    traced.paths.iter().map(|path| path.points.len()).sum()
}

fn traced_svg_command_count(traced: &TracedBitmap, options: SvgOptions) -> usize {
    let options = SvgRenderOptions::from(options);
    let has_holes = traced.paths.iter().any(|path| path.is_hole);
    let has_sibling_paths = traced.paths.len() > 1;
    traced
        .paths
        .iter()
        .filter_map(|path| {
            path_to_svg_data_with_context(path, options, None, has_holes, has_sibling_paths)
        })
        .map(|path_data| {
            path_data
                .split_whitespace()
                .filter(|token| matches!(*token, "M" | "L" | "C" | "Q" | "Z"))
                .count()
        })
        .sum()
}

const ICON_COMPLEXITY_FIT_SCORE_BAND: f64 = 0.002;

fn icon_candidate_fit_score(metrics: IconDiffMetrics) -> f64 {
    metrics.foreground_error_ratio + metrics.false_negative_ratio * 0.5 + (1.0 - metrics.iou) * 0.25
}

fn icon_candidate_complexity_score(
    metrics: IconDiffMetrics,
    point_count: usize,
    svg_command_count: usize,
    complexity_weight: f64,
) -> f64 {
    let target_foreground_pixels = metrics.target_foreground_pixels.max(1);
    let complexity_ratio = ratio(point_count, target_foreground_pixels)
        + ratio(svg_command_count, target_foreground_pixels) * 2.0;

    complexity_weight.max(0.0) * complexity_ratio
}

fn icon_candidate_score(
    metrics: IconDiffMetrics,
    point_count: usize,
    svg_command_count: usize,
    complexity_weight: f64,
) -> f64 {
    icon_candidate_fit_score(metrics)
        + icon_candidate_complexity_score(
            metrics,
            point_count,
            svg_command_count,
            complexity_weight,
        )
}

pub(crate) fn best_icon_candidate_index(candidates: &[IconOptimizationCandidate]) -> Option<usize> {
    let min_fit_score = candidates
        .iter()
        .map(|candidate| icon_candidate_fit_score(candidate.metrics))
        .min_by(f64::total_cmp)?;

    candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            icon_candidate_fit_score(candidate.metrics)
                <= min_fit_score + ICON_COMPLEXITY_FIT_SCORE_BAND
        })
        .min_by(|(_, left), (_, right)| compare_eligible_icon_candidates(left, right))
        .map(|(index, _)| index)
}

fn compare_eligible_icon_candidates(
    left: &IconOptimizationCandidate,
    right: &IconOptimizationCandidate,
) -> std::cmp::Ordering {
    left.score
        .total_cmp(&right.score)
        .then_with(|| left.svg_command_count.cmp(&right.svg_command_count))
        .then_with(|| left.point_count.cmp(&right.point_count))
}
