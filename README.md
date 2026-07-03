# icon-tracer

`icon-tracer` is a small Rust bitmap-to-SVG tracer focused on icon and logo
redrawing. The current version reads PNM bitmap files (`P1` through `P6`),
uncompressed 24/32-bit BMP files, PNG files, and JPEG files, thresholds them
into a black and white bitmap, traces the foreground regions, and writes a
filled SVG path.

## Development governance

Tracked project governance lives in `AGENTS.md` and `docs/governance.md`. Before
claiming repository work is complete, run the lightweight governance check:

```sh
scripts/check-governance.py
```

The governance check verifies required docs, reviewable file sizes, and lingering
open-work markers. Behavior-specific verification commands are listed below and
in `docs/governance.md`.

## Usage

```sh
cargo run -- [--preset default|logo|scan|icon] [--threshold auto|0-255] [--invert|--no-invert] [--alpha-background black|white] [--contour pixel|subpixel|scalar] [--curve polygon|smooth|spline|fit|potrace] [--smooth|--spline|--fit] [--turd-size N] [--opt-tolerance N] [--optimize-icon] [--isolate-foreground|--no-isolate-foreground] [--optimization-report path.json] <input> <output.svg>
```

Examples:

```sh
cargo run -- icon.png icon.svg
cargo run -- logo.png logo.svg
cargo run -- --preset logo logo.jpg logo.svg
cargo run -- --preset logo --contour subpixel logo.jpg logo.svg
cargo run -- --preset default sample.pbm sample.svg
cargo run -- --contour scalar --curve fit logo.png logo.svg
cargo run -- --contour scalar --curve potrace logo.png logo.svg
cargo run -- --threshold 180 --invert scan.pgm scan.svg
cargo run -- --threshold auto scan.pgm scan.svg
cargo run -- --smooth logo.pbm logo.svg
cargo run -- --spline logo.pbm logo.svg
cargo run -- --curve polygon --preset logo logo.pbm logo.svg
cargo run -- --fit --opt-tolerance 0.75 logo.pbm logo.svg
cargo run -- --turd-size 2 noisy.pbm clean.svg
cargo run -- --opt-tolerance 0.75 stair.pbm simplified.svg
cargo run -- --invert --alpha-background black --optimize-icon --optimization-report report.json icon.png icon.svg
cargo run -- --invert --alpha-background black --optimize-icon --isolate-foreground --optimization-report report.json icon.png icon.svg
```

## Current tracing behavior

- PNM (`P1` through `P6`), uncompressed 24/32-bit BMP, PNG, and JPEG inputs are auto-detected by magic bytes.
- PGM/PPM, BMP, PNG, and JPEG inputs use luma samples for black/white thresholding.
- The default threshold is `auto`, using Otsu's method over the input luma samples.
- `--threshold N` uses a fixed threshold where pixels with luma below `N` become black.
- PNG alpha is composited over white before thresholding by default; use `--alpha-background black` for transparent PNGs intended to sit on black.

The trace pipeline is:

1. Raster decode and alpha compositing.
2. Thresholding into a binary foreground mask, or scalar luma sampling for anti-aliased edges.
3. Contour extraction:
   - `pixel`: exact pixel-cell boundary edges.
   - `subpixel`: marching squares over the binary mask, with half-pixel edge samples.
   - `scalar`: marching squares over luma samples, with interpolated threshold crossings.
4. Loop assembly, hole detection by signed area, and `--turd-size` filtering.
5. Geometric simplification using `--opt-tolerance` for non-Potrace curve modes.
6. Curve generation:
   - `polygon`: straight SVG lines.
   - `smooth`: local cubic corner rounding.
   - `spline`: Catmull-Rom-style closed cubic spline.
   - `fit`: adaptive cubic fitting split at sharp corners.
   - `potrace`: optimal-polygon-style simplification, vertex adjustment, alpha-based corner smoothing, and graph-based opticurve merging.
7. SVG emission as one even-odd filled path. Holes are emitted in the same `<path>` using `fill-rule="evenodd"`.

