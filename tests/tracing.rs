use icon_tracer::{
    compare_icon_masks, optimize_icon_trace, trace_bitmap, trace_scalar_field, AlphaBackground,
    BinaryMask, Bitmap, ContourMode, CurveMode, IconOptimizeOptions, RasterOptions, Rgba8,
    RgbaImage, ScalarField, SvgOptions, ThresholdMode, TraceOptions, TracePath, TracedBitmap,
};

#[test]
fn traces_single_pixel_as_clockwise_square() {
    let bitmap = Bitmap::from_rows(
        3,
        3,
        &[false, false, false, false, true, false, false, false, false],
    )
    .expect("bitmap dimensions should match");

    let traced = trace_bitmap(&bitmap, TraceOptions::default());

    assert_eq!(traced.paths.len(), 1);
    assert_eq!(
        traced.paths[0].points,
        vec![(1.0, 1.0), (2.0, 1.0), (2.0, 2.0), (1.0, 2.0)]
    );
    assert!(!traced.paths[0].is_hole);
}

#[test]
fn subpixel_contour_traces_single_pixel_as_centered_diamond() {
    let bitmap = Bitmap::from_rows(1, 1, &[true]).expect("bitmap dimensions should match");

    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            contour_mode: ContourMode::Subpixel,
            ..TraceOptions::default()
        },
    );

    assert_eq!(traced.paths.len(), 1);
    assert_eq!(
        traced.paths[0].points,
        vec![(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)]
    );
    assert!(!traced.paths[0].is_hole);
}

#[test]
fn scalar_contour_interpolates_threshold_crossing() {
    let field = ScalarField::from_rows(2, 1, &[0, 192]).expect("field dimensions should match");
    let traced = trace_scalar_field(
        &field,
        RasterOptions {
            threshold: ThresholdMode::Fixed(128),
            ..RasterOptions::default()
        },
        TraceOptions {
            contour_mode: ContourMode::Scalar,
            ..TraceOptions::default()
        },
    )
    .expect("scalar trace should succeed");

    assert_eq!(traced.paths.len(), 1);
    assert!(
        traced.paths[0]
            .points
            .iter()
            .any(|point| (point.0 - 1.166667).abs() < 0.00001),
        "expected interpolated threshold crossing in {:?}",
        traced.paths[0].points
    );
}

#[test]
fn subpixel_contour_uses_opt_tolerance_to_reduce_stair_steps() {
    let bitmap = Bitmap::from_rows(
        5,
        5,
        &[
            true, false, false, false, false, true, true, false, false, false, true, true, true,
            false, false, true, true, true, true, false, true, true, true, true, true,
        ],
    )
    .expect("bitmap dimensions should match");

    let default_trace = trace_bitmap(
        &bitmap,
        TraceOptions {
            contour_mode: ContourMode::Subpixel,
            ..TraceOptions::default()
        },
    );
    let optimized_trace = trace_bitmap(
        &bitmap,
        TraceOptions {
            contour_mode: ContourMode::Subpixel,
            opt_tolerance: 0.75,
            ..TraceOptions::default()
        },
    );

    assert_eq!(default_trace.paths.len(), 1);
    assert_eq!(optimized_trace.paths.len(), 1);
    assert!(optimized_trace.paths[0].points.len() < default_trace.paths[0].points.len());
}

#[test]
fn adjacent_black_pixels_share_edges() {
    let bitmap = Bitmap::from_rows(3, 2, &[true, true, false, false, false, false])
        .expect("bitmap dimensions should match");

    let traced = trace_bitmap(&bitmap, TraceOptions::default());

    assert_eq!(traced.paths.len(), 1);
    assert_eq!(
        traced.paths[0].points,
        vec![(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (0.0, 1.0)]
    );
}

#[test]
fn white_island_inside_black_region_becomes_hole() {
    let bitmap = Bitmap::from_rows(
        3,
        3,
        &[true, true, true, true, false, true, true, true, true],
    )
    .expect("bitmap dimensions should match");

    let traced = trace_bitmap(&bitmap, TraceOptions::default());

    assert_eq!(traced.paths.len(), 2);
    assert_eq!(traced.paths.iter().filter(|path| !path.is_hole).count(), 1);
    assert_eq!(traced.paths.iter().filter(|path| path.is_hole).count(), 1);
}

#[test]
fn diagonal_black_pixels_are_separate_components() {
    let bitmap = Bitmap::from_rows(2, 2, &[true, false, false, true])
        .expect("bitmap dimensions should match");

    let traced = trace_bitmap(&bitmap, TraceOptions::default());

    assert_eq!(traced.paths.len(), 2);
    assert!(traced.paths.iter().all(|path| !path.is_hole));
    assert!(traced
        .paths
        .iter()
        .any(|path| path.points == vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]));
    assert!(traced
        .paths
        .iter()
        .any(|path| path.points == vec![(1.0, 1.0), (2.0, 1.0), (2.0, 2.0), (1.0, 2.0)]));
}

