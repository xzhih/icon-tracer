#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCAN_PATHS = (
    ".github",
    "AGENTS.md",
    "README.md",
    "docs",
    "scripts",
    "src",
    "tests",
)
LINE_LIMIT = 1000
REVIEWABLE_SUFFIXES = {".py", ".rs"}
TEXT_SUFFIXES = {".md", ".py", ".rs", ".toml", ".yaml", ".yml"}
SKIP_DIRS = {".git", "target", "__pycache__"}


def marker_terms() -> tuple[str, ...]:
    return (
        "TO" + "DO",
        "FIX" + "ME",
        "X" + "XX",
        "in" + "_progress",
        "un" + "finished",
        "T" + "BD",
        "PLACE" + "HOLDER",
        "未" + "完成",
        "待" + "办",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run lightweight repository governance checks."
    )
    parser.add_argument(
        "--max-lines",
        type=int,
        default=LINE_LIMIT,
        help=f"maximum allowed lines for reviewable source/script/test files (default: {LINE_LIMIT})",
    )
    parser.add_argument(
        "paths",
        nargs="*",
        default=list(DEFAULT_SCAN_PATHS),
        help="project-relative files or directories to scan",
    )
    return parser.parse_args()


def project_path(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def iter_files(paths: list[str]) -> list[Path]:
    files: list[Path] = []
    for raw_path in paths:
        path = (ROOT / raw_path).resolve()
        if not path.exists():
            continue
        if path.is_file():
            files.append(path)
            continue
        for candidate in path.rglob("*"):
            if any(part in SKIP_DIRS for part in candidate.relative_to(ROOT).parts):
                continue
            if candidate.is_file():
                files.append(candidate)
    return sorted(set(files))


def read_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return None


def check_required_docs() -> list[str]:
    failures: list[str] = []
    for required in ("AGENTS.md", "docs/governance.md", "README.md"):
        if not (ROOT / required).is_file():
            failures.append(f"missing required governance file: {required}")
    return failures


def check_readme_link() -> list[str]:
    readme = ROOT / "README.md"
    text = read_text(readme)
    if text is None:
        return ["README.md is not valid UTF-8 text"]
    required = ("scripts/check-governance.py", "docs/governance.md", "AGENTS.md")
    missing = [item for item in required if item not in text]
    if missing:
        return [f"README.md does not reference: {', '.join(missing)}"]
    return []


def check_line_limits(files: list[Path], max_lines: int) -> tuple[list[str], tuple[str, int]]:
    failures: list[str] = []
    largest = ("", 0)
    for path in files:
        if path.suffix not in REVIEWABLE_SUFFIXES:
            continue
        relative = path.relative_to(ROOT)
        if not relative.parts or relative.parts[0] not in {"scripts", "src", "tests"}:
            continue
        text = read_text(path)
        if text is None:
            continue
        line_count = text.count("\n") + (0 if text.endswith("\n") or not text else 1)
        if line_count > largest[1]:
            largest = (project_path(path), line_count)
        if line_count > max_lines:
            failures.append(f"{project_path(path)} has {line_count} lines > {max_lines}")
    return failures, largest


def check_open_markers(files: list[Path]) -> list[str]:
    failures: list[str] = []
    terms = marker_terms()
    for path in files:
        if path.suffix not in TEXT_SUFFIXES:
            continue
        text = read_text(path)
        if text is None:
            continue
        for line_number, line in enumerate(text.splitlines(), start=1):
            matches = [term for term in terms if term in line]
            if matches:
                failures.append(
                    f"{project_path(path)}:{line_number} contains open-work marker {matches[0]!r}"
                )
    return failures


def main() -> int:
    args = parse_args()
    files = iter_files(args.paths)

    failures: list[str] = []
    failures.extend(check_required_docs())
    failures.extend(check_readme_link())

    line_failures, largest = check_line_limits(files, args.max_lines)
    failures.extend(line_failures)
    failures.extend(check_open_markers(files))

    if failures:
        print("governance check: failed", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    largest_path, largest_lines = largest
    print("governance check: ok")
    print("required docs: ok")
    if largest_path:
        print(f"largest reviewable file: {largest_path} ({largest_lines}/{args.max_lines})")
    print("open-work markers: none")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
