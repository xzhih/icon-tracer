#!/usr/bin/env python3
"""Batch icon tracing harness for real-world samples.

The stable geometric regression suite is scripts/vector-quality.py. This script
is a pragmatic companion for icon folders: trace each image, render the SVG, and
write a contact sheet plus machine-readable optimizer metrics.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageOps


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUT_DIR = ROOT / "target" / "icon-batch"
IMAGE_SUFFIXES = {".png", ".jpg", ".jpeg"}


def main() -> int:
    parser = argparse.ArgumentParser(description="Trace a directory of icon images.")
    parser.add_argument("input_dir", help="directory containing PNG/JPEG icon samples")
    parser.add_argument("--out-dir", default=str(DEFAULT_OUT_DIR), help="output directory")
    parser.add_argument("--no-build", action="store_true", help="reuse target/release/icon-tracer")
    parser.add_argument(
        "--isolate-foreground",
        action="store_true",
        help="pass --isolate-foreground to --optimize-icon",
    )
    parser.add_argument(
        "--no-invert",
        action="store_true",
        help="do not pass --invert; useful for dark-on-light sources",
    )
    parser.add_argument(
        "--alpha-background",
        choices=("black", "white"),
        default="black",
        help="transparent-pixel composite color passed to icon-tracer",
    )
    parser.add_argument("--threshold", default=None, help="optional threshold: auto or 0-255")
    parser.add_argument("--limit", type=int, default=None, help="process only the first N files")
    args = parser.parse_args()

    input_dir = Path(args.input_dir)
    out_dir = Path(args.out_dir)
    magick = shutil.which("magick")
    if magick is None:
        print("error: ImageMagick `magick` command is required", file=sys.stderr)
        return 2
    if not input_dir.is_dir():
        print(f"error: input directory does not exist: {input_dir}", file=sys.stderr)
        return 2

    if not args.no_build:
        run(["cargo", "build", "--release"], cwd=ROOT)

    icon_tracer = ROOT / "target" / "release" / "icon-tracer"
    if not icon_tracer.exists():
        print(f"error: missing icon-tracer binary: {icon_tracer}", file=sys.stderr)
        return 2

    inputs = sorted(
        (path for path in input_dir.iterdir() if path.suffix.lower() in IMAGE_SUFFIXES),
        key=lambda path: path.name.lower(),
    )
    if args.limit is not None:
        inputs = inputs[: args.limit]
    if not inputs:
        print(f"error: no PNG/JPEG inputs found in {input_dir}", file=sys.stderr)
        return 2

    prepare_dirs(out_dir)
    rows = []
    failures = []
    for index, source in enumerate(inputs, start=1):
        row = process_icon(index, source, out_dir, icon_tracer, magick, args)
        rows.append(row)
        if row.get("error"):
            failures.append(row)
            print(f"{index:02d} {source.name:45} error   {row['error']}")
        else:
            print(
                f"{index:02d} {source.name:45} "
                f"{row['contour_mode']:8} tol={row['opt_tolerance']} "
                f"iou={row['iou']:.6f} fg_err={row['foreground_error_ratio']:.6f} "
                f"points={row['point_count']} paths={row['path_count']}"
            )

    write_json(out_dir / "summary.json", rows)
    write_csv(out_dir / "summary.csv", rows)
    write_contact_sheet(out_dir / "source-vs-rendered.png", rows)

    print(f"\nsummary: {out_dir / 'summary.csv'}")
    print(f"contact: {out_dir / 'source-vs-rendered.png'}")
    return 1 if failures else 0


def prepare_dirs(out_dir: Path) -> None:
    for name in ("svg", "png", "reports"):
        (out_dir / name).mkdir(parents=True, exist_ok=True)


def process_icon(
    index: int,
    source: Path,
    out_dir: Path,
    icon_tracer: Path,
    magick: str,
    args: argparse.Namespace,
) -> dict:
    slug = f"{index:02d}-{safe_stem(source)}"
    svg_path = out_dir / "svg" / f"{slug}.svg"
    png_path = out_dir / "png" / f"{slug}.png"
    report_path = out_dir / "reports" / f"{slug}.json"
    command = [
        str(icon_tracer),
        "--preset",
        "icon",
        "--alpha-background",
        args.alpha_background,
        "--optimize-icon",
        "--optimization-report",
        str(report_path),
    ]
    if not args.no_invert:
        command.append("--invert")
    if args.isolate_foreground:
        command.append("--isolate-foreground")
    if args.threshold is not None:
        command.extend(["--threshold", args.threshold])
    command.extend([str(source), str(svg_path)])

    row = {
        "index": index,
        "input": str(source),
        "svg": str(svg_path),
        "png": str(png_path),
        "report": str(report_path),
        "contour_mode": "",
        "opt_tolerance": "",
        "iou": 0.0,
        "foreground_error_ratio": 0.0,
        "path_count": 0,
        "point_count": 0,
        "error": "",
    }

    try:
        run(command, cwd=ROOT)
        render_svg(magick, svg_path, png_path)
        report = read_json(report_path)
        best = report.get("best_candidate", {})
        metrics = best.get("metrics", {})
        row.update(
            {
                "contour_mode": best.get("contour_mode", ""),
                "opt_tolerance": best.get("opt_tolerance", ""),
                "iou": float(metrics.get("iou", 0.0)),
                "foreground_error_ratio": float(metrics.get("foreground_error_ratio", 0.0)),
                "path_count": int(best.get("path_count", 0)),
                "point_count": int(best.get("point_count", 0)),
            }
        )
    except Exception as error:  # noqa: BLE001 - keep batch running for bad samples.
        row["error"] = str(error)

    return row


def render_svg(magick: str, svg_path: Path, png_path: Path) -> None:
    run(
        [
            magick,
            str(svg_path),
            "-background",
            "white",
            "-alpha",
            "remove",
            "-alpha",
            "off",
            str(png_path),
        ],
        cwd=ROOT,
    )


def write_contact_sheet(path: Path, rows: list[dict]) -> None:
    thumb = 180
    label_h = 42
    gap = 12
    margin = 16
    columns = 2
    cell_w = thumb * 2 + gap + margin
    cell_h = thumb + label_h + margin
    sheet_w = columns * cell_w + margin
    sheet_h = ((len(rows) + columns - 1) // columns) * cell_h + margin
    sheet = Image.new("RGB", (sheet_w, sheet_h), "white")
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()

    for offset, row in enumerate(rows):
        col = offset % columns
        row_index = offset // columns
        x = margin + col * cell_w
        y = margin + row_index * cell_h
        source = thumb_image(Path(row["input"]), thumb)
        rendered = thumb_image(Path(row["png"]), thumb) if Path(row["png"]).exists() else blank(thumb)
        sheet.paste(source, (x, y))
        sheet.paste(rendered, (x + thumb + gap, y))

        if row.get("error"):
            label = f"{row['index']:02d} error {Path(row['input']).name}"
        else:
            label = (
                f"{row['index']:02d} {row['contour_mode']} P{row['point_count']} "
                f"IoU {row['iou']:.4f}"
            )
        draw.text((x, y + thumb + 4), label, fill="black", font=font)
        draw.text((x, y + thumb + 20), Path(row["input"]).name[:46], fill="black", font=font)

    path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(path)


def thumb_image(path: Path, size: int) -> Image.Image:
    try:
        image = Image.open(path).convert("RGBA")
    except Exception:
        return blank(size)
    canvas = Image.new("RGBA", image.size, "white")
    canvas.alpha_composite(image)
    fitted = ImageOps.contain(canvas.convert("RGB"), (size, size))
    output = Image.new("RGB", (size, size), "white")
    output.paste(fitted, ((size - fitted.width) // 2, (size - fitted.height) // 2))
    return output


def blank(size: int) -> Image.Image:
    return Image.new("RGB", (size, size), "white")


def safe_stem(path: Path) -> str:
    value = re.sub(r"[^A-Za-z0-9._-]+", "-", path.stem).strip("-")
    return value or "icon"


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def write_csv(path: Path, rows: list[dict]) -> None:
    fieldnames = [
        "index",
        "input",
        "contour_mode",
        "opt_tolerance",
        "iou",
        "foreground_error_ratio",
        "path_count",
        "point_count",
        "svg",
        "png",
        "report",
        "error",
    ]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow({name: row.get(name, "") for name in fieldnames})


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        stderr = result.stderr.strip()
        stdout = result.stdout.strip()
        raise RuntimeError(stderr or stdout or f"command failed: {' '.join(command)}")
    return result


if __name__ == "__main__":
    raise SystemExit(main())