#[test]
fn renders_svg_with_evenodd_fill_rule() {
    let bitmap = Bitmap::from_rows(2, 2, &[true, false, false, false])
        .expect("bitmap dimensions should match");
    let traced = trace_bitmap(&bitmap, TraceOptions::default());

    let svg = traced.to_svg();

    assert!(svg.contains(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2 2""#));
    assert!(svg.contains(r#"<path fill="black" fill-rule="evenodd""#));
    assert!(svg.contains(r#"d="M 0 0 L 1 0 L 1 1 L 0 1 Z""#));
}

#[test]
fn renders_smooth_svg_with_cubic_corner_segments() {
    let bitmap =
        Bitmap::from_rows(2, 2, &[true, true, true, true]).expect("bitmap dimensions should match");
    let traced = trace_bitmap(&bitmap, TraceOptions::default());

    let svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Smooth,
    });

    assert!(svg.contains(r#"<path fill="black" fill-rule="evenodd""#));
    assert!(svg.contains(" C "));
    assert!(svg.contains(r#"d="M 0.5 0 L 1.5 0 C"#));
}

#[test]
fn renders_spline_svg_as_continuous_cubic_segments() {
    let bitmap =
        Bitmap::from_rows(2, 2, &[true, true, true, true]).expect("bitmap dimensions should match");
    let traced = trace_bitmap(&bitmap, TraceOptions::default());

    let svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Spline,
    });

    assert!(svg.contains(r#"d="M 0 0 C"#));
    assert!(svg.contains(" C "));
    assert!(!svg.contains(" L "));
}

#[test]
fn renders_fit_svg_with_bounded_cubic_controls() {
    let bitmap = Bitmap::from_rows(
        5,
        5,
        &[
            true, false, false, false, false, true, true, false, false, false, true, true, true,
            false, false, true, true, true, true, false, true, true, true, true, true,
        ],
    )
    .expect("bitmap dimensions should match");
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            opt_tolerance: 0.75,
            ..TraceOptions::default()
        },
    );

    let svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Fit,
    });

    assert!(svg.contains(r#"d="M 0 0 C"#));
    assert!(svg.contains(" C "));
    assert!(!svg.contains(" -"));
}

#[test]
fn renders_potrace_svg_with_midpoint_curves() {
    let traced = TracedBitmap {
        width: 2,
        height: 2,
        paths: vec![TracePath {
            is_hole: false,
            points: vec![(1.0, 0.0), (2.0, 1.0), (1.0, 2.0), (0.0, 1.0)],
        }],
    };

    let svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Potrace,
    });

    assert!(svg.contains(r#"d="M 0.5 0.5 C"#), "{svg}");
    assert!(count_cubic_segments(&svg) >= 4, "{svg}");
}

#[test]
fn potrace_curve_mode_keeps_long_edges_as_corners() {
    let traced = TracedBitmap {
        width: 4,
        height: 4,
        paths: vec![TracePath {
            is_hole: false,
            points: vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
        }],
    };

    let svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Potrace,
    });

    assert!(svg.contains(" L 0 0 L "), "{svg}");
}

#[test]
fn potrace_curve_mode_keeps_hairpin_turns_as_corners() {
    let traced = TracedBitmap {
        width: 4,
        height: 4,
        paths: vec![TracePath {
            is_hole: false,
            points: vec![(0.0, 0.0), (4.0, 0.0), (1.0, 0.0), (1.0, 4.0)],
        }],
    };

    let svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Potrace,
    });

    assert!(svg.contains(" L 4 0 L "), "{svg}");
}

#[test]
fn potrace_curve_mode_merges_non_quantized_smooth_closed_paths() {
    let points = (0..32)
        .map(|index| {
            let angle = index as f64 * std::f64::consts::TAU / 32.0;
            (20.0 + angle.cos() * 10.0, 20.0 + angle.sin() * 10.0)
        })
        .collect::<Vec<_>>();
    let traced = TracedBitmap {
        width: 40,
        height: 40,
        paths: vec![TracePath {
            is_hole: false,
            points,
        }],
    };

    let svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Potrace,
    });

    assert!(
        count_cubic_segments(&svg) <= 8,
        "smooth path was not merged enough: {svg}"
    );
}

