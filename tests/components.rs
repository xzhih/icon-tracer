use icon_tracer::{analyze_components, BinaryMask, Bounds, FloatPoint};

#[test]
fn component_facts_report_bbox_area_centroid_and_edge_touching() {
    let mask = BinaryMask::from_rows(
        5,
        5,
        &[
            true, true, false, false, false, true, true, false, false, false, false, false, true,
            true, false, false, false, true, true, false, false, false, false, false, true,
        ],
    )
    .expect("mask dimensions should match");

    let analysis = analyze_components(&mask, 1);

    assert_eq!(analysis.width, 5);
    assert_eq!(analysis.height, 5);
    assert_eq!(analysis.components.len(), 3);
    assert_eq!(analysis.edge_touching_component_count, 2);
    assert_eq!(analysis.interior_component_count, 1);

    let largest = &analysis.components[0];
    assert_eq!(largest.area_pixels, 4);
    assert_eq!(
        largest.bbox,
        Bounds {
            min_x: 0,
            min_y: 0,
            max_x: 1,
            max_y: 1,
        }
    );
    assert_eq!(largest.centroid, FloatPoint { x: 0.5, y: 0.5 });
    assert!(largest.touches_canvas_edge);

    let interior = analysis
        .components
        .iter()
        .find(|component| !component.touches_canvas_edge)
        .expect("one component should be interior");
    assert_eq!(interior.area_pixels, 4);
    assert_eq!(
        interior.bbox,
        Bounds {
            min_x: 2,
            min_y: 2,
            max_x: 3,
            max_y: 3,
        }
    );
    assert_eq!(interior.centroid, FloatPoint { x: 2.5, y: 2.5 });
}

#[test]
fn component_facts_detect_holes() {
    let mask = BinaryMask::from_rows(
        3,
        3,
        &[true, true, true, true, false, true, true, true, true],
    )
    .expect("mask dimensions should match");

    let analysis = analyze_components(&mask, 1);

    assert_eq!(analysis.components.len(), 1);
    let component = &analysis.components[0];
    assert_eq!(component.area_pixels, 8);
    assert_eq!(component.holes.len(), 1);
    assert_eq!(component.holes[0].area_pixels, 1);
    assert_eq!(
        component.holes[0].bbox,
        Bounds {
            min_x: 1,
            min_y: 1,
            max_x: 1,
            max_y: 1,
        }
    );
    assert_eq!(component.holes[0].centroid, FloatPoint { x: 1.0, y: 1.0 });
}

#[test]
fn component_holes_exclude_nested_foreground_components() {
    let mask = BinaryMask::from_rows(
        5,
        5,
        &[
            true, true, true, true, true, true, false, false, false, true, true, false, true,
            false, true, true, false, false, false, true, true, true, true, true, true,
        ],
    )
    .expect("mask dimensions should match");

    let analysis = analyze_components(&mask, 1);

    assert_eq!(analysis.components.len(), 2);
    let outer = &analysis.components[0];
    assert_eq!(outer.area_pixels, 16);
    assert_eq!(outer.holes.len(), 1);
    assert_eq!(outer.holes[0].area_pixels, 8);
    assert_eq!(
        outer.holes[0].bbox,
        Bounds {
            min_x: 1,
            min_y: 1,
            max_x: 3,
            max_y: 3,
        }
    );
    assert_eq!(outer.holes[0].centroid, FloatPoint { x: 2.0, y: 2.0 });

    let inner = &analysis.components[1];
    assert_eq!(inner.area_pixels, 1);
    assert!(inner.holes.is_empty());
}

#[test]
fn component_facts_respect_min_pixels() {
    let mask = BinaryMask::from_rows(4, 2, &[true, false, false, false, false, false, true, true])
        .expect("mask dimensions should match");

    let analysis = analyze_components(&mask, 2);

    assert_eq!(analysis.components.len(), 1);
    assert_eq!(analysis.components[0].area_pixels, 2);
}

#[test]
fn binary_mask_round_trips_through_bitmap() {
    let mask = BinaryMask::from_rows(2, 1, &[true, false]).expect("mask dimensions should match");

    let bitmap = mask.to_bitmap();
    let round_trip = bitmap.as_mask();

    assert_eq!(round_trip.width(), 2);
    assert_eq!(round_trip.height(), 1);
    assert!(round_trip.is_foreground(0, 0));
    assert!(!round_trip.is_foreground(1, 0));
}
