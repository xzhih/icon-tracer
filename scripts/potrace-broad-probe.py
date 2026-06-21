#!/usr/bin/env python3
"""Broader synthetic Potrace parity probe for icon-tracer.

This is intentionally separate from potrace-parity.py: the core parity harness
tracks fixtures that should already match Potrace exactly, while this probe
keeps a stable list of harder non-template shapes for regression feedback.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import math
import random
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
POTRACE_PARITY = ROOT / "scripts" / "potrace-parity.py"
OUT_DIR = ROOT / "target" / "potrace-broad-probe"
TOTAL_AE_LIMIT = 1705
BROAD_AE_LIMITS = {
    "capsule_0": 36,
    "capsule_1": 63,
    "capsule_2": 65,
    "capsule_3": 26,
    "capsule_4": 51,
    "capsule_5": 48,
    "capsule_6": 0,
    "capsule_7": 0,
    "capsule_8": 0,
    "fat_l": 0,
    "hook_top": 26,
    "kite": 66,
    "low_t": 38,
    "narrow_h": 23,
    "offset_hook": 27,
    "offset_t": 4,
    "offset_u": 0,
    "random_union_00": 72,
    "random_union_01": 43,
    "random_union_02": 53,
    "random_union_03": 48,
    "random_union_04": 63,
    "random_union_05": 41,
    "random_union_06": 28,
    "random_union_07": 21,
    "random_union_08": 0,
    "random_union_09": 58,
    "random_union_10": 62,
    "random_union_11": 71,
    "random_union_12": 18,
    "random_union_13": 38,
    "random_union_14": 33,
    "random_union_15": 24,
    "random_union_16": 33,
    "random_union_17": 23,
    "random_union_18": 30,
    "random_union_19": 18,
    "ring_sector_0": 53,
    "ring_sector_1": 89,
    "ring_sector_2": 46,
    "ring_sector_3": 44,
    "ring_sector_4": 40,
    "sharp_v": 58,
    "skew_rect": 45,
    "thin_e": 0,
    "thin_l": 0,
    "trapezoid": 53,
    "wide_e": 0,
    "wide_h": 26,
    "wide_l": 1,
}


def main() -> int:
    parser = argparse.ArgumentParser(description="Run broad synthetic Potrace parity probes.")
    parser.add_argument("--no-build", action="store_true", help="reuse target/release/icon-tracer")
    parser.add_argument("--out-dir", default=None, help="output directory")
    parser.add_argument("--check", action="store_true", help="fail if broad probe metrics regress")
    args = parser.parse_args()

    pp = load_potrace_parity_module()
    out_dir = Path(args.out_dir) if args.out_dir else OUT_DIR
    pp.ensure_dirs(out_dir)
    magick = pp.require_tool("magick")
    potrace = pp.require_tool("potrace")

    if not args.no_build:
        subprocess.run(["cargo", "build", "--release"], cwd=ROOT, check=True)

    icon_tracer = ROOT / "target" / "release" / "icon-tracer"
    if not icon_tracer.exists():
        print(f"error: missing binary: {icon_tracer}", file=sys.stderr)
        return 2

    rows = [
        pp.run_fixture(magick, potrace, icon_tracer, out_dir, fixture)
        for fixture in broad_fixtures(pp)
    ]
    report = {
        "canvas": {"width": pp.CANVAS, "height": pp.CANVAS},
        "mode": pp.MODE,
        "potrace_opts": list(pp.POTRACE_OPTS),
        "icon_opts": list(pp.ICON_OPTS),
        "total_mask_ae_pixels": sum(row["mask_ae_pixels"] for row in rows),
        "fixtures": rows,
    }
    pp.write_json(out_dir / "report.json", report)
    pp.write_csv(out_dir / "report.csv", rows)
    write_broad_markdown(pp, out_dir / "report.md", rows)
    pp.print_table(rows)
    print_broad_summary(rows)
    print_worst_gaps(rows)
    print(f"\nreport: {out_dir / 'report.md'}")

    if args.check:
        failures = broad_regressions(rows)
        if failures:
            print("\nbroad parity regressions:", file=sys.stderr)
            for failure in failures:
                print(f"- {failure}", file=sys.stderr)
            return 1
        print("broad parity check: ok")
    return 0


def load_potrace_parity_module():
    spec = importlib.util.spec_from_file_location("potrace_parity", POTRACE_PARITY)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {POTRACE_PARITY}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def broad_fixtures(pp) -> list:
    fixtures = []
    fixtures.extend(rounded_union_fixtures(pp))
    fixtures.extend(capsule_fixtures(pp))
    fixtures.extend(ring_sector_fixtures(pp))
    fixtures.extend(polygon_fixtures(pp))
    fixtures.extend(random_union_fixtures(pp))
    return fixtures


def rounded_union_fixtures(pp) -> list:
    specs = [
        ("wide_l", [(50, 44, 92, 210, 15), (50, 166, 214, 210, 18)]),
        ("fat_l", [(54, 46, 112, 208, 22), (54, 148, 210, 208, 24)]),
        ("thin_l", [(66, 48, 94, 204, 12), (66, 176, 202, 204, 12)]),
        ("offset_t", [(110, 48, 154, 210, 17), (44, 58, 206, 102, 17)]),
        ("low_t", [(106, 42, 150, 212, 17), (58, 64, 198, 112, 20)]),
        (
            "wide_h",
            [(48, 50, 94, 204, 18), (162, 50, 208, 204, 18), (48, 112, 208, 152, 16)],
        ),
        (
            "narrow_h",
            [(70, 54, 106, 200, 14), (150, 54, 186, 200, 14), (70, 112, 186, 148, 14)],
        ),
        (
            "offset_u",
            [(48, 44, 92, 190, 18), (154, 58, 204, 198, 20), (48, 146, 204, 204, 22)],
        ),
        (
            "wide_e",
            [(48, 48, 98, 208, 16), (48, 48, 210, 92, 16), (48, 108, 190, 150, 16), (48, 166, 210, 208, 16)],
        ),
        (
            "thin_e",
            [(68, 54, 104, 202, 12), (68, 54, 194, 88, 12), (68, 112, 176, 146, 12), (68, 168, 194, 202, 12)],
        ),
        (
            "offset_hook",
            [(54, 48, 98, 208, 17), (54, 156, 204, 202, 19), (146, 108, 198, 202, 20)],
        ),
        (
            "hook_top",
            [(58, 52, 102, 204, 18), (58, 52, 198, 96, 18), (154, 52, 198, 142, 18)],
        ),
    ]
    return [pp.Fixture(name, rounded_rect_union(pp, rects)) for name, rects in specs]


def capsule_fixtures(pp) -> list:
    specs = [
        ((38, 184), (218, 72), 17),
        ((42, 188), (204, 92), 24),
        ((52, 64), (206, 184), 15),
        ((34, 128), (222, 116), 18),
        ((42, 104), (218, 154), 16),
        ((70, 42), (88, 214), 19),
        ((38, 190), (164, 54), 22),
        ((92, 206), (214, 70), 13),
        ((40, 78), (210, 92), 21),
    ]
    return [
        pp.Fixture(
            f"capsule_{index}",
            fill(pp, lambda x, y, s=start, e=end, w=width: pp.distance_to_segment((x + 0.5, y + 0.5), s, e) <= w),
        )
        for index, (start, end, width) in enumerate(specs)
    ]


def ring_sector_fixtures(pp) -> list:
    specs = [
        (30, 310, 42, 78),
        (70, 290, 38, 80),
        (120, 420, 48, 82),
        (210, 140, 36, 76),
        (350, 190, 44, 82),
    ]
    return [ring_sector_fixture(pp, f"ring_sector_{index}", *spec) for index, spec in enumerate(specs)]


def polygon_fixtures(pp) -> list:
    return [
        polygon_fixture(pp, "trapezoid", [(58, 62), (194, 46), (212, 194), (42, 210)]),
        polygon_fixture(pp, "skew_rect", [(70, 52), (184, 68), (204, 202), (50, 188)]),
        polygon_fixture(pp, "kite", [(128, 34), (210, 126), (128, 222), (48, 130)]),
        polygon_fixture(
            pp,
            "sharp_v",
            [(58, 60), (96, 60), (128, 150), (160, 60), (198, 60), (146, 204), (110, 204)],
        ),
    ]


def random_union_fixtures(pp) -> list:
    rng = random.Random(7)
    fixtures = []
    for index in range(20):
        rects = []
        for _ in range(rng.randint(2, 4)):
            width = rng.randint(32, 120)
            height = rng.randint(32, 150)
            left = rng.randint(36, 220 - width)
            top = rng.randint(36, 220 - height)
            radius = rng.randint(8, min(28, max(8, min(width, height) // 2)))
            rects.append((float(left), float(top), float(left + width), float(top + height), float(radius)))
        fixtures.append(pp.Fixture(f"random_union_{index:02d}", rounded_rect_union(pp, rects)))
    return fixtures


def rounded_rect_union(pp, rects) -> list[bool]:
    checks = [rounded_rect(rect) for rect in rects]
    return fill(pp, lambda x, y: any(check(x, y) for check in checks))


def rounded_rect(rect):
    left, top, right, bottom, radius = rect

    def inside(x: int, y: int) -> bool:
        px = x + 0.5
        py = y + 0.5
        nearest_x = min(max(px, left + radius), right - radius)
        nearest_y = min(max(py, top + radius), bottom - radius)
        return (px - nearest_x) ** 2 + (py - nearest_y) ** 2 <= radius * radius

    return inside


def ring_sector_fixture(pp, name: str, start_deg: float, end_deg: float, inner: float, outer: float):
    center = pp.CANVAS / 2

    def inside(x: int, y: int) -> bool:
        px = x + 0.5 - center
        py = y + 0.5 - center
        radius_squared = px * px + py * py
        if not inner * inner < radius_squared <= outer * outer:
            return False
        angle = math.degrees(math.atan2(py, px)) % 360
        if start_deg <= end_deg:
            return start_deg <= angle <= end_deg
        return angle >= start_deg or angle <= end_deg

    return pp.Fixture(name, fill(pp, inside))


def polygon_fixture(pp, name: str, points):
    def inside(x: int, y: int) -> bool:
        px = x + 0.5
        py = y + 0.5
        hit = False
        previous = len(points) - 1
        for index, (x1, y1) in enumerate(points):
            x0, y0 = points[previous]
            if (y1 > py) != (y0 > py):
                crossing = (x0 - x1) * (py - y1) / (y0 - y1) + x1
                if px < crossing:
                    hit = not hit
            previous = index
        return hit

    return pp.Fixture(name, fill(pp, inside))


def fill(pp, predicate) -> list[bool]:
    return [predicate(x, y) for y in range(pp.CANVAS) for x in range(pp.CANVAS)]


def total_mask_ae_pixels(rows: list[dict]) -> int:
    return sum(row["mask_ae_pixels"] for row in rows)


def print_broad_summary(rows: list[dict]) -> None:
    print(f"\ntotal broad AE: {total_mask_ae_pixels(rows)} / {TOTAL_AE_LIMIT}")


def write_broad_markdown(pp, path: Path, rows: list[dict]) -> None:
    pp.write_markdown(path, rows)
    text = path.read_text(encoding="utf-8")
    summary = f"Total broad AE: `{total_mask_ae_pixels(rows)}` / `{TOTAL_AE_LIMIT}`"
    text = text.replace(f"Mode: `{pp.MODE}`\n", f"Mode: `{pp.MODE}`\n\n{summary}\n", 1)
    path.write_text(text, encoding="utf-8")


def print_worst_gaps(rows: list[dict]) -> None:
    print("\nworst broad gaps:")
    for row in sorted(rows, key=lambda item: (-item["mask_ae_pixels"], item["fixture"]))[:12]:
        print(
            f"- {row['fixture']}: AE={row['mask_ae_pixels']} "
            f"inputAE={row['input_icon_ae_pixels']}/{row['input_potrace_ae_pixels']} "
            f"commands={row['icon_command_count']}/{row['potrace_command_count']}"
        )


def broad_regressions(rows: list[dict]) -> list[str]:
    failures = []
    total = total_mask_ae_pixels(rows)
    if total > TOTAL_AE_LIMIT:
        failures.append(f"total mask_ae_pixels {total} > {TOTAL_AE_LIMIT}")
    for row in rows:
        limit = BROAD_AE_LIMITS.get(row["fixture"])
        if limit is None:
            failures.append(f"{row['fixture']} has no broad AE limit")
        elif row["mask_ae_pixels"] > limit:
            failures.append(f"{row['fixture']} mask_ae_pixels {row['mask_ae_pixels']} > {limit}")
    return failures


if __name__ == "__main__":
    raise SystemExit(main())
