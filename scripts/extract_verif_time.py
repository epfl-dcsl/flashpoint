#!/usr/bin/env python3
"""Report Kani verification time per harness in hours."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


HARNESS_LINE = re.compile(r"^Checking harness (?P<harness>.+?)\.\.\.\s*$")
# Accept the spelling used by Kani as well as "Vertification" from older logs.
TIME_LINE = re.compile(
    r"^Ver(?:t)?ification Time:\s*(?P<seconds>\d+(?:\.\d+)?)s\s*$"
)


def extract_times(lines: list[str]) -> list[tuple[str, float]]:
    results: list[tuple[str, float]] = []
    current_harness: str | None = None

    for line in lines:
        harness_match = HARNESS_LINE.match(line.strip())
        if harness_match:
            current_harness = harness_match.group("harness")
            continue

        time_match = TIME_LINE.match(line.strip())
        if time_match and current_harness is not None:
            hours = float(time_match.group("seconds")) / 3600
            results.append((current_harness, hours))
            current_harness = None

    return results


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Extract Kani verification times and report them in hours."
    )
    parser.add_argument(
        "log_file",
        type=Path,
        help="Kani log file to parse",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        lines = args.log_file.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as error:
        print(f"error: cannot read {args.log_file}: {error}", file=sys.stderr)
        return 1

    times = extract_times(lines)
    if not times:
        print(f"error: no verification times found in {args.log_file}", file=sys.stderr)
        return 1

    for harness, hours in times:
        print(f"{harness}: {hours:.3f} hours")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
