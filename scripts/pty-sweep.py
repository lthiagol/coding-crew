#!/usr/bin/env python3
"""Sweep raul's plain render across widths and catch a line that overflows.

For every width from 40 to 120 the sweep runs

    raul --render --plain --width W --height H [work-folder]

and writes the output to `<out>/<W>.txt`. It measures every line in terminal
columns; a line wider than W fails the command, naming the width, the line
number and the measured length. A render that does not finish within
`--timeout` seconds fails too, so a hung raul cannot stall the gate.

The output folder is reported and kept, so a failing render can be read. It
defaults to a fresh folder under the system temporary directory.

Run from the repository root:

    python3 scripts/pty-sweep.py --raul target/debug/raul \
        --work mock-data/coding-lanes/work
"""

import argparse
import subprocess
import sys
import tempfile
import unicodedata
from pathlib import Path

# The width range the sweep covers. It starts at the narrowest width the tabs
# are designed for and ends at the render mode's default width.
FIRST_WIDTH = 40
LAST_WIDTH = 120

# The render mode's default height (crates/raul/src/main.rs). It is also the
# row count every output file must carry.
DEFAULT_HEIGHT = 40

# The largest dimension raul accepts (crates/raul/src/main.rs).
MAX_DIMENSION = 1000

# A render is a bounded process: a hung one fails the sweep instead of
# stalling the gate.
DEFAULT_TIMEOUT = 30


def positive_dimension(value):
    """A positive dimension up to the limit raul accepts."""
    try:
        number = int(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"not a number: {value}") from None
    if number < 1 or number > MAX_DIMENSION:
        raise argparse.ArgumentTypeError(
            f"want a positive number up to {MAX_DIMENSION}, not {value}"
        )
    return number


def positive_seconds(value):
    """A positive number of seconds."""
    try:
        number = int(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"not a number: {value}") from None
    if number < 1:
        raise argparse.ArgumentTypeError(
            f"want a positive number of seconds, not {value}"
        )
    return number


def parse_args(argv):
    parser = argparse.ArgumentParser(
        description="Sweep raul --render --plain from 40 to 120 columns and "
        "fail when a line is wider than its width.",
    )
    parser.add_argument(
        "--raul",
        default="target/debug/raul",
        metavar="PATH",
        help="the raul binary (default: target/debug/raul)",
    )
    parser.add_argument(
        "--work",
        metavar="PATH",
        help="the work folder passed to raul; omitted, raul discovers it",
    )
    parser.add_argument(
        "--height",
        type=positive_dimension,
        default=DEFAULT_HEIGHT,
        metavar="N",
        help=f"the rows of each render (default: {DEFAULT_HEIGHT})",
    )
    parser.add_argument(
        "--out",
        metavar="DIR",
        help="where to write one file per width (default: a fresh temp folder)",
    )
    parser.add_argument(
        "--timeout",
        type=positive_seconds,
        default=DEFAULT_TIMEOUT,
        metavar="SECONDS",
        help=f"the bound on one render (default: {DEFAULT_TIMEOUT}s)",
    )
    return parser.parse_args(argv)


def cell_width(character):
    """The terminal columns one character takes."""
    # Combining marks, format characters and the zero-width joiner take no
    # column; wide and fullwidth characters take two.
    if unicodedata.category(character) in ("Mn", "Me", "Cf"):
        return 0
    if unicodedata.east_asian_width(character) in ("W", "F"):
        return 2
    return 1


def display_width(text):
    """The terminal columns a line takes."""
    return sum(cell_width(character) for character in text)


def rows_of(text):
    """The lines of an output that ends every row with a newline."""
    if text.endswith("\n"):
        text = text[:-1]
    return text.split("\n") if text else []


def sweep(args, out):
    """Run every width; return one `(width, message)` pair per failure."""
    failures = []
    for width in range(FIRST_WIDTH, LAST_WIDTH + 1):
        command = [
            args.raul,
            "--render",
            "--plain",
            "--width",
            str(width),
            "--height",
            str(args.height),
        ]
        if args.work is not None:
            command.append(args.work)
        try:
            result = subprocess.run(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
                timeout=args.timeout,
            )
        except subprocess.TimeoutExpired:
            (out / f"{width}.txt").write_bytes(b"")
            failures.append(
                (width, f"width {width}: raul did not finish within {args.timeout}s")
            )
            continue
        except OSError as error:
            failures.append(
                (width, f"width {width}: could not run {args.raul}: {error}")
            )
            continue

        # The file is written even when the width fails: the artifact is what
        # a caller reads to see the overflow.
        (out / f"{width}.txt").write_bytes(result.stdout)

        if result.returncode != 0:
            stderr = result.stderr.decode("utf-8", "replace").strip()
            failures.append(
                (width, f"width {width}: raul exited {result.returncode}: {stderr}")
            )
            continue

        text = result.stdout.decode("utf-8", "replace")
        rows = rows_of(text)
        if len(rows) != args.height:
            failures.append(
                (
                    width,
                    f"width {width}: raul printed {len(rows)} rows, "
                    f"expected {args.height}",
                )
            )
        for number, row in enumerate(rows, start=1):
            measured = display_width(row)
            if measured > width:
                failures.append(
                    (
                        width,
                        f"width {width}: line {number} is {measured} columns "
                        f"(limit {width}): {row!r}",
                    )
                )
    return failures


def main(argv=None):
    args = parse_args(argv)
    if args.out is None:
        out = Path(tempfile.mkdtemp(prefix="raul-pty-sweep-"))
    else:
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
    print(f"sweep output: {out}", flush=True)

    failures = sweep(args, out)
    widths = LAST_WIDTH - FIRST_WIDTH + 1
    if failures:
        for _, failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        failed_widths = sorted({width for width, _ in failures})
        print(
            f"sweep: {widths} widths {FIRST_WIDTH}-{LAST_WIDTH}, height "
            f"{args.height}: {len(failed_widths)} width"
            f"{'' if len(failed_widths) == 1 else 's'} failed; output kept in {out}",
            file=sys.stderr,
        )
        return 1

    print(
        f"sweep: {widths} widths {FIRST_WIDTH}-{LAST_WIDTH}, height "
        f"{args.height}: every line fits; output in {out}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
