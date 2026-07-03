# icon-tracer

`icon-tracer` is a small Rust bitmap-to-SVG tracer focused on icon and logo
redrawing. It reads PNM (`P1` through `P6`), uncompressed 24/32-bit BMP, PNG,
and JPEG files, thresholds or samples the foreground, traces regions, and writes
one filled SVG path.

The default mode is icon-oriented: it uses subpixel contours and a native
Potrace-style curve pipeline tuned for compact icon output.

## Install

Install the latest Homebrew release:

```sh
brew install xzhih/tap/icon-tracer
```

This is a Homebrew formula for the CLI binary, not a cask. Homebrew resolves the
`xzhih/tap` tap name to the `xzhih/homebrew-tap` repository.

Prebuilt archives are attached to each GitHub Release for macOS universal,
Linux x86_64, Linux ARM64, and Windows x86_64.

Rust callers can use the `icon_tracer` library API, starting with
`trace_image_to_svg`.

To build from this checkout instead:

```sh
cargo build --release
cargo run -- icon.png icon.svg
```

## Quick Start

```sh
icon-tracer icon.png icon.svg
icon-tracer --preset logo logo.jpg logo.svg
icon-tracer --threshold 180 --invert scan.pgm scan.svg
icon-tracer --contour scalar --curve fit logo.png logo.svg
icon-tracer --invert --alpha-background black --optimize-icon --isolate-foreground --optimization-report report.json icon.png icon.svg
```

## Usage

```sh
icon-tracer [--preset default|logo|scan|icon] [--threshold auto|0-255]
  [--invert|--no-invert] [--alpha-background black|white]
  [--contour pixel|subpixel|scalar]
  [--curve polygon|smooth|spline|fit|potrace] [--smooth|--spline|--fit]
  [--turd-size N] [--opt-tolerance N]
  [--optimize-icon] [--isolate-foreground|--no-isolate-foreground]
  [--optimization-report path.json]
  <input> <output.svg>
```

Explicit flags override preset defaults regardless of argument order.

## Inputs And Defaults

Supported inputs:

- PNM bitmap files: `P1` through `P6`
- uncompressed 24/32-bit BMP files
- PNG files
- JPEG files

Thresholding and alpha handling:

- The default threshold is `auto`, using Otsu's method over luma samples.
- `--threshold N` makes pixels with luma below `N` foreground.
- PGM, PPM, BMP, PNG, and JPEG inputs use luma for black/white thresholding.
- PNG alpha is composited over white by default.
- Use `--alpha-background black` for transparent PNGs intended for dark
  backgrounds.

Presets:

| Preset | Defaults |
| --- | --- |
| `default` | `--threshold auto --contour pixel --curve polygon --turd-size 0 --opt-tolerance 0` |
| `logo` | `--threshold auto --contour subpixel --curve potrace --turd-size 4 --opt-tolerance 0.75` |
| `scan` | `--threshold auto --contour pixel --curve polygon --turd-size 2 --opt-tolerance 0` |
| `icon` | `--threshold auto --contour subpixel --curve potrace --turd-size 2 --opt-tolerance 0.75` |

Without `--preset`, `icon-tracer` uses `icon`.

## Tracing Options

Contour modes:

- `pixel`: exact pixel-cell boundary edges. Use this with `--preset default`
  for raw pixel/polygon tracing.
- `subpixel`: marching squares over the binary mask, with half-pixel edge
  samples to reduce stair-step outlines.
- `scalar`: marching squares over PNG/JPEG luma samples, with interpolated
  threshold crossings for anti-aliased edges.

Curve modes:

- `polygon`: straight SVG line segments.
- `smooth`: local cubic corner rounding.
- `spline`: Catmull-Rom-style closed cubic splines.
- `fit`: adaptive cubic fitting split at sharp corners.
- `potrace`: native optimal-polygon-style simplification, vertex adjustment,
  alpha smoothing, and opticurve merging.

Other controls:

- `--smooth`, `--spline`, and `--fit` are shortcuts for matching `--curve`
  values.
- `--turd-size N` drops traced components whose area is at most `N` pixels.
- `--opt-tolerance N` controls geometric simplification. With
  `--contour pixel --curve potrace`, it is passed to the opticurve merge stage
  while preserving the extracted pixel contour.
- `--optimize-icon` runs an internal candidate search over icon-oriented
  contour and tolerance settings. It currently supports PNG/JPEG inputs because
  it needs RGBA/luma samples.
- `--isolate-foreground` is only valid with `--optimize-icon`. It uses
  deterministic border-color and edge-component heuristics to remove app-icon
  backgrounds before scoring.
- `--optimization-report path.json` writes optimizer candidate metrics,
  including contour mode, opt tolerance, and turd size.

## Pipeline

The trace pipeline is:

1. Decode raster input and composite alpha.
2. Threshold into a binary foreground mask, or sample scalar luma for
   anti-aliased edges.
3. Extract contours.
4. Assemble loops, detect holes by signed area, and apply `--turd-size`.
5. Simplify geometry with `--opt-tolerance` where applicable.
6. Generate curves.
7. Emit one even-odd filled SVG path.

This is an icon-oriented bitmap-to-SVG foundation, not a port of Potrace. The
`potrace` curve mode follows the same broad stages, but the polygon search,
vertex adjustment, and opticurve merge are native `icon-tracer` implementations
tuned for this project's contour data.

## Development

Tracked governance lives in `AGENTS.md` and `docs/governance.md`.

Minimum checks before completing repository work:

```sh
scripts/check-governance.py
cargo fmt --check
git diff --check
```

For Rust behavior changes:

```sh
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Default Rust tests are kept fast for day-to-day work. Slower icon-quality
regressions for candidate selection, templates, and optimizer tradeoffs are
feature-gated:

```sh
cargo test --features slow-tests --lib
```

## Quality Harnesses

For visual-quality work, run the SVG round-trip harness:

```sh
scripts/vector-quality.py
scripts/vector-quality.py --no-build
scripts/vector-quality.py --no-build --update-baseline
scripts/vector-quality.py --no-build --sweep
```

It writes reports under `target/vector-quality/` and compares against
`scripts/vector-quality-baseline.json`. It depends on ImageMagick's `magick`
command and stays outside normal `cargo test`.

For real icon folders, run the batch harness:

```sh
scripts/icon-batch.py path/to/icons --out-dir target/icon-batch
scripts/icon-batch.py path/to/icons --isolate-foreground --out-dir target/icon-batch-isolated
```

It writes traced SVGs, rendered PNGs, optimization reports, CSV/JSON summaries,
and a contact sheet.

For black-box Potrace parity work:

```sh
scripts/potrace-parity.py --no-build
```

This development oracle generates PBM fixtures, runs local Potrace 1.16 with
explicit defaults, compares rendered binary masks, and writes results under
`target/potrace-parity/`. Potrace is not called by the Rust runtime.

## Release

Release automation is documented in `docs/release.md`.

```sh
git tag v0.1.0
git push origin v0.1.0
```

A pushed version tag builds release archives, publishes a GitHub Release, and
updates the Homebrew formula in `xzhih/homebrew-tap`.
