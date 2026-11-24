# Quillnote

Quillnote is a desktop app for Markdown notes. A writer keeps a folder of
notes, edits them in a plain editor with a live preview, and exports a note
as a page to share.

## What it is made of

- `src/quillnote/markdown.py` — reads a note's title and its headings, and
  renders the note to HTML.
- `src/quillnote/notes.py` — the notes folder: list, open, save.
- `data/notes/` — the sample notes the app opens on first run.

## Running it

    python3 -m quillnote.app ~/Notes     # open the app on a notes folder

## How the team works

A note is never lost: a save writes a temporary file and then renames it.
Every change to the editor is reviewed before it reaches the main branch.
