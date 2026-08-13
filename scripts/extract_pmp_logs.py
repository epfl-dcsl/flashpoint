#!/usr/bin/env python3
"""Extract and label PMP snapshots from a FlashPoint boot log."""

from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path


PMP_LINE = re.compile(
    r"^\[(?:DEBUG|INFO) \| (?P<source>anchor|tpm-driver)\] "
    r"hart (?P<hart>\d+) - PMP\s+(?P<entry>\d+)\b"
)


@dataclass
class Snapshot:
    source: str
    hart: int
    lines: list[str]
    entries: list[int]


# Occurrences are counted independently for each (source, hart) pair.  This is
# important because output from different harts is interleaved during reset and
# initialization.
LABELS: dict[tuple[str, int], list[str]] = {
    ("anchor", 0): [
        "Reset",
        "After initialization / before SRTM",
        "After SRTM / before untrusted firmware",
        "After untrusted firmware / before DRTM",
        "After DRTM / before security monitor",
    ],
    ("anchor", 1): [
        "Reset",
        "After initialization / before untrusted firmware",
        "After untrusted firmware / before security monitor",
    ],
    ("tpm-driver", 0): [
        "Inside TPM driver during SRTM",
        "Inside TPM driver during DRTM",
    ],
}

# Desired report order is by boot phase, not by hart or by the timing of
# interleaved log lines.  The non-boot hart can reach its pre-SM wait while the
# boot hart is still performing DRTM.
REPORT_ORDER = [
    (("anchor", 0), 0), (("anchor", 1), 0),
    (("anchor", 0), 1), (("anchor", 1), 1),
    (("tpm-driver", 0), 0),
    (("anchor", 0), 2),
    (("anchor", 0), 3),
    (("tpm-driver", 0), 1),
    (("anchor", 1), 2), (("anchor", 0), 4),
]


def extract_snapshots(lines: list[str]) -> dict[tuple[str, int], list[Snapshot]]:
    snapshots: dict[tuple[str, int], list[Snapshot]] = defaultdict(list)
    current: dict[tuple[str, int], Snapshot] = {}

    for raw_line in lines:
        line = raw_line.rstrip("\r\n")
        match = PMP_LINE.match(line)
        if not match:
            continue

        source = match.group("source")
        hart = int(match.group("hart"))
        entry = int(match.group("entry"))
        key = (source, hart)

        # PMP 0 marks a new dump for this source/hart.  Dumps belonging to
        # other harts may continue between its entries.
        if entry == 0:
            snapshot = Snapshot(source, hart, [], [])
            snapshots[key].append(snapshot)
            current[key] = snapshot
        elif key not in current:
            # Retain malformed/partial input instead of silently losing it.
            snapshot = Snapshot(source, hart, [], [])
            snapshots[key].append(snapshot)
            current[key] = snapshot

        current[key].lines.append(line)
        current[key].entries.append(entry)

    return snapshots


def snapshot_label(key: tuple[str, int], occurrence: int) -> str:
    labels = LABELS.get(key, [])
    if occurrence < len(labels):
        return labels[occurrence]
    return f"Unclassified snapshot {occurrence + 1}"


def render(snapshots: dict[tuple[str, int], list[Snapshot]]) -> str:
    sections: list[str] = ["=== PMP snapshots by boot phase ===", ""]
    emitted: set[tuple[tuple[str, int], int]] = set()

    ordered = list(REPORT_ORDER)
    # Unknown harts and extra occurrences are still reported at the end.
    ordered.extend(
        (key, occurrence)
        for key in sorted(snapshots)
        for occurrence in range(len(snapshots[key]))
        if (key, occurrence) not in ordered
    )
    for key, occurrence in ordered:
        if occurrence < len(snapshots.get(key, [])) and (key, occurrence) not in emitted:
            emitted.add((key, occurrence))
            snapshot = snapshots[key][occurrence]
            label = snapshot_label(key, occurrence)
            sections.append(f"--- {label} | Hart {snapshot.hart} ---")
            sections.extend(snapshot.lines)
            sections.append("")

            expected = list(range(16))
            if snapshot.entries != expected:
                print(
                    f"warning: {snapshot.source} hart {snapshot.hart}, {label}: "
                    f"expected PMP entries 0-15, found {snapshot.entries}",
                    file=sys.stderr,
                )

    return "\n".join(sections).rstrip() + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Sort FlashPoint PMP log lines into labeled phase/hart snapshots."
    )
    parser.add_argument("log_file", type=Path, help="QEMU boot log to parse")
    parser.add_argument(
        "-o", "--output", type=Path, help="write output here instead of standard output"
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        lines = args.log_file.read_text(encoding="utf-8", errors="replace").splitlines(keepends=True)
    except OSError as error:
        print(f"error: cannot read {args.log_file}: {error}", file=sys.stderr)
        return 1

    snapshots = extract_snapshots(lines)
    if not snapshots:
        print("error: no anchor or TPM-driver PMP entries found", file=sys.stderr)
        return 1

    output = render(snapshots)
    if args.output:
        try:
            args.output.write_text(output, encoding="utf-8")
        except OSError as error:
            print(f"error: cannot write {args.output}: {error}", file=sys.stderr)
            return 1
    else:
        sys.stdout.write(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
