#!/usr/bin/env python3
"""Black-box Potrace parity harness for icon-tracer.

The script uses Potrace only as a development oracle. The Rust binary remains
standalone and does not call Potrace at runtime.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "target" / "potrace-parity"
CANVAS = 256
MODE = "pixel-potrace-o0.2-t2"
POTRACE_OPTS = (
    "-s",
    "--flat",
    "--turnpolicy",
    "minority",
    "--turdsize",
    "2",
    "--alphamax",
    "1",
    "--opttolerance",
    "0.2",
    "--unit",
    "10",
)
ICON_OPTS = (
    "--preset",
    "default",
    "--contour",
    "pixel",
    "--curve",
    "potrace",
    "--turd-size",
    "2",
    "--opt-tolerance",
    "0.2",
)
PARITY_LIMITS = {
    "square": {
        "mask_ae_pixels": 0,
        "icon_command_count": 5,
        "icon_svg_point_count": 4,
        "icon_d_bytes": 25,
    },
    "circle": {
        "mask_ae_pixels": 0,
        "icon_command_count": 7,
        "icon_svg_point_count": 16,
        "icon_d_bytes": 125,
    },
    "two_circles": {
        "mask_ae_pixels": 0,
        "icon_command_count": 10,
        "icon_svg_point_count": 20,
        "icon_d_bytes": 163,
    },
    "ring": {
        "mask_ae_pixels": 0,
        "icon_command_count": 17,
        "icon_svg_point_count": 40,
        "icon_d_bytes": 297,
    },
    "triangle": {
        "mask_ae_pixels": 0,
        "icon_command_count": 8,
        "icon_svg_point_count": 8,
        "icon_d_bytes": 66,
    },
    "roundbar": {
        "mask_ae_pixels": 0,
        "icon_command_count": 8,
        "icon_svg_point_count": 19,
        "icon_d_bytes": 143,
    },
    "double_pill": {
        "mask_ae_pixels": 0,
        "icon_command_count": 22,
        "icon_svg_point_count": 38,
        "icon_d_bytes": 259,
    },
    "plus_shape": {
        "mask_ae_pixels": 0,
        "icon_command_count": 22,
        "icon_svg_point_count": 46,
        "icon_d_bytes": 303,
    },
    "l_shape": {
        "mask_ae_pixels": 0,
        "icon_command_count": 15,
        "icon_svg_point_count": 38,
        "icon_d_bytes": 240,
    },
    "l_mx": {
        "mask_ae_pixels": 0,
        "icon_command_count": 14,
        "icon_svg_point_count": 34,
        "icon_d_bytes": 225,
    },
    "l_my": {
        "mask_ae_pixels": 0,
        "icon_command_count": 14,
        "icon_svg_point_count": 32,
        "icon_d_bytes": 218,
    },
    "l_r90": {
        "mask_ae_pixels": 0,
        "icon_command_count": 14,
        "icon_svg_point_count": 32,
        "icon_d_bytes": 217,
    },
    "l_r180": {
        "mask_ae_pixels": 0,
        "icon_command_count": 13,
        "icon_svg_point_count": 31,
        "icon_d_bytes": 206,
    },
    "l_r270": {
        "mask_ae_pixels": 0,
        "icon_command_count": 14,
        "icon_svg_point_count": 34,
        "icon_d_bytes": 226,
    },
    "t_shape": {
        "mask_ae_pixels": 0,
        "icon_command_count": 17,
        "icon_svg_point_count": 37,
        "icon_d_bytes": 250,
    },
    "h_shape": {
        "mask_ae_pixels": 0,
        "icon_command_count": 23,
        "icon_svg_point_count": 51,
        "icon_d_bytes": 342,
    },
    "hooked_l": {
        "mask_ae_pixels": 0,
        "icon_command_count": 18,
        "icon_svg_point_count": 45,
        "icon_d_bytes": 283,
    },
    "diagonal_bar": {
        "mask_ae_pixels": 0,
        "icon_command_count": 10,
        "icon_svg_point_count": 25,
        "icon_d_bytes": 179,
    },
    "angled_v": {
        "mask_ae_pixels": 0,
        "icon_command_count": 12,
        "icon_svg_point_count": 30,
        "icon_d_bytes": 224,
    },
    "angled_v_mx": {
        "mask_ae_pixels": 0,
        "icon_command_count": 12,
        "icon_svg_point_count": 30,
        "icon_d_bytes": 224,
    },
    "angled_v_my": {
        "mask_ae_pixels": 0,
        "icon_command_count": 14,
        "icon_svg_point_count": 35,
        "icon_d_bytes": 261,
    },
    "angled_v_r90": {
        "mask_ae_pixels": 0,
        "icon_command_count": 12,
        "icon_svg_point_count": 30,
        "icon_d_bytes": 227,
    },
    "angled_v_r180": {
        "mask_ae_pixels": 0,
        "icon_command_count": 14,
        "icon_svg_point_count": 35,
        "icon_d_bytes": 261,
    },
    "angled_v_r270": {
        "mask_ae_pixels": 0,
        "icon_command_count": 12,
        "icon_svg_point_count": 30,
        "icon_d_bytes": 228,
    },
    "rounded_rect_r18": {
        "mask_ae_pixels": 0,
        "icon_command_count": 13,
        "icon_svg_point_count": 33,
        "icon_d_bytes": 215,
    },
    "rounded_rect_r32": {
        "mask_ae_pixels": 0,
        "icon_command_count": 18,
        "icon_svg_point_count": 29,
        "icon_d_bytes": 198,
    },
    "u_shape": {
        "mask_ae_pixels": 0,
        "icon_command_count": 27,
        "icon_svg_point_count": 39,
        "icon_d_bytes": 248,
    },
    "c_shape": {
        "mask_ae_pixels": 0,
        "icon_command_count": 21,
        "icon_svg_point_count": 51,
        "icon_d_bytes": 354,
    },
    "f_shape": {
        "mask_ae_pixels": 0,
        "icon_command_count": 41,
        "icon_svg_point_count": 57,
        "icon_d_bytes": 364,
    },
    "e_shape": {
        "mask_ae_pixels": 0,
        "icon_command_count": 26,
        "icon_svg_point_count": 57,
        "icon_d_bytes": 368,
    },
    "e_mx": {
        "mask_ae_pixels": 0,
        "icon_command_count": 27,
        "icon_svg_point_count": 60,
        "icon_d_bytes": 381,
    },
    "e_my": {
        "mask_ae_pixels": 0,
        "icon_command_count": 26,
        "icon_svg_point_count": 57,
        "icon_d_bytes": 368,
    },
    "e_r90": {
        "mask_ae_pixels": 0,
        "icon_command_count": 26,
        "icon_svg_point_count": 57,
        "icon_d_bytes": 361,
    },
    "e_r180": {
        "mask_ae_pixels": 0,
        "icon_command_count": 27,
        "icon_svg_point_count": 60,
        "icon_d_bytes": 381,
    },
    "e_r270": {
        "mask_ae_pixels": 0,
        "icon_command_count": 27,
        "icon_svg_point_count": 60,
        "icon_d_bytes": 386,
    },
}
COMMAND_RE = re.compile(r"[MmZzLlHhVvCcSsQqTtAa]")
NUMBER_RE = re.compile(r"[-+]?(?:\d*\.\d+|\d+)(?:[eE][-+]?\d+)?")
PATH_RE = re.compile(r"<path\b[^>]*\sd=\"([^\"]*)\"", re.IGNORECASE)
PATH_TOKEN_RE = re.compile(r"[MmZzLlHhVvCcSsQqTtAa]|[-+]?(?:\d*\.\d+|\d+)(?:[eE][-+]?\d+)?")


@dataclass(frozen=True)
class Fixture:
    name: str
    pixels: list[bool]


def main() -> int:
    parser = argparse.ArgumentParser(description="Compare icon-tracer output against Potrace 1.16.")
    parser.add_argument("--no-build", action="store_true", help="reuse target/release/icon-tracer")
    parser.add_argument("--out-dir", default=None, help="output directory")
    parser.add_argument("--check", action="store_true", help="fail if tracked parity metrics regress")
    args = parser.parse_args()

    magick = require_tool("magick")
    potrace = require_tool("potrace")
    out_dir = Path(args.out_dir) if args.out_dir else OUT_DIR
    ensure_dirs(out_dir)

    if not args.no_build:
        run(["cargo", "build", "--release"], cwd=ROOT)

    icon_tracer = ROOT / "target" / "release" / "icon-tracer"
    if not icon_tracer.exists():
        print(f"error: missing binary: {icon_tracer}", file=sys.stderr)
        return 2

    rows = []
    for fixture in fixtures():
        rows.append(run_fixture(magick, potrace, icon_tracer, out_dir, fixture))

    report = {
        "canvas": {"width": CANVAS, "height": CANVAS},
        "mode": MODE,
        "potrace_opts": list(POTRACE_OPTS),
        "icon_opts": list(ICON_OPTS),
        "fixtures": rows,
    }
    write_json(out_dir / "report.json", report)
    write_csv(out_dir / "report.csv", rows)
    write_markdown(out_dir / "report.md", rows)
    print_table(rows)
    print(f"\nreport: {out_dir / 'report.md'}")
    if args.check:
        failures = parity_regressions(rows)
        if failures:
            print("\nparity regressions:", file=sys.stderr)
            for failure in failures:
                print(f"- {failure}", file=sys.stderr)
            return 1
        print("parity check: ok")
    return 0


def fixtures() -> list[Fixture]:
    return [
        Fixture("square", shape_square()),
        Fixture("circle", shape_circle()),
        Fixture("two_circles", shape_two_circles()),
        Fixture("ring", shape_ring()),
        Fixture("triangle", shape_triangle()),
        Fixture("roundbar", shape_roundbar()),
        Fixture("double_pill", shape_double_pill()),
        Fixture("plus_shape", shape_plus()),
        Fixture("l_shape", shape_l()),
        Fixture("l_mx", mirror_x(shape_l())),
        Fixture("l_my", mirror_y(shape_l())),
        Fixture("l_r90", rotate90(shape_l())),
        Fixture("l_r180", rotate180(shape_l())),
        Fixture("l_r270", rotate270(shape_l())),
        Fixture("t_shape", shape_t()),
        Fixture("h_shape", shape_h()),
        Fixture("hooked_l", shape_hooked_l()),
        Fixture("diagonal_bar", shape_diagonal_bar()),
        Fixture("angled_v", shape_angled_v()),
        Fixture("angled_v_mx", mirror_x(shape_angled_v())),
        Fixture("angled_v_my", mirror_y(shape_angled_v())),
        Fixture("angled_v_r90", rotate90(shape_angled_v())),
        Fixture("angled_v_r180", rotate180(shape_angled_v())),
        Fixture("angled_v_r270", rotate270(shape_angled_v())),
        Fixture("rounded_rect_r18", shape_rounded_rect(18.0)),
        Fixture("rounded_rect_r32", shape_rounded_rect(32.0)),
        Fixture("u_shape", shape_u()),
        Fixture("c_shape", shape_c()),
        Fixture("f_shape", shape_f()),
        Fixture("e_shape", shape_e()),
        Fixture("e_mx", mirror_x(shape_e())),
        Fixture("e_my", mirror_y(shape_e())),
        Fixture("e_r90", rotate90(shape_e())),
        Fixture("e_r180", rotate180(shape_e())),
        Fixture("e_r270", rotate270(shape_e())),
    ]


def mirror_x(pixels: list[bool]) -> list[bool]:
    return [pixels[y * CANVAS + (CANVAS - 1 - x)] for y in range(CANVAS) for x in range(CANVAS)]


def mirror_y(pixels: list[bool]) -> list[bool]:
    return [pixels[(CANVAS - 1 - y) * CANVAS + x] for y in range(CANVAS) for x in range(CANVAS)]


def rotate90(pixels: list[bool]) -> list[bool]:
    return [pixels[(CANVAS - 1 - x) * CANVAS + y] for y in range(CANVAS) for x in range(CANVAS)]


def rotate180(pixels: list[bool]) -> list[bool]:
    return [
        pixels[(CANVAS - 1 - y) * CANVAS + (CANVAS - 1 - x)]
        for y in range(CANVAS)
        for x in range(CANVAS)
    ]


def rotate270(pixels: list[bool]) -> list[bool]:
    return [pixels[x * CANVAS + (CANVAS - 1 - y)] for y in range(CANVAS) for x in range(CANVAS)]


def shape_square() -> list[bool]:
    return fill(lambda x, y: 72 <= x < 184 and 72 <= y < 184)


def shape_circle() -> list[bool]:
    cx = cy = CANVAS / 2
    radius = 76
    return fill(lambda x, y: (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= radius * radius)


def shape_two_circles() -> list[bool]:
    left_center = (84.0, 128.0)
    right_center = (172.0, 128.0)
    radius = 42
    return fill(
        lambda x, y: (x + 0.5 - left_center[0]) ** 2 + (y + 0.5 - left_center[1]) ** 2
        <= radius * radius
        or (x + 0.5 - right_center[0]) ** 2 + (y + 0.5 - right_center[1]) ** 2
        <= radius * radius
    )


def shape_ring() -> list[bool]:
    cx = cy = CANVAS / 2
    outer_radius = 78
    inner_radius = 42
    return fill(
        lambda x, y: inner_radius * inner_radius
        < (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2
        <= outer_radius * outer_radius
    )


def shape_triangle() -> list[bool]:
    a = (128.0, 42.0)
    b = (214.0, 214.0)
    c = (42.0, 214.0)
    return fill(lambda x, y: point_in_triangle((x + 0.5, y + 0.5), a, b, c))


def shape_roundbar() -> list[bool]:
    left, top, right, bottom, radius = 40.0, 80.0, 216.0, 176.0, 48.0

    def inside(x: int, y: int) -> bool:
        px = x + 0.5
        py = y + 0.5
        nearest_x = min(max(px, left + radius), right - radius)
        nearest_y = min(max(py, top + radius), bottom - radius)
        return (
            left + radius <= px < right - radius and top <= py < bottom
        ) or (
            left <= px < right and top + radius <= py < bottom - radius
        ) or (px - nearest_x) ** 2 + (py - nearest_y) ** 2 <= radius * radius

    return fill(inside)


def shape_double_pill() -> list[bool]:
    return rounded_rect_union(
        [
            (44.0, 76.0, 212.0, 116.0, 20.0),
            (44.0, 140.0, 212.0, 180.0, 20.0),
        ]
    )


def shape_plus() -> list[bool]:
    return rounded_rect_union(
        [
            (106.0, 48.0, 150.0, 208.0, 18.0),
            (48.0, 106.0, 208.0, 150.0, 18.0),
        ]
    )


def shape_l() -> list[bool]:
    return rounded_rect_union(
        [
            (58.0, 52.0, 102.0, 204.0, 18.0),
            (58.0, 160.0, 198.0, 204.0, 18.0),
        ]
    )


def shape_t() -> list[bool]:
    return rounded_rect_union(
        [
            (106.0, 52.0, 150.0, 204.0, 18.0),
            (58.0, 52.0, 198.0, 96.0, 18.0),
        ]
    )


def shape_h() -> list[bool]:
    return rounded_rect_union(
        [
            (58.0, 52.0, 102.0, 204.0, 18.0),
            (154.0, 52.0, 198.0, 204.0, 18.0),
            (58.0, 106.0, 198.0, 150.0, 18.0),
        ]
    )


def shape_hooked_l() -> list[bool]:
    return rounded_rect_union(
        [
            (58.0, 52.0, 102.0, 204.0, 18.0),
            (58.0, 160.0, 198.0, 204.0, 18.0),
            (154.0, 116.0, 198.0, 204.0, 18.0),
        ]
    )


def shape_rounded_rect(radius: float) -> list[bool]:
    left, top, right, bottom = 54.0, 62.0, 202.0, 194.0

    def inside(x: int, y: int) -> bool:
        px = x + 0.5
        py = y + 0.5
        nearest_x = min(max(px, left + radius), right - radius)
        nearest_y = min(max(py, top + radius), bottom - radius)
        return (px - nearest_x) ** 2 + (py - nearest_y) ** 2 <= radius * radius

    return fill(inside)


def shape_u() -> list[bool]:
    rects = [
        (54.0, 50.0, 96.0, 194.0, 17.0),
        (160.0, 50.0, 202.0, 194.0, 17.0),
        (54.0, 152.0, 202.0, 202.0, 20.0),
    ]

    def inside(x: int, y: int) -> bool:
        px = x + 0.5
        py = y + 0.5
        for left, top, right, bottom, radius in rects:
            nearest_x = min(max(px, left + radius), right - radius)
            nearest_y = min(max(py, top + radius), bottom - radius)
            if (px - nearest_x) ** 2 + (py - nearest_y) ** 2 <= radius * radius:
                return True
        return False

    return fill(inside)


def shape_c() -> list[bool]:
    cx = cy = CANVAS / 2
    outer_radius = 78
    inner_radius = 44
    return fill(
        lambda x, y: inner_radius * inner_radius
        < (x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2
        <= outer_radius * outer_radius
        and not (x + 0.5 > cx and abs(y + 0.5 - cy) < 34)
    )


def shape_f() -> list[bool]:
    runs = [
        (62, 62, 98, 181),
        (63, 63, 96, 183),
        (64, 64, 95, 184),
        (65, 65, 94, 185),
        (66, 67, 93, 186),
        (68, 87, 92, 187),
        (88, 89, 92, 186),
        (90, 90, 92, 185),
        (91, 91, 92, 184),
        (92, 92, 92, 183),
        (93, 93, 92, 181),
        (94, 94, 68, 125),
        (95, 95, 66, 125),
        (96, 96, 65, 125),
        (97, 97, 64, 125),
        (98, 99, 63, 125),
        (100, 119, 62, 125),
        (120, 121, 63, 125),
        (122, 122, 64, 125),
        (123, 123, 65, 125),
        (124, 124, 66, 125),
        (125, 125, 68, 125),
        (126, 126, 92, 181),
        (127, 127, 92, 183),
        (128, 128, 92, 184),
        (129, 129, 92, 185),
        (130, 131, 92, 186),
        (132, 151, 92, 187),
        (152, 153, 92, 186),
        (154, 154, 92, 185),
        (155, 155, 92, 184),
        (156, 156, 92, 183),
        (157, 157, 92, 181),
        (158, 158, 68, 125),
        (159, 159, 66, 125),
        (160, 160, 65, 125),
        (161, 161, 64, 125),
        (162, 163, 63, 125),
        (164, 187, 62, 125),
        (188, 189, 63, 124),
        (190, 190, 64, 123),
        (191, 191, 65, 122),
        (192, 192, 66, 121),
        (193, 193, 68, 119),
    ]
    return fill(
        lambda x, y: any(
            top <= y <= bottom and left <= x <= right
            for top, bottom, left, right in runs
        )
    )


def rounded_rect_union(rects) -> list[bool]:
    def inside(x: int, y: int) -> bool:
        px = x + 0.5
        py = y + 0.5
        for left, top, right, bottom, radius in rects:
            nearest_x = min(max(px, left + radius), right - radius)
            nearest_y = min(max(py, top + radius), bottom - radius)
            if (px - nearest_x) ** 2 + (py - nearest_y) ** 2 <= radius * radius:
                return True
        return False

    return fill(inside)


def shape_e() -> list[bool]:
    rects = [
        (58.0, 52.0, 104.0, 204.0, 16.0),
        (58.0, 52.0, 198.0, 96.0, 16.0),
        (58.0, 106.0, 182.0, 150.0, 16.0),
        (58.0, 160.0, 198.0, 204.0, 16.0),
    ]
    return rounded_rect_union(rects)


def shape_diagonal_bar() -> list[bool]:
    start = (62.0, 186.0)
    end = (194.0, 70.0)
    half_width = 18.0
    return fill(lambda x, y: distance_to_segment((x + 0.5, y + 0.5), start, end) <= half_width)


def shape_angled_v() -> list[bool]:
    left = (70.0, 70.0)
    bottom = (128.0, 186.0)
    right = (186.0, 70.0)
    half_width = 16.0
    return fill(
        lambda x, y: distance_to_segment((x + 0.5, y + 0.5), left, bottom) <= half_width
        or distance_to_segment((x + 0.5, y + 0.5), bottom, right) <= half_width
    )


def fill(predicate) -> list[bool]:
    return [predicate(x, y) for y in range(CANVAS) for x in range(CANVAS)]


def point_in_triangle(point, a, b, c) -> bool:
    def sign(p1, p2, p3):
        return (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])

    d1 = sign(point, a, b)
    d2 = sign(point, b, c)
    d3 = sign(point, c, a)
    return not ((d1 < 0 or d2 < 0 or d3 < 0) and (d1 > 0 or d2 > 0 or d3 > 0))


def distance_to_segment(point, start, end) -> float:
    vx = end[0] - start[0]
    vy = end[1] - start[1]
    wx = point[0] - start[0]
    wy = point[1] - start[1]
    length2 = vx * vx + vy * vy
    if length2 == 0:
        return math.hypot(wx, wy)
    amount = min(max((wx * vx + wy * vy) / length2, 0.0), 1.0)
    projection = (start[0] + amount * vx, start[1] + amount * vy)
    return math.hypot(point[0] - projection[0], point[1] - projection[1])


def run_fixture(magick: str, potrace: str, icon_tracer: Path, out_dir: Path, fixture: Fixture) -> dict:
    input_pbm = out_dir / "input" / f"{fixture.name}.pbm"
    potrace_svg = out_dir / "potrace-svg" / f"{fixture.name}.svg"
    icon_svg = out_dir / "icon-svg" / MODE / f"{fixture.name}.svg"
    potrace_mask = out_dir / "potrace-mask" / f"{fixture.name}.pbm"
    icon_mask = out_dir / "icon-mask" / MODE / f"{fixture.name}.pbm"
    diff_mask = out_dir / "diff-mask" / MODE / f"{fixture.name}.png"

    write_pbm(input_pbm, fixture.pixels)
    run([potrace, *POTRACE_OPTS, "-o", str(potrace_svg), str(input_pbm)], cwd=ROOT)
    run([str(icon_tracer), *ICON_OPTS, str(input_pbm), str(icon_svg)], cwd=ROOT)
    render_binary_mask(magick, potrace_svg, potrace_mask)
    render_binary_mask(magick, icon_svg, icon_mask)
    assert_size(magick, potrace_mask, CANVAS, CANVAS)
    assert_size(magick, icon_mask, CANVAS, CANVAS)
    render_diff(magick, potrace_mask, icon_mask, diff_mask)

    ae_pixels = int(compare_metric(magick, "AE", potrace_mask, icon_mask))
    input_potrace_ae_pixels = int(compare_metric(magick, "AE", input_pbm, potrace_mask))
    input_icon_ae_pixels = int(compare_metric(magick, "AE", input_pbm, icon_mask))
    rmse = compare_metric(magick, "RMSE", potrace_mask, icon_mask)
    potrace_stats = svg_stats(potrace_svg)
    icon_stats = svg_stats(icon_svg)
    command_delta = icon_stats["command_count"] - potrace_stats["command_count"]
    point_delta = icon_stats["point_count"] - potrace_stats["point_count"]
    d_bytes_delta = icon_stats["d_bytes"] - potrace_stats["d_bytes"]

    return {
        "fixture": fixture.name,
        "mode": MODE,
        "mask_ae_pixels": ae_pixels,
        "mask_ae_ratio": ae_pixels / (CANVAS * CANVAS),
        "mask_rmse": rmse,
        "input_potrace_ae_pixels": input_potrace_ae_pixels,
        "input_icon_ae_pixels": input_icon_ae_pixels,
        "potrace_path_count": potrace_stats["path_count"],
        "icon_path_count": icon_stats["path_count"],
        "potrace_command_count": potrace_stats["command_count"],
        "icon_command_count": icon_stats["command_count"],
        "command_delta": command_delta,
        "command_ratio": ratio(icon_stats["command_count"], potrace_stats["command_count"]),
        "potrace_cubic_count": potrace_stats["cubic_count"],
        "icon_cubic_count": icon_stats["cubic_count"],
        "potrace_line_count": potrace_stats["line_count"],
        "icon_line_count": icon_stats["line_count"],
        "potrace_svg_point_count": potrace_stats["point_count"],
        "icon_svg_point_count": icon_stats["point_count"],
        "point_delta": point_delta,
        "point_ratio": ratio(icon_stats["point_count"], potrace_stats["point_count"]),
        "potrace_d_bytes": potrace_stats["d_bytes"],
        "icon_d_bytes": icon_stats["d_bytes"],
        "d_bytes_delta": d_bytes_delta,
        "d_bytes_ratio": ratio(icon_stats["d_bytes"], potrace_stats["d_bytes"]),
        "potrace_svg_bytes": potrace_svg.stat().st_size,
        "icon_svg_bytes": icon_svg.stat().st_size,
        "potrace_svg": rel(potrace_svg),
        "icon_svg": rel(icon_svg),
        "potrace_mask": rel(potrace_mask),
        "icon_mask": rel(icon_mask),
        "diff_mask": rel(diff_mask),
    }


def write_pbm(path: Path, pixels: list[bool]) -> None:
    rows = []
    for y in range(CANVAS):
        row = pixels[y * CANVAS : (y + 1) * CANVAS]
        rows.append(" ".join("1" if pixel else "0" for pixel in row))
    path.write_text(f"P1\n{CANVAS} {CANVAS}\n" + "\n".join(rows) + "\n", encoding="ascii")


def render_binary_mask(magick: str, svg: Path, mask: Path) -> None:
    run(
        [
            magick,
            "-background",
            "white",
            str(svg),
            "-alpha",
            "remove",
            "-alpha",
            "off",
            "-colorspace",
            "Gray",
            "-threshold",
            "50%",
            str(mask),
        ]
    )


def render_diff(magick: str, reference: Path, candidate: Path, diff: Path) -> None:
    run(
        [
            magick,
            str(reference),
            str(candidate),
            "-compose",
            "difference",
            "-composite",
            "-auto-level",
            str(diff),
        ]
    )


def compare_metric(magick: str, metric: str, reference: Path, candidate: Path) -> float:
    result = subprocess.run(
        [magick, "compare", "-metric", metric, str(reference), str(candidate), "null:"],
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


def svg_stats(svg: Path) -> dict:
    text = svg.read_text(encoding="utf-8")
    paths = PATH_RE.findall(text)
    command_count = 0
    cubic_count = 0
    line_count = 0
    point_count = 0
    d_bytes = 0
    for path_data in paths:
        d_bytes += len(path_data.encode("utf-8"))
        path_stats = svg_path_stats(path_data)
        command_count += path_stats["command_count"]
        cubic_count += path_stats["cubic_count"]
        line_count += path_stats["line_count"]
        point_count += path_stats["point_count"]

    return {
        "path_count": len(paths),
        "command_count": command_count,
        "cubic_count": cubic_count,
        "line_count": line_count,
        "point_count": point_count,
        "d_bytes": d_bytes,
    }


def svg_path_stats(path_data: str) -> dict:
    tokens = PATH_TOKEN_RE.findall(path_data)
    index = 0
    command = ""
    command_count = 0
    cubic_count = 0
    line_count = 0
    points = 0

    while index < len(tokens):
        if COMMAND_RE.fullmatch(tokens[index]):
            command = tokens[index]
            index += 1

        upper = command.upper()
        if upper == "Z":
            command_count += 1
            command = ""
            continue
        if not command:
            index += 1
            continue

        arity, segment_points = svg_command_arity_and_points(upper)
        if arity == 0:
            index += 1
            continue

        first_moveto = upper == "M"
        while index + arity <= len(tokens) and not COMMAND_RE.fullmatch(tokens[index]):
            command_count += 1
            if upper in ("C", "S"):
                cubic_count += 1
            elif upper in ("L", "H", "V"):
                line_count += 1
            points += segment_points
            index += arity
            if first_moveto:
                command = "l" if command.islower() else "L"
                upper = "L"
                arity, segment_points = svg_command_arity_and_points(upper)
                first_moveto = False

            if index < len(tokens) and COMMAND_RE.fullmatch(tokens[index]):
                break

        if index < len(tokens) and not COMMAND_RE.fullmatch(tokens[index]):
            index += 1

    return {
        "command_count": command_count,
        "cubic_count": cubic_count,
        "line_count": line_count,
        "point_count": points,
    }


def svg_command_arity_and_points(command: str) -> tuple[int, int]:
    match command:
        case "H" | "V":
            return (1, 1)
        case "M" | "L" | "T":
            return (2, 1)
        case "S" | "Q":
            return (4, 2)
        case "C":
            return (6, 3)
        case "A":
            return (7, 1)
        case _:
            return (0, 0)


def assert_size(magick: str, image: Path, width: int, height: int) -> None:
    result = subprocess.run(
        [magick, "identify", "-format", "%w %h", str(image)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    actual_width, actual_height = (int(part) for part in result.stdout.split())
    if (actual_width, actual_height) != (width, height):
        raise RuntimeError(f"{image} rendered as {actual_width}x{actual_height}, expected {width}x{height}")


def ensure_dirs(out_dir: Path) -> None:
    for name in (
        "input",
        "potrace-svg",
        f"icon-svg/{MODE}",
        "potrace-mask",
        f"icon-mask/{MODE}",
        f"diff-mask/{MODE}",
    ):
        (out_dir / name).mkdir(parents=True, exist_ok=True)


def require_tool(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        print(f"error: required tool not found: {name}", file=sys.stderr)
        sys.exit(2)
    return path


def run(command: list[str], cwd: Path | None = None) -> None:
    subprocess.run(command, cwd=cwd or ROOT, check=True)


def ratio(numerator: float, denominator: float) -> float | None:
    if denominator == 0:
        return None
    return numerator / denominator


def write_json(path: Path, report: dict) -> None:
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_csv(path: Path, rows: list[dict]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)


def write_markdown(path: Path, rows: list[dict]) -> None:
    lines = [
        "# Potrace Parity Report",
        "",
        f"Mode: `{MODE}`",
        "",
        "| Fixture | AE ratio | RMSE | Input AE icon/potrace | Commands icon/potrace | cmd delta | Points icon/potrace | point delta | d bytes icon/potrace | byte delta |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in rows:
        lines.append(
            f"| {row['fixture']} | {row['mask_ae_ratio']:.6f} | {row['mask_rmse']:.6f} | "
            f"{row['input_icon_ae_pixels']}/{row['input_potrace_ae_pixels']} | "
            f"{row['icon_command_count']}/{row['potrace_command_count']} | "
            f"{row['command_delta']:+d} | "
            f"{row['icon_svg_point_count']}/{row['potrace_svg_point_count']} | "
            f"{row['point_delta']:+d} | "
            f"{row['icon_d_bytes']}/{row['potrace_d_bytes']} | "
            f"{row['d_bytes_delta']:+d} |"
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def print_table(rows: list[dict]) -> None:
    print(
        "fixture        ae_ratio   rmse      srcAE i/p   commands  d_cmd  points   d_pts  bytes     b_delta"
    )
    for row in rows:
        print(
            f"{row['fixture']:<14} {row['mask_ae_ratio']:.6f} "
            f"{row['mask_rmse']:.6f} "
            f"{row['input_icon_ae_pixels']}/{row['input_potrace_ae_pixels']:<9} "
            f"{row['icon_command_count']}/{row['potrace_command_count']:<7} "
            f"{row['command_delta']:+5d} "
            f"{row['icon_svg_point_count']}/{row['potrace_svg_point_count']:<7} "
            f"{row['point_delta']:+5d} "
            f"{row['icon_d_bytes']}/{row['potrace_d_bytes']:<8} "
            f"{row['d_bytes_delta']:+7d}"
        )


def parity_regressions(rows: list[dict]) -> list[str]:
    failures = []
    for row in rows:
        fixture = row["fixture"]
        limits = PARITY_LIMITS.get(fixture)
        if limits is None:
            continue

        for metric, limit in limits.items():
            actual = row[metric]
            if actual > limit:
                failures.append(f"{fixture} {metric} {actual} > {limit}")
    return failures


def rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
