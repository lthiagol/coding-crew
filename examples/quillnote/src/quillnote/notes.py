"""The notes folder: list, open and save. A save writes a temporary file, then renames it."""

import os
from pathlib import Path


def list_notes(folder):
    return sorted(p.name for p in Path(folder).glob("*.md"))


def open_note(folder, name):
    return (Path(folder) / name).read_text(encoding="utf-8")


def save_note(folder, name, text):
    target = Path(folder) / name
    temp = target.with_suffix(".md.tmp")
    temp.write_text(text, encoding="utf-8")
    os.replace(temp, target)


def read_note(folder, name):
    """Return (title, body) for a note; a missing file raises FileNotFoundError."""
    text = open_note(folder, name)
    lines = text.splitlines()
    title = next((line[2:].strip() for line in lines if line.startswith("# ")), name)
    return title, text


def autosave(folder, name, text, cursor):
    """Save the note and return the cursor, so the editor puts it back where it was."""
    save_note(folder, name, text)
    return cursor


TITLES = {}


def index_titles(folder):
    """Map each note's title to its file name, for search."""
    for name in list_notes(folder):
        title, _ = read_note(folder, name)
        TITLES[title] = name
    return TITLES