#[test]
fn fit_curve_reduces_cubic_segments_for_subpixel_stair_steps() {
    let bitmap = Bitmap::from_rows(
        5,
        5,
        &[
            true, false, false, false, false, true, true, false, false, false, true, true, true,
            false, false, true, true, true, true, false, true, true, true, true, true,
        ],
    )
    .expect("bitmap dimensions should match");
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            contour_mode: ContourMode::Subpixel,
            ..TraceOptions::default()
        },
    );

    let spline_svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Spline,
    });
    let fit_svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Fit,
    });

    assert!(count_cubic_segments(&fit_svg) < count_cubic_segments(&spline_svg));
}

#[test]
fn fit_curve_keeps_samples_close_to_subpixel_stair_path() {
    let bitmap = Bitmap::from_rows(
        5,
        5,
        &[
            true, false, false, false, false, true, true, false, false, false, true, true, true,
            false, false, true, true, true, true, false, true, true, true, true, true,
        ],
    )
    .expect("bitmap dimensions should match");
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            contour_mode: ContourMode::Subpixel,
            ..TraceOptions::default()
        },
    );

    let fit_svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Fit,
    });
    let fit_segments = cubic_segments_from_svg(&fit_svg);
    let max_distance =
        max_cubic_sample_distance_to_closed_path(&fit_segments, &traced.paths[0].points);

    assert!(
        max_distance <= 0.85,
        "fit drifted {max_distance:.3} px from source path"
    );
}

#[test]
fn fit_curve_does_not_over_segment_smooth_subpixel_circle() {
    let bitmap = circle_bitmap(31, 31, 15.0, 15.0, 11.0);
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            contour_mode: ContourMode::Subpixel,
            opt_tolerance: 0.75,
            ..TraceOptions::default()
        },
    );

    assert_eq!(traced.paths.len(), 1);

    let fit_svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Fit,
    });
    let fit_segments = cubic_segments_from_svg(&fit_svg);
    let max_distance =
        max_cubic_sample_distance_to_closed_path(&fit_segments, &traced.paths[0].points);

    assert!(
        fit_segments.len() <= 4,
        "smooth circle used {} cubic segments: {fit_svg}",
        fit_segments.len()
    );
    assert!(
        max_distance <= 0.95,
        "fit drifted {max_distance:.3} px from source path"
    );
}

#[test]
fn fit_curve_does_not_over_segment_smooth_open_arc() {
    let bitmap = open_arc_bitmap(48, 48, 24.0, 24.0, 11.0, 16.0, 0.8);
    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            contour_mode: ContourMode::Subpixel,
            opt_tolerance: 0.75,
            ..TraceOptions::default()
        },
    );

    assert_eq!(traced.paths.len(), 1);

    let fit_svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Fit,
    });
    let fit_segments = cubic_segments_from_svg(&fit_svg);
    let max_distance =
        max_cubic_sample_distance_to_closed_path(&fit_segments, &traced.paths[0].points);

    assert!(
        fit_segments.len() <= 18,
        "smooth open arc used {} cubic segments: {fit_svg}",
        fit_segments.len()
    );
    assert!(
        max_distance <= 0.95,
        "fit drifted {max_distance:.3} px from source path"
    );
}

#[test]
fn fit_curve_splits_closed_paths_at_corners() {
    let bitmap =
        Bitmap::from_rows(2, 2, &[true, true, true, true]).expect("bitmap dimensions should match");
    let traced = trace_bitmap(&bitmap, TraceOptions::default());

    let svg = traced.to_svg_with_options(SvgOptions {
        curve_mode: CurveMode::Fit,
    });

    assert!(count_cubic_segments(&svg) >= 4);
}

