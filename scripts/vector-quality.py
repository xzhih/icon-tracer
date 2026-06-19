#!/usr/bin/env python3
"""Round-trip vector quality harness for icon-tracer.

This script intentionally lives outside `cargo test` because it depends on an
external SVG renderer, currently ImageMagick's `magick` command.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "target" / "vector-quality"
SWEEP_OUT_DIR = ROOT / "target" / "vector-quality-sweep"
BASELINE_PATH = ROOT / "scripts" / "vector-quality-baseline.json"
CANVAS = 512
REGRESSION_THRESHOLDS = {
    "rmse": 0.0015,
    "mae": 0.00025,
    "ae_ratio": 0.00050,
}


@dataclass(frozen=True)
class Fixture:
    name: str
    body: str


@dataclass(frozen=True)
class Mode:
    name: str
    args: tuple[str, ...]


MODES = [
    Mode("subpixel", ("--contour", "subpixel", "--curve", "potrace", "--opt-tolerance", "0.75")),
    Mode("scalar", ("--contour", "scalar", "--curve", "potrace", "--opt-tolerance", "0.5")),
]


SWEEP_TOLERANCES = ("0.25", "0.5", "0.75", "1.0", "1.25")


FIXTURES = [
    Fixture("square", '<rect x="136" y="136" width="240" height="240" fill="black"/>'),
    Fixture("circle", '<circle cx="256" cy="256" r="148" fill="black"/>'),
    Fixture("triangle", '<polygon points="256,82 411,382 101,382" fill="black"/>'),
    Fixture(
        "ring",
        '<circle cx="256" cy="256" r="160" fill="black"/>'
        '<circle cx="256" cy="256" r="86" fill="white"/>',
    ),
    Fixture(
        "open_arc",
        '<path fill="black" d="'
        'M 360 123 '
        'C 302 82 218 80 159 128 '
        'C 91 184 88 289 151 351 '
        'C 209 411 300 413 362 368 '
        'C 376 358 379 338 368 324 '
        'C 357 309 336 306 321 317 '
        'C 281 345 224 341 188 304 '
        'C 149 264 151 202 190 166 '
        'C 225 133 280 131 320 159 '
        'C 335 170 356 166 368 151 '
        'C 379 137 375 115 360 123 Z"/>',
    ),
]


def main() -> int:
    parser = argparse.ArgumentParser(description="Run SVG -> PNG -> SVG round-trip quality checks.")
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="reuse target/release/icon-tracer instead of building it first",
    )
    parser.add_argument(
        "--out-dir",
        default=None,
        help="output directory for fixtures, traced SVGs, PNGs, diffs, and reports",
    )
    parser.add_argument(
        "--baseline",
        default=None,
        help="baseline JSON used for delta and regression checks",
    )
    parser.add_argument(
        "--update-baseline",
        action="store_true",
        help="write the current run as the baseline instead of checking regressions",
    )
    parser.add_argument(
        "--sweep",
        action="store_true",
        help="run an exploratory contour/tolerance matrix instead of the stable baseline modes",
    )
    parser.add_argument(
        "--max-rmse-regression",
        type=float,
        default=REGRESSION_THRESHOLDS["rmse"],
        help="allowed normalized RMSE increase before the run fails",
    )
    parser.add_argument(
        "--max-mae-regression",
        type=float,
        default=REGRESSION_THRESHOLDS["mae"],
        help="allowed normalized MAE increase before the run fails",
    )
    parser.add_argument(
        "--max-ae-ratio-regression",
        type=float,
        default=REGRESSION_THRESHOLDS["ae_ratio"],
        help="allowed absolute-error ratio increase before the run fails",
    )
    args = parser.parse_args()
    baseline_path = Path(args.baseline) if args.baseline else BASELINE_PATH
    explicit_baseline = args.baseline is not None

    if args.sweep and args.update_baseline and not explicit_baseline:
        print(
            "error: --sweep --update-baseline requires an explicit --baseline path",
            file=sys.stderr,
        )
        return 2

    magick = shutil.which("magick")
    if magick is None:
        print("error: ImageMagick `magick` command is required", file=sys.stderr)
        return 2

    modes = sweep_modes() if args.sweep else MODES
    out_dir = Path(args.out_dir) if args.out_dir else (SWEEP_OUT_DIR if args.sweep else OUT_DIR)
    ensure_dirs(out_dir, modes)

    if not args.no_build:
        run(["cargo", "build", "--release"], cwd=ROOT)

    icon_tracer = ROOT / "target" / "release" / "icon-tracer"
    if not icon_tracer.exists():
        print(f"error: missing icon-tracer binary: {icon_tracer}", file=sys.stderr)
        return 2

    rows = []
    for fixture in FIXTURES:
        write_source_fixture(magick, out_dir, fixture)
        for mode in modes:
            rows.append(run_fixture(magick, icon_tracer, out_dir, fixture, mode))

    report = {
        "canvas": {"width": CANVAS, "height": CANVAS},
        "modes": [{"name": mode.name, "args": list(mode.args)} for mode in modes],
        "fixtures": rows,
    }
    baseline = None
    regressions = []
    thresholds = {
        "rmse": args.max_rmse_regression,
        "mae": args.max_mae_regression,
        "ae_ratio": args.max_ae_ratio_regression,
    }
    use_baseline = baseline_path.exists() and (not args.sweep or explicit_baseline)

    if args.update_baseline:
        baseline_path.parent.mkdir(parents=True, exist_ok=True)
        write_json(baseline_path, report)
    elif use_baseline:
        baseline = read_json(baseline_path)
        regressions = attach_baseline_deltas(rows, baseline, thresholds)
        report["baseline"] = {
            "path": display_path(baseline_path),
            "thresholds": thresholds,
            "regressions": regressions,
        }

    write_json(out_dir / "report.json", report)
    write_markdown(
        out_dir / "report.md",
        rows,
        baseline_path=baseline_path if baseline is not None else None,
        thresholds=thresholds,
        regressions=regressions,
        sweep=args.sweep,
    )
    print_table(rows)
    if args.update_baseline:
        print(f"\nbaseline updated: {display_path(baseline_path)}")
    elif use_baseline:
        print(f"\nbaseline: {display_path(baseline_path)}")
        if regressions:
            print("\nregressions:")
            for regression in regressions:
                print(
                    f"- {regression['name']}/{regression['mode']} "
                    f"{regression['metric']} +{regression['delta']:.6f} "
                    f"> {regression['threshold']:.6f}"
                )
    elif args.sweep and not explicit_baseline:
        print("\nbaseline: skipped for sweep")
    else:
        print(f"\nbaseline missing: {display_path(baseline_path)}")
        print("create it with: scripts/vector-quality.py --update-baseline")

    print(f"\nreport: {out_dir / 'report.md'}")
    return 1 if regressions else 0


def sweep_modes() -> list[Mode]:
    modes = []

    for contour in ("subpixel", "scalar"):
        for tolerance in SWEEP_TOLERANCES:
            modes.append(
                Mode(
                    f"{contour}-tol-{tolerance}",
                    ("--contour", contour, "--curve", "potrace", "--opt-tolerance", tolerance),
                )
            )

    return modes


def ensure_dirs(out_dir: Path, modes: list[Mode] | tuple[Mode, ...]) -> None:
    for name in ("source-svg", "source-png"):
        (out_dir / name).mkdir(parents=True, exist_ok=True)
    for mode in modes:
        for name in ("traced-svg", "traced-png", "diff-png"):
            (out_dir / name / mode.name).mkdir(parents=True, exist_ok=True)


def write_source_fixture(magick: str, out_dir: Path, fixture: Fixture) -> None:
    source_svg = out_dir / "source-svg" / f"{fixture.name}.svg"
    source_png = out_dir / "source-png" / f"{fixture.name}.png"

    source_svg.write_text(svg_document(fixture.body), encoding="utf-8")
    render_svg(magick, source_svg, source_png)


def run_fixture(
    magick: str,
    icon_tracer: Path,
    out_dir: Path,
    fixture: Fixture,
    mode: Mode,
) -> dict:
    source_png = out_dir / "source-png" / f"{fixture.name}.png"
    traced_svg = out_dir / "traced-svg" / mode.name / f"{fixture.name}.svg"
    traced_png = out_dir / "traced-png" / mode.name / f"{fixture.name}.png"
    diff_png = out_dir / "diff-png" / mode.name / f"{fixture.name}.png"

    run(
        [
            str(icon_tracer),
            *mode.args,
            str(source_png),
            str(traced_svg),
        ],
        cwd=ROOT,
    )
    render_svg(magick, traced_svg, traced_png)
    render_diff(magick, source_png, traced_png, diff_png)

    rmse = compare_metric(magick, "RMSE", source_png, traced_png)
    mae = compare_metric(magick, "MAE", source_png, traced_png)
    ae_pixels = int(compare_metric(magick, "AE", source_png, traced_png))
    svg = traced_svg.read_text(encoding="utf-8")

    return {
        "name": fixture.name,
        "mode": mode.name,
        "rmse": rmse,
        "mae": mae,
        "ae_pixels": ae_pixels,
        "ae_ratio": ae_pixels / (CANVAS * CANVAS),
        "cubic_segments": svg.count(" C "),
        "line_segments": svg.count(" L "),
        "svg_bytes": traced_svg.stat().st_size,
        "source_svg": rel(out_dir / "source-svg" / f"{fixture.name}.svg"),
        "source_png": rel(source_png),
        "traced_svg": rel(traced_svg),
        "traced_png": rel(traced_png),
        "diff_png": rel(diff_png),
    }


def svg_document(body: str) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{CANVAS}" height="{CANVAS}" '
        f'viewBox="0 0 {CANVAS} {CANVAS}">'
        f'<rect width="{CANVAS}" height="{CANVAS}" fill="white"/>'
        f"{body}"
        "</svg>\n"
    )


def render_svg(magick: str, svg: Path, png: Path) -> None:
    run([magick, "-background", "white", str(svg), "-alpha", "remove", "-alpha", "off", str(png)])


def render_diff(magick: str, source_png: Path, traced_png: Path, diff_png: Path) -> None:
    run(
        [
            magick,
            str(source_png),
            str(traced_png),
            "-compose",
            "difference",
            "-composite",
            "-auto-level",
            str(diff_png),
        ]
    )


def compare_metric(magick: str, metric: str, source_png: Path, traced_png: Path) -> float:
    result = subprocess.run(
        [magick, "compare", "-metric", metric, str(source_png), str(traced_png), "null:"],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    if result.returncode not in (0, 1):
        raise RuntimeError(result.stderr.strip() or f"compare {metric} failed")

    output = result.stderr.strip()
    if metric == "AE":
        return float(output.split()[0])

    normalized = re.search(r"\(([^)]+)\)", output)
    if normalized:
        return float(normalized.group(1))
    return float(output.split()[0])


def run(command: list[str], cwd: Path | None = None) -> None:
    subprocess.run(command, cwd=cwd or ROOT, check=True)


def write_json(path: Path, report: dict) -> None:
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def attach_baseline_deltas(rows: list[dict], baseline: dict, thresholds: dict[str, float]) -> list[dict]:
    baseline_rows = {
        (row["name"], row["mode"]): row
        for row in baseline.get("fixtures", [])
    }
    regressions = []

    for row in rows:
        baseline_row = baseline_rows.get((row["name"], row["mode"]))
        if baseline_row is None:
            continue

        delta = {
            "rmse": row["rmse"] - baseline_row["rmse"],
            "mae": row["mae"] - baseline_row["mae"],
            "ae_ratio": row["ae_ratio"] - baseline_row["ae_ratio"],
            "cubic_segments": row["cubic_segments"] - baseline_row["cubic_segments"],
            "line_segments": row["line_segments"] - baseline_row["line_segments"],
            "svg_bytes": row["svg_bytes"] - baseline_row["svg_bytes"],
        }
        row["baseline"] = {
            key: baseline_row[key]
            for key in ("rmse", "mae", "ae_ratio", "cubic_segments", "line_segments", "svg_bytes")
        }
        row["delta"] = delta

        for metric, threshold in thresholds.items():
            if delta[metric] > threshold:
                regressions.append(
                    {
                        "name": row["name"],
                        "mode": row["mode"],
                        "metric": metric,
                        "delta": delta[metric],
                        "threshold": threshold,
                    }
                )

    return regressions


def write_markdown(
    path: Path,
    rows: list[dict],
    *,
    baseline_path: Path | None,
    thresholds: dict[str, float],
    regressions: list[dict],
    sweep: bool,
) -> None:
    lines = ["# Vector Quality Report", ""]

    if sweep:
        lines.extend(["Mode: sweep", ""])

    if baseline_path is not None:
        lines.extend(
            [
                f"Baseline: `{display_path(baseline_path)}`",
                (
                    "Regression thresholds: "
                    f"RMSE +{thresholds['rmse']:.6f}, "
                    f"MAE +{thresholds['mae']:.6f}, "
                    f"AE ratio +{thresholds['ae_ratio']:.6f}"
                ),
                "",
            ]
        )

    has_delta = any("delta" in row for row in rows)
    if has_delta:
        lines.extend(
            [
                "| fixture | mode | rmse | delta | mae | delta | ae_ratio | delta | cubic | delta | line | svg_bytes |",
                "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
            ]
        )
    else:
        lines.extend(
            [
                "| fixture | mode | rmse | mae | ae_ratio | cubic | line | svg_bytes |",
                "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
            ]
        )

    for row in rows:
        if has_delta:
            delta = row.get("delta", {})
            lines.append(
                f"| {row['name']} | {row['mode']} | {row['rmse']:.6f} | "
                f"{format_delta(delta.get('rmse'))} | {row['mae']:.6f} | "
                f"{format_delta(delta.get('mae'))} | {row['ae_ratio']:.6f} | "
                f"{format_delta(delta.get('ae_ratio'))} | {row['cubic_segments']} | "
                f"{format_integer_delta(delta.get('cubic_segments'))} | "
                f"{row['line_segments']} | {row['svg_bytes']} |"
            )
        else:
            lines.append(
                f"| {row['name']} | {row['mode']} | {row['rmse']:.6f} | {row['mae']:.6f} | "
                f"{row['ae_ratio']:.6f} | {row['cubic_segments']} | "
                f"{row['line_segments']} | {row['svg_bytes']} |"
            )

    if regressions:
        lines.extend(["", "Regressions:", ""])
        for regression in regressions:
            lines.append(
                f"- `{regression['name']}/{regression['mode']}` "
                f"{regression['metric']} increased by {regression['delta']:.6f} "
                f"(limit {regression['threshold']:.6f})."
            )

    lines.extend(
        [
            "",
            "Artifacts:",
            "",
            "- `source-svg/`: analytic SVG fixtures",
            "- `source-png/`: rendered raster inputs",
            "- `traced-svg/<mode>/`: icon-tracer outputs",
            "- `traced-png/<mode>/`: rendered icon-tracer outputs",
            "- `diff-png/<mode>/`: amplified raster differences",
        ]
    )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def print_table(rows: list[dict]) -> None:
    print("fixture     mode               rmse      d_rmse    mae       d_mae     ae_ratio  d_ae      cubic  d_cubic  line  svg_bytes")
    for row in rows:
        delta = row.get("delta", {})
        print(
            f"{row['name']:<10} "
            f"{row['mode']:<18} "
            f"{row['rmse']:.6f}  "
            f"{format_delta(delta.get('rmse')):>8}  "
            f"{row['mae']:.6f}  "
            f"{format_delta(delta.get('mae')):>8}  "
            f"{row['ae_ratio']:.6f}  "
            f"{format_delta(delta.get('ae_ratio')):>8}  "
            f"{row['cubic_segments']:>5}  "
            f"{format_integer_delta(delta.get('cubic_segments')):>7}  "
            f"{row['line_segments']:>4}  "
            f"{row['svg_bytes']:>9}"
        )


def format_delta(value: float | None) -> str:
    if value is None:
        return "-"
    return f"{value:+.6f}"


def format_integer_delta(value: int | None) -> str:
    if value is None:
        return "-"
    return f"{value:+d}"


def rel(path: Path) -> str:
    if not path.is_absolute():
        path = ROOT / path

    return str(path.relative_to(ROOT))


def display_path(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
