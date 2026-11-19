#!/usr/bin/env python3
"""Regenerate the ranges of scripts/terminal-columns.txt from its rules.

The rules are the `rule` lines at the top of that file; this script encodes no
width of its own. `--check` fails when the committed ranges differ from the
ones the rules generate, so the table's "generated from the rules" claim is a
claim the gate reads. `--write` rewrites the ranges in place.

Only the range block is derived: the rules, the header and the cases are
written by hand. The block is the lines between the header and the first case
line, each `START-END WIDTH`.

Standard library only.

    python3 scripts/generate-terminal-columns.py --check
"""

import argparse
import re
import sys
import unicodedata
from pathlib import Path

TABLE_PATH = Path(__file__).with_name("terminal-columns.txt")

LAST_CODEPOINT = 0x10FFFF

# The range readers take one column for a codepoint no `rule` line holds. A
# default rule that says otherwise would be a rule the readers do not read.
DEFAULT_WIDTH = 1

# The table's header names the Unicode version its rules were generated from.
# A different Python's unicodedata is a different table, so the probe refuses
# to pass one its rules did not generate.
UNICODE_VERSION = re.compile(r"Unicode ([0-9]+\.[0-9]+\.[0-9]+)")


def declared_unicode(text):
    match = UNICODE_VERSION.search(text)
    if match is None:
        raise ValueError("the table's header does not name its Unicode version")
    return match.group(1)


def width_value(value, number):
    """The width a rule line names, as an int or `tab` or `join`."""
    if value in ("tab", "join"):
        return value
    try:
        width = int(value)
    except ValueError:
        width = None
    if width not in (0, 1, 2):
        raise ValueError(
            f"line {number}: a width is 0, 1, 2, tab or join, not {value}"
        )
    return width


def parse_range(field, number):
    """A `START-END` hex range from a rule line."""
    start, separator, end = field.partition("-")
    if not separator:
        raise ValueError(f"line {number}: want a START-END range, not {field}")
    start = int(start, 16)
    end = int(end, 16)
    if start > end:
        raise ValueError(f"line {number}: range {field} runs backwards")
    return start, end


def parse_rules(text):
    """The rules the table's header states.

    The `rule` lines are the source: this probe reads them and derives the
    ranges. A rule the table does not state cannot be enforced, and a rule
    edited here without regenerating the ranges fails `--check`.
    """
    tab_stop = None
    join_after = None
    default = None
    rules = []
    for number, raw in enumerate(text.split("\n"), start=1):
        fields = raw.split()
        if not fields or fields[0] != "rule":
            continue
        if len(fields) < 3:
            raise ValueError(
                f"line {number}: want rule NAME WIDTH SELECTOR ARG... "
                f"or rule default W"
            )
        kind = fields[1]
        if kind == "tab-stop":
            if len(fields) != 3:
                raise ValueError(f"line {number}: want rule tab-stop N")
            tab_stop = int(fields[2])
            if tab_stop < 1:
                raise ValueError(f"line {number}: the tab stop must be positive")
        elif kind == "join-after":
            if len(fields) != 3:
                raise ValueError(f"line {number}: want rule join-after CP")
            join_after = int(fields[2], 16)
            if not 0 <= join_after <= LAST_CODEPOINT:
                raise ValueError(f"line {number}: not a codepoint: {fields[2]}")
        elif kind == "default":
            if len(fields) != 3:
                raise ValueError(f"line {number}: want rule default W")
            default = width_value(fields[2], number)
        else:
            if len(fields) < 5:
                raise ValueError(
                    f"line {number}: want rule NAME WIDTH SELECTOR ARG..."
                )
            width = width_value(fields[2], number)
            selector = fields[3]
            arguments = fields[4:]
            if selector == "range":
                values = tuple(parse_range(field, number) for field in arguments)
            elif selector in ("category", "east-asian-width"):
                values = tuple(arguments)
            else:
                raise ValueError(
                    f"line {number}: a selector is category, "
                    f"east-asian-width or range, not {selector}"
                )
            rules.append((width, selector, values))
    if tab_stop is None:
        raise ValueError("the table's rules do not name a tab stop")
    if join_after is None:
        raise ValueError("the table's rules do not name a join-after codepoint")
    if default is None:
        raise ValueError("the table's rules do not name a default width")
    if default != DEFAULT_WIDTH:
        raise ValueError(
            f"the range readers take {DEFAULT_WIDTH} for a codepoint no range "
            f"holds; the default rule says {default}"
        )
    if not rules:
        raise ValueError("the table carries no width rules")
    return tab_stop, join_after, rules