#[test]
fn traced_bitmap_to_mask_rasterizes_evenodd_holes() {
    let traced = TracedBitmap {
        width: 4,
        height: 4,
        paths: vec![
            TracePath {
                is_hole: false,
                points: vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
            },
            TracePath {
                is_hole: true,
                points: vec![(1.0, 1.0), (3.0, 1.0), (3.0, 3.0), (1.0, 3.0)],
            },
        ],
    };

    let mask = traced.to_mask();

    assert!(mask.is_foreground(0, 0));
    assert!(!mask.is_foreground(1, 1));
    assert!(!mask.is_foreground(2, 2));
    assert!(mask.is_foreground(3, 3));
}

#[test]
fn icon_diff_metrics_report_normalized_mask_difference() {
    let target = BinaryMask::from_rows(2, 2, &[true, true, false, false])
        .expect("target mask should be valid");
    let candidate = BinaryMask::from_rows(2, 2, &[true, false, true, false])
        .expect("candidate mask should be valid");

    let metrics = compare_icon_masks(&target, &candidate).expect("masks should compare");

    assert_eq!(metrics.true_positive_pixels, 1);
    assert_eq!(metrics.false_positive_pixels, 1);
    assert_eq!(metrics.false_negative_pixels, 1);
    assert_eq!(metrics.xor_pixels, 2);
    assert_eq!(metrics.xor_ratio, 0.5);
    assert!((metrics.iou - 1.0 / 3.0).abs() < f64::EPSILON);
}

#[test]
fn optimize_icon_trace_evaluates_candidates_and_returns_best_svg() {
    let pixels = (0..4)
        .flat_map(|y| {
            (0..4).map(move |x| {
                if (1..=2).contains(&x) && (1..=2).contains(&y) {
                    rgba(0, 0, 0)
                } else {
                    rgba(255, 255, 255)
                }
            })
        })
        .collect::<Vec<_>>();
    let image = RgbaImage::from_rows(4, 4, &pixels).expect("image should be valid");

    let result = optimize_icon_trace(
        &image,
        IconOptimizeOptions {
            raster_options: RasterOptions {
                threshold: ThresholdMode::Fixed(128),
                ..RasterOptions::default()
            },
            contour_modes: vec![ContourMode::Subpixel, ContourMode::Scalar],
            opt_tolerances: vec![0.0, 0.75],
            ..IconOptimizeOptions::default()
        },
    )
    .expect("optimizer should run");

    assert_eq!(result.candidates.len(), 4);
    assert!(result.best_candidate.metrics.xor_ratio <= 0.25);
    assert!(result.to_svg().contains("<svg"));
}

#[test]
fn optimize_icon_trace_isolates_foreground_from_icon_background() {
    let pixels = (0..6)
        .flat_map(|y| {
            (0..6).map(move |x| {
                if (2..=3).contains(&x) && (2..=3).contains(&y) {
                    rgba(0, 0, 0)
                } else {
                    rgba(255, 255, 255)
                }
            })
        })
        .collect::<Vec<_>>();
    let image = RgbaImage::from_rows(6, 6, &pixels).expect("image should be valid");

    let result = optimize_icon_trace(
        &image,
        IconOptimizeOptions {
            raster_options: RasterOptions {
                threshold: ThresholdMode::Fixed(128),
                invert: true,
                ..RasterOptions::default()
            },
            contour_modes: vec![ContourMode::Subpixel],
            opt_tolerances: vec![0.0],
            isolate_foreground: true,
            ..IconOptimizeOptions::default()
        },
    )
    .expect("optimizer should run");

    assert_eq!(result.best_candidate.metrics.target_foreground_pixels, 4);
    assert!(result.best_candidate.metrics.iou >= 0.75);
}

