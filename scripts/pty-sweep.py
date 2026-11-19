#!/usr/bin/env python3
"""Sweep raul's plain render across widths and catch a line that overflows.

For every width from 40 to 120 the sweep runs

    raul --render --plain --width W --height H [work-folder]

and writes the output to `<out>/<W>.txt`. It measures every line in terminal
columns; a line wider than W fails the command, naming the width, the line
number and the measured length. The rules live in `terminal-columns.txt`
beside this file, which the Rust width check reads too. A render that does
not finish within `--timeout` seconds fails too, so a hung raul cannot stall
the gate.

The output folder is reported and kept, so a failing render can be read. It
defaults to a fresh folder under the system temporary directory.

Run from the repository root:

    python3 scripts/pty-sweep.py --raul target/debug/raul \
        --work mock-data/coding-lanes/work
"""

import argparse
import bisect
import subprocess
import sys
import tempfile
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

# The one table of terminal-column rules, read here and by
# crates/raul/tests/render_plain.rs. Neither measure carries its own rules.
TABLE_PATH = Path(__file__).with_name("terminal-columns.txt")

# A codepoint the table does not name takes one column.
DEFAULT_COLUMNS = 1


class ColumnTable:
    """The terminal-column rules in `terminal-columns.txt`.

    A range gives a codepoint's fixed columns, or `tab` for the one cell that
    advances to the next tab stop. A codepoint in no range takes
    `DEFAULT_COLUMNS`. The cases at the foot of the file are the rules pinned
    on named characters; `check` measures them through this same table.
    """

    def __init__(self, tab_stop, ranges, cases):
        self.tab_stop = tab_stop
        self.ranges = ranges
        self.cases = cases
        self._starts = [start for start, _, _ in ranges]

    @classmethod
    def load(cls, path):
        """Read a table file, refusing a malformed one."""
        tab_stop = None
        ranges = []
        cases = []
        for number, raw in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            line = raw.split("#", 1)[0].strip()
            if not line:
                continue
            fields = line.split()
            try:
                if fields[0] == "tab-stop":
                    if len(fields) != 2:
                        raise ValueError("want: tab-stop N")
                    tab_stop = int(fields[1])
                    if tab_stop < 1:
                        raise ValueError("the tab stop must be positive")
                elif fields[0] == "case":
                    if len(fields) != 4:
                        raise ValueError("want: case NAME CODEPOINTS COLUMNS")
                    text = "".join(
                        chr(int(codepoint, 16))
                        for codepoint in fields[2].split(",")
                    )
                    cases.append((fields[1], text, int(fields[3])))
                else:
                    if len(fields) != 2:
                        raise ValueError("want: START-END WIDTH")
                    start, end = (
                        int(bound, 16) for bound in fields[0].split("-")
                    )
                    width = fields[1]
                    if width != "tab":
                        width = int(width)
                        if width not in (0, 2):
                            raise ValueError(
                                f"a range takes 0 or 2 columns, not {width}"
                            )
                    if start > end:
                        raise ValueError(f"range {fields[0]} runs backwards")
                    ranges.append((start, end, width))
            except ValueError as error:
                raise ValueError(f"{path}:{number}: {error}") from None
        if tab_stop is None:
            raise ValueError(f"{path}: no tab-stop line")
        if not ranges:
            raise ValueError(f"{path}: no ranges")
        if not cases:
            raise ValueError(f"{path}: no cases")
        previous = -1
        for start, end, _ in ranges:
            if start <= previous:
                raise ValueError(f"{path}: ranges are not in order")
            previous = end
        return cls(tab_stop, ranges, cases)

    def advance(self, character, column):
        """The columns `character` adds when it starts at `column`."""
        codepoint = ord(character)
        index = bisect.bisect_right(self._starts, codepoint) - 1
        if index >= 0:
            start, end, width = self.ranges[index]
            if start <= codepoint <= end:
                if width == "tab":
                    return self.tab_stop - column % self.tab_stop
                return width
        return DEFAULT_COLUMNS

    def display_width(self, text):
        """The terminal columns a line takes, a tab advancing in place."""
        column = 0
        for character in text:
            column += self.advance(character, column)
        return column

    def check(self):
        """Measure the table's cases through the table; return the failures."""
        failures = []
        for name, text, expected in self.cases:
            measured = self.display_width(text)
            if measured != expected:
                failures.append(
                    f"case {name}: measured {measured} columns, "
                    f"the table says {expected}"
                )
        return failures


def load_table():
    """The table both measures read."""
    try:
        return ColumnTable.load(TABLE_PATH)
    except (OSError, ValueError) as error:
        raise SystemExit(f"sweep: cannot read {TABLE_PATH}: {error}") from None


def measure_stdin(table):
    """One measured line of stdin per output line."""
    for row in sys.stdin.read().splitlines():
        print(table.display_width(row))
    return 0


def check_the_table(table):
    """Report the table's own cases, then whether they all agree."""
    for name, text, expected in table.cases:
        print(f"{name}\t{table.display_width(text)}\t{expected}")
    failures = table.check()
    for failure in failures:
        print(f"FAIL: table: {failure}", file=sys.stderr)
    if failures:
        print(
            f"table: {len(table.cases)} cases, "
            f"{len(failures)} disagree",
            file=sys.stderr,
        )
        return 1
    print(f"table: {len(table.cases)} cases agree")
    return 0


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
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument(
        "--check-table",
        action="store_true",
        help="measure the table's cases and exit; no render is run",
    )
    modes.add_argument(
        "--measure",
        action="store_true",
        help="measure lines of stdin in terminal columns and exit",
    )
    return parser.parse_args(argv)


def rows_of(text):
    """The lines of an output that ends every row with a newline."""
    if text.endswith("\n"):
        text = text[:-1]
    return text.split("\n") if text else []


def sweep(args, out, table):
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
            measured = table.display_width(row)
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
    table = load_table()
    if args.check_table:
        return check_the_table(table)
    if args.measure:
        return measure_stdin(table)

    # The rule tables are asserted on every sweep too: a measure that drifts
    # from the table fails here, before a render is read.
    failures = table.check()
    if failures:
        for failure in failures:
            print(f"FAIL: table: {failure}", file=sys.stderr)
        return 1

    if args.out is None:
        out = Path(tempfile.mkdtemp(prefix="raul-pty-sweep-"))
    else:
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
    print(f"sweep output: {out}", flush=True)

    failures = sweep(args, out, table)
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