def columns(codepoint, rules):
    """The range width for a codepoint, or None for the default."""
    character = chr(codepoint)
    category = unicodedata.category(character)
    east_asian_width = None
    for width, selector, values in rules:
        if selector == "range":
            if any(start <= codepoint <= end for start, end in values):
                return width
        elif selector == "category":
            if category in values:
                return width
        elif selector == "east-asian-width":
            if east_asian_width is None:
                east_asian_width = unicodedata.east_asian_width(character)
            if east_asian_width in values:
                return width
    return None


def ranges(rules):
    """Every run of codepoints with one width, default runs left out."""
    out = []
    start = None
    end = None
    width = None
    for codepoint in range(LAST_CODEPOINT + 1):
        current = columns(codepoint, rules)
        if start is not None and current == width and codepoint == end + 1:
            end = codepoint
            continue
        if start is not None and width is not None:
            out.append((start, end, width))
        start, end, width = codepoint, codepoint, current
    if start is not None and width is not None:
        out.append((start, end, width))
    return out


def range_lines(rules):
    return [f"{start:04X}-{end:04X} {width}" for start, end, width in ranges(rules)]


def is_range_line(line):
    """Whether a reader would read this line as a range.

    The readers strip comments and accept any whitespace and lowercase hex, so
    this recogniser does too: a range the readers read cannot hide from the
    probe by being spelled differently.
    """
    fields = line.split()
    if len(fields) != 2:
        return False
    start, separator, end = fields[0].partition("-")
    if not separator:
        return False
    try:
        int(start, 16)
        int(end, 16)
    except ValueError:
        return False
    if fields[1] in ("tab", "join"):
        return True
    try:
        return int(fields[1]) in (0, 1, 2)
    except ValueError:
        return False


def split_table(text):
    """The lines before the ranges, and the lines after them.

    Every range line must belong to the one contiguous block. A range planted
    beyond it is read by the sweep and the Rust check, so it has to fail here
    too: the probe holds over every range the readers read.
    """
    lines = text.split("\n")
    indices = []
    for index, raw in enumerate(lines):
        line = raw.split("#", 1)[0].strip()
        if line and is_range_line(line):
            indices.append(index)
    if not indices:
        raise ValueError("the table carries no range block")
    first = indices[0]
    for offset, index in enumerate(indices):
        if index != first + offset:
            raise ValueError(
                f"line {index + 1}: a range outside the range block"
            )
    return lines[:first], lines[indices[-1] + 1 :]


def reproduce(text):
    """The committed table with its ranges regenerated from the rules."""
    _, _, rules = parse_rules(text)
    before, after = split_table(text)
    return "\n".join(before + range_lines(rules) + after)


def first_difference(committed, generated):
    for number, (want, got) in enumerate(
        zip(committed.split("\n"), generated.split("\n")), start=1
    ):
        if want != got:
            return f"line {number}: committed {want!r}, generated {got!r}"
    return "the files differ in length"


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Regenerate the ranges of scripts/terminal-columns.txt "
        "from the rules in its header.",
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="rewrite the table's ranges instead of checking them",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail when the committed ranges differ (the default)",
    )
    args = parser.parse_args(argv)
    if args.write and args.check:
        parser.error("--write and --check are exclusive")

    try:
        committed = TABLE_PATH.read_text(encoding="utf-8")
        declared = declared_unicode(committed)
        if declared != unicodedata.unidata_version:
            print(
                f"terminal-columns: this Python's Unicode is "
                f"{unicodedata.unidata_version}; the table's rules are "
                f"Unicode {declared}. Regenerate with the pinned interpreter.",
                file=sys.stderr,
            )
            return 1
        generated = reproduce(committed)
    except (OSError, ValueError) as error:
        print(f"terminal-columns: {error}", file=sys.stderr)
        return 1

    if args.write:
        if generated != committed:
            TABLE_PATH.write_text(generated, encoding="utf-8")
            print(f"terminal-columns: wrote {TABLE_PATH}")
        return 0

    if generated != committed:
        print(
            f"terminal-columns: {TABLE_PATH} is not what its rules generate: "
            f"{first_difference(committed, generated)}",
            file=sys.stderr,
        )
        return 1
    _, _, rules = parse_rules(committed)
    print(
        f"terminal-columns: {len(range_lines(rules))} ranges agree with the rules"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