#[test]
fn optimize_icon_trace_accepts_small_edge_pruned_foreground() {
    let pixels = (0..40)
        .flat_map(|y| {
            (0..40).map(move |x| {
                let center_logo = (10..30).contains(&x) && (10..30).contains(&y);
                let edge_highlight = y == 39 && x >= 20;
                let corner_noise = y == 36 && (36..39).contains(&x);

                if center_logo || edge_highlight || corner_noise {
                    rgba(255, 255, 255)
                } else {
                    rgba(0, 0, 0)
                }
            })
        })
        .collect::<Vec<_>>();
    let image = RgbaImage::from_rows(40, 40, &pixels).expect("image should be valid");

    let result = optimize_icon_trace(
        &image,
        IconOptimizeOptions {
            raster_options: RasterOptions {
                threshold: ThresholdMode::Fixed(128),
                invert: true,
                alpha_background: AlphaBackground::Black,
            },
            contour_modes: vec![ContourMode::Subpixel],
            opt_tolerances: vec![0.0],
            isolate_foreground: true,
            ..IconOptimizeOptions::default()
        },
    )
    .expect("optimizer should run");

    assert_eq!(result.best_candidate.metrics.target_foreground_pixels, 400);
    assert_eq!(result.best_candidate.path_count, 1);
}

#[test]
fn optimize_icon_trace_preserves_disconnected_corner_mark_without_edge_residue() {
    let pixels = (0..40)
        .flat_map(|y| {
            (0..40).map(move |x| {
                let center_logo = (10..30).contains(&x) && (10..30).contains(&y);
                let corner_mark = y == 36 && (36..39).contains(&x);

                if center_logo || corner_mark {
                    rgba(255, 255, 255)
                } else {
                    rgba(0, 0, 0)
                }
            })
        })
        .collect::<Vec<_>>();
    let image = RgbaImage::from_rows(40, 40, &pixels).expect("image should be valid");

    let result = optimize_icon_trace(
        &image,
        IconOptimizeOptions {
            raster_options: RasterOptions {
                threshold: ThresholdMode::Fixed(128),
                invert: true,
                alpha_background: AlphaBackground::Black,
            },
            contour_modes: vec![ContourMode::Subpixel],
            opt_tolerances: vec![0.0],
            isolate_foreground: true,
            ..IconOptimizeOptions::default()
        },
    )
    .expect("optimizer should run");

    assert_eq!(result.best_candidate.metrics.target_foreground_pixels, 403);
    assert_eq!(result.best_candidate.path_count, 2);
}

#[test]
fn optimize_icon_trace_reports_svg_complexity() {
    let pixels = (0..32)
        .flat_map(|y| {
            (0..32).map(move |x| {
                let dx = x as f64 + 0.5 - 16.0;
                let dy = y as f64 + 0.5 - 16.0;

                if dx * dx + dy * dy <= 10.0 * 10.0 {
                    rgba(255, 255, 255)
                } else {
                    rgba(0, 0, 0)
                }
            })
        })
        .collect::<Vec<_>>();
    let image = RgbaImage::from_rows(32, 32, &pixels).expect("image should be valid");

    let result = optimize_icon_trace(
        &image,
        IconOptimizeOptions {
            raster_options: RasterOptions {
                threshold: ThresholdMode::Fixed(128),
                invert: true,
                ..RasterOptions::default()
            },
            contour_modes: vec![ContourMode::Subpixel],
            opt_tolerances: vec![0.25, 0.75],
            ..IconOptimizeOptions::default()
        },
    )
    .expect("optimizer should run");

    let tight_candidate = result
        .candidates
        .iter()
        .find(|candidate| (candidate.trace_options.opt_tolerance - 0.25).abs() < f64::EPSILON)
        .expect("tight candidate should exist");

    assert!(result.best_candidate.svg_command_count > 0);
    assert!(tight_candidate.svg_command_count >= result.best_candidate.svg_command_count);
}

#[test]
fn optimize_icon_trace_limits_complexity_tradeoff_to_close_fits() {
    let pixels = (0..48)
        .flat_map(|y| {
            (0..48).map(move |x| {
                let dx = x as f64 + 0.5 - 24.0;
                let dy = y as f64 + 0.5 - 24.0;
                let in_ring = dx * dx + dy * dy <= 16.0 * 16.0 && dx * dx + dy * dy >= 8.0 * 8.0;

                if in_ring {
                    rgba(255, 255, 255)
                } else {
                    rgba(0, 0, 0)
                }
            })
        })
        .collect::<Vec<_>>();
    let image = RgbaImage::from_rows(48, 48, &pixels).expect("image should be valid");

    let result = optimize_icon_trace(
        &image,
        IconOptimizeOptions {
            raster_options: RasterOptions {
                threshold: ThresholdMode::Fixed(128),
                invert: true,
                ..RasterOptions::default()
            },
            contour_modes: vec![ContourMode::Subpixel],
            opt_tolerances: vec![0.25, 6.0],
            ..IconOptimizeOptions::default()
        },
    )
    .expect("optimizer should run");
    let best_fit = result
        .candidates
        .iter()
        .min_by(|left, right| {
            left.metrics
                .foreground_error_ratio
                .total_cmp(&right.metrics.foreground_error_ratio)
        })
        .expect("optimizer should report candidates");

    assert!(
        result.best_candidate.metrics.foreground_error_ratio
            <= best_fit.metrics.foreground_error_ratio + 0.002,
        "selected candidate degraded fit too far: best={:?}, best_fit={:?}",
        result.best_candidate,
        best_fit
    );
}