- Without `--preset`, `icon-tracer` uses the `icon` preset.
- `--contour pixel` traces exact pixel-cell boundaries. Use `--preset default` for raw pixel/polygon tracing.
- `--contour subpixel` uses a marching-squares contour over the binary image, producing half-pixel coordinates that reduce stair-step outlines.
- `--contour scalar` uses marching squares over PNG/JPEG luma samples and linearly interpolates threshold crossings before fitting curves. This preserves anti-aliased edge information that binary contours discard.
- `--smooth` rounds polygon corners with cubic Bézier segments.
- `--spline` emits a continuous closed cubic Bézier spline through path points.
- `--fit` splits closed paths at sharp corners, then adaptively fits bounded cubic Bézier segments to continuous runs.
- `--curve potrace` is the icon/logo-oriented path. It first finds a lower-complexity polygon from the contour with a Potrace-paper-inspired possible-segment graph, adjusts polygon vertices from neighboring fitted lines, applies alpha smoothing with corner preservation, then uses a graph search to merge compatible adjacent cubic curves.
- `--curve polygon|smooth|spline|fit|potrace` selects the curve output mode; `--smooth`, `--spline`, and `--fit` are shortcuts.
- `--turd-size N` drops traced components whose area is at most `N` pixels.
- `--opt-tolerance N` controls geometric simplification for the project-specific contour modes. With `--contour pixel --curve potrace`, the CLI keeps the extracted pixel contour intact and passes `N` to the opticurve merge stage, matching Potrace's option semantics more closely for the black-box parity path.
- `--optimize-icon` runs an internal feedback loop over icon-oriented contour/tolerance candidates, scores each candidate against the source foreground mask, and writes the best SVG. This currently supports PNG/JPEG inputs because it needs RGBA/luma samples.
- `--isolate-foreground` is only valid with `--optimize-icon`. It uses deterministic border-color and edge-component heuristics to remove app-icon backgrounds before scoring. It is intentionally opt-in because it does not perform semantic logo recognition. In this mode, the optimizer also sweeps bounded `--turd-size` candidates so tiny isolated residue can be dropped when the mask error remains close to the best fit.
- `--optimization-report path.json` writes the internal candidate metrics when `--optimize-icon` is enabled, including the contour mode, opt tolerance, and turd size used by each candidate.

## Vector quality feedback

For visual-quality work, run the SVG round-trip harness:

```sh
scripts/vector-quality.py
```

It generates analytic SVG fixtures, renders them to PNG, traces them back to
SVG with `icon-tracer`, renders the traced SVG, and records pixel-level RMSE, MAE,
absolute-error ratio, SVG size, and path segment counts under
`target/vector-quality/`. It currently runs both `subpixel` and `scalar` contour
modes against the same fixtures. This depends on ImageMagick's `magick` command
and is kept out of normal `cargo test` so the core test suite stays
self-contained.

The checked-in baseline lives at `scripts/vector-quality-baseline.json`. A
normal run compares current output against that baseline, writes delta columns
to `target/vector-quality/report.md`, and exits non-zero if normalized RMSE,
MAE, or absolute-error ratio regresses beyond the configured thresholds:

```sh
scripts/vector-quality.py --no-build
```

After an intentional quality improvement, refresh the baseline:

```sh
scripts/vector-quality.py --no-build --update-baseline
```

For exploratory tuning, run the tolerance sweep. It writes a separate report to
`target/vector-quality-sweep/` and skips the stable baseline unless an explicit
`--baseline` path is supplied:

```sh
scripts/vector-quality.py --no-build --sweep
```

For real icon folders, run the batch harness:

```sh
scripts/icon-batch.py /Users/zero/Downloads/icon --out-dir target/icon-batch
scripts/icon-batch.py /Users/zero/Downloads/icon --isolate-foreground --out-dir target/icon-batch-isolated
```

It writes traced SVGs, rendered PNGs, optimization reports, a CSV/JSON summary,
and a `source-vs-rendered.png` contact sheet. This is a feedback harness for
real-world icon samples; the analytic SVG fixtures above remain the stable
round-trip regression suite.

The Rust-side `--optimize-icon` loop uses a faster internal mask-difference
metric: source foreground mask vs traced foreground mask, with normalized XOR,
foreground error, false-positive/false-negative rates, precision, recall, and
IoU. That makes it useful for parameter selection during tracing. When
foreground isolation is enabled, close-fitting candidates may use a larger
bounded turd size to remove tiny isolated residue. The `scripts/vector-quality.py`
round-trip still remains the final visual guard because it compares rendered SVG
pixels.

For black-box Potrace parity work, run:

```sh
scripts/potrace-parity.py --no-build
```

It generates PBM fixtures, runs local Potrace 1.16 with explicit defaults, runs
`icon-tracer` with `--contour pixel --curve potrace --opt-tolerance 0.2`, renders
both outputs to binary masks, and records pixel-error plus SVG structure metrics
under `target/potrace-parity/`. This is a development oracle only; Potrace is not
called by the Rust runtime.

## Presets

Presets are convenience defaults. Any explicit flag overrides the preset
regardless of argument order. The implicit preset is `icon`.

Presets only tune tracing. `icon-tracer` does not compose app-icon canvases,
rounded backgrounds, preview images, package assets, or semantic brand
decisions; those belong in a caller or workflow layer.

- `default`: raw bitmap tracing with `--threshold auto --contour pixel --curve polygon --turd-size 0 --opt-tolerance 0`
- `logo`: `--threshold auto --contour subpixel --curve potrace --turd-size 4 --opt-tolerance 0.75`
- `scan`: `--threshold auto --contour pixel --curve polygon --turd-size 2 --opt-tolerance 0`
- `icon`: `--threshold auto --contour subpixel --curve potrace --turd-size 2 --opt-tolerance 0.75`

This is an icon-oriented bitmap-to-SVG foundation, not a port of potrace. The
`potrace` curve mode now implements the same broad stages, but the polygon
search, vertex adjustment, and opticurve merge are native `icon-tracer`
implementations tuned for this project's contour data rather than copied from
potrace internals.
