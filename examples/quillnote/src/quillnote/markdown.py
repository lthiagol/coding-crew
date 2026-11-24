"""Reads a note's title and headings, and renders it to HTML."""

import html


def title(text):
    """The first level-one heading, or an empty string."""
    for line in text.splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return ""


def headings(text):
    """Each heading as (level, text), in order."""
    found = []
    for line in text.splitlines():
        stripped = line.lstrip("#")
        level = len(line) - len(stripped)
        if 0 < level <= 6 and stripped.startswith(" "):
            found.append((level, stripped.strip()))
    return found


def render(text):
    """A small renderer: headings, bullet lists and paragraphs."""
    out, in_list = [], False
    for line in text.splitlines():
        if line.startswith("- "):
            if not in_list:
                out.append("<ul>")
                in_list = True
            out.append(f"<li>{html.escape(line[2:])}</li>")
            continue
        if in_list:
            out.append("</ul>")
            in_list = False
        level = len(line) - len(line.lstrip("#"))
        if 0 < level <= 6 and line[level:level + 1] == " ":
            out.append(f"<h{level}>{html.escape(line[level + 1:])}</h{level}>")
        elif line.strip():
            out.append(f"<p>{html.escape(line)}</p>")
    if in_list:
        out.append("</ul>")
    return "\n".join(out)