fn rgba(red: u8, green: u8, blue: u8) -> Rgba8 {
    Rgba8 {
        red,
        green,
        blue,
        alpha: 255,
    }
}

fn circle_bitmap(width: usize, height: usize, center_x: f64, center_y: f64, radius: f64) -> Bitmap {
    let radius_squared = radius * radius;
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let sample_x = x as f64 + 0.5;
                let sample_y = y as f64 + 0.5;
                let dx = sample_x - center_x;
                let dy = sample_y - center_y;
                dx * dx + dy * dy <= radius_squared
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(width, height, &pixels).expect("bitmap dimensions should match")
}

fn open_arc_bitmap(
    width: usize,
    height: usize,
    center_x: f64,
    center_y: f64,
    inner_radius: f64,
    outer_radius: f64,
    gap_angle: f64,
) -> Bitmap {
    let inner_radius_squared = inner_radius * inner_radius;
    let outer_radius_squared = outer_radius * outer_radius;
    let mid_radius = (inner_radius + outer_radius) / 2.0;
    let cap_radius = (outer_radius - inner_radius) / 2.0;
    let cap_upper = (
        center_x + mid_radius * gap_angle.cos(),
        center_y - mid_radius * gap_angle.sin(),
    );
    let cap_lower = (
        center_x + mid_radius * gap_angle.cos(),
        center_y + mid_radius * gap_angle.sin(),
    );
    let cap_radius_squared = cap_radius * cap_radius;
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let sample_x = x as f64 + 0.5;
                let sample_y = y as f64 + 0.5;
                let dx = sample_x - center_x;
                let dy = sample_y - center_y;
                let radius_squared = dx * dx + dy * dy;
                let angle = dy.atan2(dx).abs();
                let in_arc = radius_squared >= inner_radius_squared
                    && radius_squared <= outer_radius_squared
                    && angle >= gap_angle;
                let upper_dx = sample_x - cap_upper.0;
                let upper_dy = sample_y - cap_upper.1;
                let lower_dx = sample_x - cap_lower.0;
                let lower_dy = sample_y - cap_lower.1;
                let in_upper_cap = upper_dx * upper_dx + upper_dy * upper_dy <= cap_radius_squared;
                let in_lower_cap = lower_dx * lower_dx + lower_dy * lower_dy <= cap_radius_squared;

                in_arc || in_upper_cap || in_lower_cap
            })
        })
        .collect::<Vec<_>>();

    Bitmap::from_rows(width, height, &pixels).expect("bitmap dimensions should match")
}

#[test]
fn turd_size_filters_small_components() {
    let bitmap = Bitmap::from_rows(
        5,
        3,
        &[
            true, false, false, false, false, false, false, true, true, false, false, false, true,
            true, false,
        ],
    )
    .expect("bitmap dimensions should match");

    let traced = trace_bitmap(
        &bitmap,
        TraceOptions {
            turd_size: 2,
            ..TraceOptions::default()
        },
    );

    assert_eq!(traced.paths.len(), 1);
    assert_eq!(
        traced.paths[0].points,
        vec![(2.0, 1.0), (4.0, 1.0), (4.0, 3.0), (2.0, 3.0)]
    );
}

fn count_cubic_segments(svg: &str) -> usize {
    svg.matches(" C ").count()
}

type Cubic = ((f64, f64), (f64, f64), (f64, f64), (f64, f64));

