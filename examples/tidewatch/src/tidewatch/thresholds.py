"""The high mark each harbour sets. A reading above it is a tide alert."""

import csv
import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def high_marks():
    """Map each harbour id to its high mark in metres."""
    harbours = json.loads((DATA / "harbours.json").read_text())["harbours"]
    return {h["id"]: h["high_mark_m"] for h in harbours}


def readings():
    """Yield (gauge, harbour, level_m) for every row of gauges.csv."""
    with (DATA / "gauges.csv").open() as handle:
        for row in csv.DictReader(handle):
            yield row["gauge"], row["harbour"], float(row["level_m"])


def over_high_mark(level_m, harbour, marks):
    return level_m > marks[harbour]

SURGE_LIMIT_M = 0.6


def surge_limit(harbour):
    """The surge, in metres above the high mark, that sends a storm alert."""
    return SURGE_LIMIT_M
