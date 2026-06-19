use crate::{Bitmap, BitmapError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryMask {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Vec<bool>,
}

impl BinaryMask {
    pub fn from_rows(width: usize, height: usize, pixels: &[bool]) -> Result<Self, BitmapError> {
        if pixels.len() != width.saturating_mul(height) {
            return Err(BitmapError::DimensionMismatch {
                width,
                height,
                pixels: pixels.len(),
            });
        }

        Ok(Self {
            width,
            height,
            pixels: pixels.to_vec(),
        })
    }

    pub fn from_bitmap(bitmap: &Bitmap) -> Self {
        let width = bitmap.width();
        let height = bitmap.height();
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| bitmap.is_black(x, y)))
            .collect::<Vec<_>>();

        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixels(&self) -> &[bool] {
        &self.pixels
    }

    pub fn is_foreground(&self, x: usize, y: usize) -> bool {
        self.pixels[y * self.width + x]
    }

    pub fn to_bitmap(&self) -> Bitmap {
        Bitmap::from_rows(self.width, self.height, &self.pixels)
            .expect("mask dimensions should match pixels")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub min_x: usize,
    pub min_y: usize,
    pub max_x: usize,
    pub max_y: usize,
}

impl Bounds {
    pub fn width(self) -> usize {
        self.max_x - self.min_x + 1
    }

    pub fn height(self) -> usize {
        self.max_y - self.min_y + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleFacts {
    pub area_pixels: usize,
    pub bbox: Bounds,
    pub centroid: FloatPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentFacts {
    pub id: usize,
    pub area_pixels: usize,
    pub bbox: Bounds,
    pub centroid: FloatPoint,
    pub touches_canvas_edge: bool,
    pub holes: Vec<HoleFacts>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentAnalysis {
    pub width: usize,
    pub height: usize,
    pub components: Vec<ComponentFacts>,
    pub interior_component_count: usize,
    pub edge_touching_component_count: usize,
}

pub fn analyze_components(mask: &BinaryMask, min_pixels: usize) -> ComponentAnalysis {
    let raw_components = connected_components(mask, min_pixels.max(1));
    let components = raw_components
        .iter()
        .enumerate()
        .map(|(index, component)| component_facts(index + 1, component, mask))
        .collect::<Vec<_>>();

    ComponentAnalysis {
        width: mask.width,
        height: mask.height,
        interior_component_count: components
            .iter()
            .filter(|component| !component.touches_canvas_edge)
            .count(),
        edge_touching_component_count: components
            .iter()
            .filter(|component| component.touches_canvas_edge)
            .count(),
        components,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RawComponent {
    pub(crate) pixels: Vec<usize>,
    pub(crate) min_x: usize,
    pub(crate) min_y: usize,
    pub(crate) max_x: usize,
    pub(crate) max_y: usize,
    sum_x: usize,
    sum_y: usize,
}

pub(crate) fn connected_components(mask: &BinaryMask, min_pixels: usize) -> Vec<RawComponent> {
    let mut visited = vec![false; mask.pixels.len()];
    let mut queue = Vec::new();
    let mut components = Vec::new();

    for start in 0..mask.pixels.len() {
        if !mask.pixels[start] || visited[start] {
            continue;
        }

        let mut component = RawComponent {
            pixels: Vec::new(),
            min_x: mask.width,
            min_y: mask.height,
            max_x: 0,
            max_y: 0,
            sum_x: 0,
            sum_y: 0,
        };
        queue.clear();
        queue.push(start);
        visited[start] = true;

        let mut cursor = 0;
        while cursor < queue.len() {
            let current = queue[cursor];
            cursor += 1;
            let x = current % mask.width;
            let y = current / mask.width;

            component.pixels.push(current);
            component.min_x = component.min_x.min(x);
            component.min_y = component.min_y.min(y);
            component.max_x = component.max_x.max(x);
            component.max_y = component.max_y.max(y);
            component.sum_x += x;
            component.sum_y += y;

            for (next_x, next_y) in orthogonal_neighbors(x, y, mask.width, mask.height) {
                let index = next_y * mask.width + next_x;
                if mask.pixels[index] && !visited[index] {
                    visited[index] = true;
                    queue.push(index);
                }
            }
        }

        if component.pixels.len() >= min_pixels {
            components.push(component);
        }
    }

    components.sort_by_key(|component| std::cmp::Reverse(component.pixels.len()));
    components
}

fn orthogonal_neighbors(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> impl Iterator<Item = (usize, usize)> {
    let mut neighbors = [(0usize, 0usize); 4];
    let mut count = 0usize;

    if x > 0 {
        neighbors[count] = (x - 1, y);
        count += 1;
    }
    if x + 1 < width {
        neighbors[count] = (x + 1, y);
        count += 1;
    }
    if y > 0 {
        neighbors[count] = (x, y - 1);
        count += 1;
    }
    if y + 1 < height {
        neighbors[count] = (x, y + 1);
        count += 1;
    }

    neighbors.into_iter().take(count)
}

fn component_facts(id: usize, component: &RawComponent, mask: &BinaryMask) -> ComponentFacts {
    let bbox = Bounds {
        min_x: component.min_x,
        min_y: component.min_y,
        max_x: component.max_x,
        max_y: component.max_y,
    };
    let touches_canvas_edge = bbox.min_x == 0
        || bbox.min_y == 0
        || bbox.max_x + 1 == mask.width
        || bbox.max_y + 1 == mask.height;

    ComponentFacts {
        id,
        area_pixels: component.pixels.len(),
        bbox,
        centroid: FloatPoint {
            x: component.sum_x as f64 / component.pixels.len() as f64,
            y: component.sum_y as f64 / component.pixels.len() as f64,
        },
        touches_canvas_edge,
        holes: detect_component_holes(component, mask),
    }
}

fn detect_component_holes(component: &RawComponent, mask: &BinaryMask) -> Vec<HoleFacts> {
    let bbox = Bounds {
        min_x: component.min_x,
        min_y: component.min_y,
        max_x: component.max_x,
        max_y: component.max_y,
    };
    let local_width = bbox.width();
    let local_height = bbox.height();
    let mut foreground = vec![false; local_width * local_height];

    for y in bbox.min_y..=bbox.max_y {
        for x in bbox.min_x..=bbox.max_x {
            if mask.is_foreground(x, y) {
                foreground[(y - bbox.min_y) * local_width + (x - bbox.min_x)] = true;
            }
        }
    }

    let mut visited = vec![false; foreground.len()];
    let mut queue = Vec::new();
    let mut holes = Vec::new();

    for start in 0..foreground.len() {
        if foreground[start] || visited[start] {
            continue;
        }

        let mut touches_edge = false;
        let mut pixels = 0usize;
        let mut min_x = local_width;
        let mut min_y = local_height;
        let mut max_x = 0usize;
        let mut max_y = 0usize;
        let mut sum_x = 0usize;
        let mut sum_y = 0usize;

        queue.clear();
        queue.push(start);
        visited[start] = true;
        let mut cursor = 0;

        while cursor < queue.len() {
            let current = queue[cursor];
            cursor += 1;
            let x = current % local_width;
            let y = current / local_width;

            if x == 0 || y == 0 || x + 1 == local_width || y + 1 == local_height {
                touches_edge = true;
            }

            pixels += 1;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            sum_x += x + bbox.min_x;
            sum_y += y + bbox.min_y;

            for (next_x, next_y) in orthogonal_neighbors(x, y, local_width, local_height) {
                let index = next_y * local_width + next_x;
                if !foreground[index] && !visited[index] {
                    visited[index] = true;
                    queue.push(index);
                }
            }
        }

        if !touches_edge {
            holes.push(HoleFacts {
                area_pixels: pixels,
                bbox: Bounds {
                    min_x: min_x + bbox.min_x,
                    min_y: min_y + bbox.min_y,
                    max_x: max_x + bbox.min_x,
                    max_y: max_y + bbox.min_y,
                },
                centroid: FloatPoint {
                    x: sum_x as f64 / pixels as f64,
                    y: sum_y as f64 / pixels as f64,
                },
            });
        }
    }

    holes.sort_by_key(|hole| std::cmp::Reverse(hole.area_pixels));
    holes
}