fn cubic_segments_from_svg(svg: &str) -> Vec<Cubic> {
    let path_data = svg
        .split_once(r#"d=""#)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(path_data, _)| path_data)
        .expect("SVG should contain path data");
    let tokens = path_data.split_whitespace().collect::<Vec<_>>();
    let mut segments = Vec::new();
    let mut index = 0;
    let mut current = (0.0, 0.0);

    while index < tokens.len() {
        match tokens[index] {
            "M" => {
                current = (
                    parse_svg_number(tokens[index + 1]),
                    parse_svg_number(tokens[index + 2]),
                );
                index += 3;
            }
            "C" => {
                let control1 = (
                    parse_svg_number(tokens[index + 1].trim_end_matches(',')),
                    parse_svg_number(tokens[index + 2].trim_end_matches(',')),
                );
                let control2 = (
                    parse_svg_number(tokens[index + 3].trim_end_matches(',')),
                    parse_svg_number(tokens[index + 4].trim_end_matches(',')),
                );
                let end = (
                    parse_svg_number(tokens[index + 5]),
                    parse_svg_number(tokens[index + 6]),
                );
                segments.push((current, control1, control2, end));
                current = end;
                index += 7;
            }
            "Z" => index += 1,
            token => panic!("unexpected SVG path token: {token}"),
        }
    }

    segments
}

fn parse_svg_number(token: &str) -> f64 {
    token.parse().expect("SVG coordinate should parse")
}

fn max_cubic_sample_distance_to_closed_path(segments: &[Cubic], path: &[(f64, f64)]) -> f64 {
    let mut max_distance: f64 = 0.0;

    for cubic in segments {
        for sample in 0..=12 {
            let parameter = sample as f64 / 12.0;
            let point = cubic_point(*cubic, parameter);
            max_distance = max_distance.max(distance_to_closed_path(point, path));
        }
    }

    max_distance
}

fn cubic_point(cubic: Cubic, parameter: f64) -> (f64, f64) {
    let inverse = 1.0 - parameter;
    let b0 = inverse * inverse * inverse;
    let b1 = 3.0 * parameter * inverse * inverse;
    let b2 = 3.0 * parameter * parameter * inverse;
    let b3 = parameter * parameter * parameter;

    (
        cubic.0 .0 * b0 + cubic.1 .0 * b1 + cubic.2 .0 * b2 + cubic.3 .0 * b3,
        cubic.0 .1 * b0 + cubic.1 .1 * b1 + cubic.2 .1 * b2 + cubic.3 .1 * b3,
    )
}

fn distance_to_closed_path(point: (f64, f64), path: &[(f64, f64)]) -> f64 {
    path.iter()
        .zip(path.iter().cycle().skip(1))
        .map(|(start, end)| distance_to_segment(point, *start, *end))
        .fold(f64::INFINITY, f64::min)
}

fn distance_to_segment(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> f64 {
    let segment = (end.0 - start.0, end.1 - start.1);
    let length_squared = segment.0 * segment.0 + segment.1 * segment.1;

    if length_squared <= f64::EPSILON {
        return (point.0 - start.0).hypot(point.1 - start.1);
    }

    let projection =
        ((point.0 - start.0) * segment.0 + (point.1 - start.1) * segment.1) / length_squared;
    let projection = projection.clamp(0.0, 1.0);
    let closest = (
        start.0 + segment.0 * projection,
        start.1 + segment.1 * projection,
    );

    (point.0 - closest.0).hypot(point.1 - closest.1)
}

#[test]
fn opt_tolerance_simplifies_stair_step_paths() {
    let bitmap = Bitmap::from_rows(
        5,
        5,
        &[
            true, false, false, false, false, true, true, false, false, false, true, true, true,
            false, false, true, true, true, true, false, true, true, true, true, true,
        ],
    )
    .expect("bitmap dimensions should match");

    let default_trace = trace_bitmap(&bitmap, TraceOptions::default());
    let optimized_trace = trace_bitmap(
        &bitmap,
        TraceOptions {
            opt_tolerance: 0.75,
            ..TraceOptions::default()
        },
    );

    assert_eq!(default_trace.paths.len(), 1);
    assert_eq!(optimized_trace.paths.len(), 1);
    assert!(optimized_trace.paths[0].points.len() < default_trace.paths[0].points.len());
    assert_eq!(
        optimized_trace.paths[0].points,
        vec![(0.0, 0.0), (5.0, 5.0), (0.0, 5.0)]
    );
}
