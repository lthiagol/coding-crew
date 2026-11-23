#!/usr/bin/env python3
"""Generate the mock task fixtures for coding-crew.

Every fixture satisfies the gates of the phase it is in. The dependency graph
is acyclic, and a task past approved is claimed only after the done work it
waits on has finished, so the set is valid as a whole.

Run from the repo root:

    python3 mock-data/generate.py
"""

import json
import pathlib
import shutil
from datetime import datetime, timedelta

ROOT = pathlib.Path(__file__).resolve().parent
TASKS = ROOT / "coding-crew" / "work"
ACTIVITY = ROOT / "coding-crew" / "activity.json"
RUNTIME = ROOT / "coding-crew" / "runtime.json"

# The models the flow's configuration defaults to (crates/core/src/config.rs).
# The flow files name their agents only. The orchestrator reviews when the
# crew is two, so it runs the reviewer's family: the reviewer is never the
# model that wrote the code.
ORCHESTRATOR_MODEL = "claude-opus-5-5"
RUNNER_MODEL = "claude-haiku-5-5"
REVIEWER_MODEL = "claude-opus-5-5"

# The briefs. The fixtures describe this project's own work, so a task
# opened in raul reads like a task a human could review. Drafts carry a
# title alone; every later phase carries the intent, scope, criteria and
# steps its gates read.
BRIEFS = {
    "TSK01": {
        "title": "The Settings tab remembers the chosen theme",
    },
    "TSK02": {
        "title": "The footer says how many rows it hides",
    },
    "TSK03": {
        "title": "hugo opens the herdr tab and starts its agent",
    },
    "TSK04": {
        "title": "The Needs you row says the review state",
        "outcome": "A Needs you row carries the round spent of the budget and the open findings with their severities.",
        "problem": "A human deciding whether to review a task with five open findings, two of them high, and the budget spent cannot see any of it.",
        "in_scope": [
            "the Needs you row's second line",
            "the review state derived from the rounds and the findings",
        ],
        "out_of_scope": [
            "the detail pane's Review section",
            "the Path row",
        ],
        "criteria": [
            (
                "A row with open findings says how many are open and names the highest severity.",
                "cargo test -p coding-crew-raul needs",
            ),
            (
                "A row whose budget is spent says the round spent of the budget.",
                "cargo test -p coding-crew-raul needs",
            ),
        ],
        "steps": [
            (
                "Derive the review state from the rounds and the findings.",
                "the state is a pure function of the task",
                ["crates/raul/src/needs.rs"],
            ),
            (
                "Put the state on the second line and cut it on a grapheme.",
                "a narrow row keeps whole graphemes",
                ["crates/raul/src/needs.rs"],
            ),
        ],
    },
    "TSK05": {
        "title": "The detail pane keeps its scroll on a redraw",
        "outcome": "The detail pane keeps the line it was on when the document is rebuilt.",
        "problem": "The engine's viewport resets when the document is rebuilt, so a redraw jumps to the top of a long task.",
        "in_scope": [
            "the detail pane's viewport",
            "the rebuild that follows a resize",
        ],
        "out_of_scope": [
            "the row tabs' scrolling",
            "the engine's own viewport code",
        ],
        "criteria": [
            (
                "A resize keeps the first visible line, clamped to the new height.",
                "cargo test -p coding-crew-raul detail",
            ),
            (
                "A document shorter than the pane settles at the top.",
                "cargo test -p coding-crew-raul detail",
            ),
        ],
        "steps": [
            (
                "Save the viewport before the rebuild and restore it after.",
                "the line survives the rebuild",
                ["crates/raul/src/detail.rs"],
            ),
            (
                "Clamp the restored line to the new document's length.",
                "a shortened document has no blank viewport",
                ["crates/raul/src/detail.rs"],
            ),
        ],
    },
    "TSK06": {
        "title": "The Backlog row says the item's body",
        "outcome": "A Backlog row carries the item's own body, cut on a grapheme boundary.",
        "problem": "The generator writes an item's body and the Backlog tab never draws it, so an idea reads as its title alone.",
        "in_scope": [
            "the Backlog row",
            "the cut marker",
        ],
        "out_of_scope": [
            "the smaller-task model",
            "the detail pane",
        ],
        "criteria": [
            (
                "A row with a body shows its first words after the title.",
                "cargo test -p coding-crew-raul backlog",
            ),
            (
                "A body cut by the pane ends in the marker, never in half a grapheme.",
                "cargo test -p coding-crew-raul backlog",
            ),
        ],
        "steps": [
            (
                "Read the body the item already carries and draw it after the title.",
                "the row names the item's body",
                ["crates/raul/src/backlog.rs"],
            ),
            (
                "Cut the body through the shared grapheme helper.",
                "no rendered row splits a grapheme",
                ["crates/raul/src/backlog.rs"],
            ),
        ],
    },
    "TSK07": {
        "title": "The review nests under its round",
        "outcome": "A round is drawn with the findings it raised under it, and a round with none says so.",
        "problem": "The projection flattens the rounds into an eight-column table and repeats the findings as a flat list, so the link between a round and what it found is lost.",
        "in_scope": [
            "the Review section of the projection",
            "the findings a round names",
        ],
        "out_of_scope": [
            "the review model",
            "the round budget",
        ],
        "criteria": [
            (
                "Round 01 is drawn above the findings it raised.",
                "cargo test -p coding-crew-core markdown",
            ),
            (
                "A round with no findings says so on its own line.",
                "cargo test -p coding-crew-core markdown",
            ),
            (
                "A finding that belongs to no round is still shown.",
                "cargo test -p coding-crew-core markdown",
            ),
        ],
        "steps": [
            (
                "Nest each finding under the round that names it.",
                "the finding ids appear under their round",
                ["crates/core/src/markdown.rs"],
            ),
            (
                "Draw the rounds as blocks at the pane's width, not a table.",
                "the Review section reads at forty columns",
                ["crates/core/src/markdown.rs"],
            ),
            (
                "Keep the findings no round names in their own block.",
                "an orphan finding is still on the page",
                ["crates/core/src/markdown.rs"],
            ),
        ],
    },
    "TSK08": {
        "title": "The document gets its own palette",
        "outcome": "The document paints from eighteen roles of its own rather than the chrome's slots.",
        "problem": "The bridge collapses forty-seven markdown fields onto ten chrome slots, and the projection reaches five of them, so alerts, links, list depths and code are dead colour.",
        "in_scope": [
            "the document palette",
            "the bridge that maps the engine's fields",
        ],
        "out_of_scope": [
            "the chrome palette",
            "the terminal preset",
        ],
        "criteria": [
            (
                "Every role in the document palette is painted by some construct.",
                "cargo test -p coding-crew-theme palette",
            ),
            (
                "The terminal preset stays colourless.",
                "cargo test -p coding-crew-theme palette",
            ),
        ],
        "steps": [
            (
                "Write the eighteen roles and map each engine field onto one.",
                "the palette has eighteen named roles",
                ["crates/theme/src/theme.rs"],
            ),
            (
                "Paint the projection's constructs from the roles.",
                "no role in the document palette is dead",
                ["crates/core/src/markdown.rs"],
            ),
        ],
    },
    "TSK09": {
        "title": "The fixtures grow real",
        "outcome": "The mock holds real work in enough shapes that every screen can be judged against it.",
        "problem": "The mock is lorem ipsum: one sentence per field, one criterion, one step, no questions and no children, so a task opened today is a stub.",
        "in_scope": [
            "mock-data/generate.py",
            "the fixtures it writes",
        ],
        "out_of_scope": [
            "the model in crates/core",
            "the projection",
            "raul's screens",
        ],
        "criteria": [
            (
                "Every field the model defines is populated by some fixture.",
                "cargo test -p coding-crew-core mock_data",
            ),
            (
                "The generator reproduces the committed tree byte for byte.",
                "python3 mock-data/generate.py && git diff --exit-code mock-data",
            ),
        ],
        "steps": [
            (
                "Write real briefs, criteria and steps for every phase.",
                "no fixture reads as lorem ipsum",
                ["mock-data/generate.py"],
            ),
            (
                "Add the review, finding and question shapes the screens need.",
                "every nested field has a fixture",
                ["mock-data/generate.py"],
            ),
        ],
    },
    "TSK10": {
        "title": "The Overview counts follow the fixtures",
        "outcome": "Every count the Overview draws is computed from the loaded tasks.",
        "problem": "The Overview's tests hand-keep the counts, so a new fixture leaves the screen right about the old data and wrong about the new.",
        "in_scope": [
            "the Overview's counts",
            "the pins that read the fixtures",
        ],
        "out_of_scope": [
            "the status model",
            "the Path tree",
        ],
        "criteria": [
            (
                "The status counts equal the tasks the store loaded.",
                "cargo test -p coding-crew-raul overview",
            ),
            (
                "The claimable count equals the tasks a start would accept.",
                "cargo test -p coding-crew-raul overview",
            ),
        ],
        "steps": [
            (
                "Count from the Work the pane was given, not from a constant.",
                "the count follows the loaded set",
                ["crates/raul/src/overview.rs"],
            ),
            (
                "Pin the screen against counts derived from the fixtures.",
                "a fixture added alone does not fail the pin",
                ["crates/raul/tests/render_plain.rs"],
            ),
        ],
    },
    "TSK11": {
        "title": "The Path tree announces a dropped branch",
        "kind": "feature",
        "outcome": "A branch the pane cannot fit is named with its count, never dropped silently.",
        "problem": "A short pane drops a whole branch and the trunk looks like the whole tree.",
        "in_scope": [
            "the Path announcement",
            "the window the tree is drawn in",
        ],
        "out_of_scope": [
            "the Needs you tab",
            "the Overview",
        ],
        "criteria": [
            (
                "A dropped branch is named with the rows it hid.",
                "cargo test -p coding-crew-raul path",
            ),
            (
                "A branch that fits is never announced as hidden.",
                "cargo test -p coding-crew-raul path",
            ),
        ],
        "steps": [
            (
                "Collect the branches the window leaves out.",
                "each dropped branch carries its count",
                ["crates/raul/src/path.rs"],
            ),
            (
                "Draw the summary before the trunk when the tree cannot fit.",
                "the announcement names what it hides",
                ["crates/raul/src/path.rs"],
            ),
        ],
    },
    "TSK12": {
        "title": "hugo's queue names the work it hides",
        "kind": "bug",
        "outcome": "The queue's hidden rows are named and counted.",
        "problem": "A narrow main pane cuts the queue without naming what it leaves out.",
        "in_scope": [
            "hugo's queue block",
            "the hidden count",
        ],
        "out_of_scope": [
            "the crew block",
            "the configuration",
        ],
        "criteria": [
            (
                "A cut queue says how many rows it hid.",
                "cargo test -p coding-crew-hugo",
            ),
            (
                "An empty queue says it is empty rather than zero hidden.",
                "cargo test -p coding-crew-hugo",
            ),
        ],
        "steps": [
            (
                "Window the queue and remember the rows outside it.",
                "the window and the count agree",
                ["crates/hugo/src/app.rs"],
            ),
            (
                "Draw the count beside the queue's heading.",
                "the count is on the heading's row",
                ["crates/hugo/src/app.rs"],
            ),
        ],
    },
    "TSK13": {
        "title": "The detail pane paints the marked note",
        "outcome": "A ==marked== span in free text is painted on the selection's colours.",
        "problem": "The engine renders the mark and the pane never gives it a colour, so the mark is a word with two equals signs around it.",
        "in_scope": [
            "the projection's mark",
            "the detail pane's paint",
        ],
        "out_of_scope": [
            "the chrome palette",
            "the Review section",
        ],
        "criteria": [
            (
                "A marked span is painted on text.selected.",
                "cargo test -p coding-crew-core | grep detail",
            ),
            (
                "A fenced rust block keeps syntect's code tokens.",
                "cargo test -p coding-crew-raul detail",
            ),
        ],
        "steps": [
            (
                "Carry the mark through the bridge to the selection's slots.",
                "the marked words take the selection's colours",
                ["crates/raul/src/detail.rs"],
            ),
            (
                "Keep the code block's own paint under the mark.",
                "the fenced block still reads as code",
                ["crates/raul/src/detail.rs"],
            ),
        ],
    },
    "TSK14": {
        "title": "The Needs you row cuts on a grapheme boundary",
        "outcome": "A row that does not fit is cut between whole graphemes.",
        "problem": "A scalar cut leaves a dangling combining mark at the edge of a narrow row.",
        "in_scope": [
            "the Needs you row",
            "the cut helper",
        ],
        "out_of_scope": [
            "the Path row",
            "the footer",
        ],
        "criteria": [
            (
                "A combining mark never starts a row.",
                "cargo test -p coding-crew-raul needs",
            ),
            (
                "The cut marker is the last grapheme, not a half one.",
                "cargo test -p coding-crew-raul needs",
            ),
        ],
        "steps": [
            (
                "Cut the title through the shared helper.",
                "the row keeps whole graphemes",
                ["crates/raul/src/needs.rs"],
            ),
            (
                "Pin the cut against a title built from combining marks.",
                "a scalar cut fails the pin",
                ["crates/raul/src/needs.rs"],
            ),
        ],
    },
    "TSK15": {
        "title": "The Backlog group header says its count",
        "outcome": "Every group header names the rows under it.",
        "problem": "A header that loses its count reads as a group of an unknown size.",
        "in_scope": [
            "the Backlog group headers",
            "the hidden announcement",
        ],
        "out_of_scope": [
            "the Needs you counts",
            "the Overview",
        ],
        "criteria": [
            (
                "A header always names its count.",
                "cargo test -p coding-crew-raul backlog",
            ),
            (
                "A wholly hidden group is announced, not left as a bare header.",
                "cargo test -p coding-crew-raul backlog",
            ),
        ],
        "steps": [
            (
                "Count the rows of each group and draw the count on its header.",
                "the header and the rows agree",
                ["crates/raul/src/backlog.rs"],
            ),
            (
                "Name a hidden group with its count in the announcement.",
                "no hidden group is a bare count",
                ["crates/raul/src/backlog.rs"],
            ),
        ],
    },
    "TSK16": {
        "title": "The round table fits the pane at forty columns",
        "outcome": "The review's rounds are readable at the pane's narrow design width.",
        "problem": "The eight-column round table shrinks every cell at forty columns and abbreviations stand in for words.",
        "in_scope": [
            "the rounds' shape",
            "the forty-column rendering",
        ],
        "out_of_scope": [
            "the review model",
            "the detail pane's header",
        ],
        "criteria": [
            (
                "At forty columns every word of a round is whole.",
                "cargo test -p coding-crew-core markdown",
            ),
            (
                "The round's kind is a word, not a two-letter abbreviation.",
                "cargo test -p coding-crew-core markdown",
            ),
        ],
        "steps": [
            (
                "Replace the table with blocks that wrap.",
                "a round fits a forty-column pane",
                ["crates/core/src/markdown.rs"],
            ),
            (
                "Keep each finding under the round that raised it.",
                "the nesting survives the narrow width",
                ["crates/core/src/markdown.rs"],
            ),
        ],
    },
    "TSK17": {
        "title": "The review budget stops at five rounds",
        "outcome": "A fifth changes-needed round escalates instead of starting a sixth.",
        "problem": "A task that can always ask for one more round reviews itself into completion.",
        "in_scope": [
            "the budget gate",
            "the escalate transition",
        ],
        "out_of_scope": [
            "the round record",
            "the review focus",
        ],
        "criteria": [
            (
                "The fifth changes-needed round escalates.",
                "cargo test -p coding-crew-core lifecycle",
            ),
            (
                "A pass on the fifth round is still accepted.",
                "cargo test -p coding-crew-core lifecycle",
            ),
        ],
        "steps": [
            (
                "Read the external rounds against max_rounds in the gate.",
                "request_changes needs one round left",
                ["crates/core/src/lifecycle.rs"],
            ),
            (
                "Pin the asymmetric edges on the fifth round.",
                "a sixth changes-needed round is refused",
                ["crates/core/tests/task_model.rs"],
            ),
        ],
    },
    "TSK18": {
        "title": "hugo's sidebar survives a broken config",
        "outcome": "A configuration that does not parse is named, left alone, and the defaults are shown beside it.",
        "problem": "A reload that meets a broken file can blank the sidebar or overwrite the file.",
        "in_scope": [
            "the configuration read",
            "the sidebar's error row",
        ],
        "out_of_scope": [
            "the order's gates",
            "the model catalogue",
        ],
        "criteria": [
            (
                "The broken file is named with its error and unchanged.",
                "cargo test -p coding-crew-hugo config",
            ),
            (
                "The defaults are shown beside the error.",
                "cargo test -p coding-crew-hugo config",
            ),
        ],
        "steps": [
            (
                "Keep the read's error and the file's bytes.",
                "the file is byte-identical after a reload",
                ["crates/hugo/src/app.rs"],
            ),
            (
                "Draw the error above the defaults.",
                "the sidebar names both",
                ["crates/hugo/src/app.rs"],
            ),
        ],
    },
    "TSK19": {
        "title": "The document palette tells the heading levels apart",
        "outcome": "The heading levels, the alerts, the links, the list depths and the code are told apart by colour.",
        "problem": "One heading colour for every level makes a long document read as one flat block.",
        "in_scope": [
            "the heading roles",
            "the document palette",
        ],
        "out_of_scope": [
            "the chrome slots",
            "the syntax theme",
        ],
        "criteria": [
            (
                "A first-level heading and a second-level heading take different roles.",
                "cargo test -p coding-crew-theme palette",
            ),
            (
                "An alert takes its own role.",
                "cargo test -p coding-crew-theme palette",
            ),
        ],
        "steps": [
            (
                "Give each level its role.",
                "the levels differ in colour",
                ["crates/theme/src/theme.rs"],
            ),
            (
                "Give the alerts a role of their own.",
                "an alert is not painted as body text",
                ["crates/theme/src/theme.rs"],
            ),
        ],
    },
    "TSK20": {
        "title": "The review nests the findings under the round",
        "outcome": "The Review section reads as a history: each round, then what it found.",
        "problem": "Rounds and findings are drawn as two unrelated lists, so the reader has to match timestamps by eye.",
        "in_scope": [
            "the Review section",
            "the finding-to-round link",
        ],
        "out_of_scope": [
            "the finding model",
            "the row tabs",
        ],
        "criteria": [
            (
                "The finding ids a round names appear under that round.",
                "cargo test -p coding-crew-core markdown",
            ),
            (
                "A finding no round names is still on the page.",
                "cargo test -p coding-crew-core markdown",
            ),
        ],
        "steps": [
            (
                "Group the findings by the rounds that name them.",
                "no finding is painted twice",
                ["crates/core/src/markdown.rs"],
            ),
            (
                "Draw the unclaimed findings after the rounds.",
                "an orphan finding stays visible",
                ["crates/core/src/markdown.rs"],
            ),
        ],
    },
    "TSK21": {
        "title": "The code gutter numbers every line",
        "outcome": "A fenced block carries a numbered gutter down its left edge.",
        "problem": "A long block cannot be referred to by line, so a finding has to quote it.",
        "in_scope": [
            "the fenced code block",
            "the gutter",
        ],
        "out_of_scope": [
            "the syntax theme",
            "inline code",
        ],
        "criteria": [
            (
                "Every line of a block has its number.",
                "cargo test -p coding-crew-engine code",
            ),
            (
                "A wrapped line does not take a second number.",
                "cargo test -p coding-crew-engine code",
            ),
        ],
        "steps": [
            (
                "Draw the gutter beside the code.",
                "the numbers align with the lines",
                ["crates/engine/src/markdown/code.rs"],
            ),
            (
                "Keep the number on the line, not on its wraps.",
                "a wrapped line has one number",
                ["crates/engine/src/markdown/code.rs"],
            ),
        ],
    },
    "TSK22": {
        "title": "The terminal column table is generated from its rules",
        "outcome": "The committed column table is what its rule lines generate.",
        "problem": "A range edited by hand drifts from the rule it claims to unroll.",
        "in_scope": [
            "scripts/terminal-columns.txt",
            "scripts/generate-terminal-columns.py",
        ],
        "out_of_scope": [
            "the sweep's measurements",
            "the width tests",
        ],
        "criteria": [
            (
                "Regenerating the table leaves the committed file unchanged.",
                "python3 scripts/generate-terminal-columns.py --check",
            ),
            (
                "A rule edited without regenerating fails the check.",
                "cargo test -p coding-crew-raul render_plain",
            ),
        ],
        "steps": [
            (
                "Parse the rule lines and derive the ranges from them.",
                "the table's ranges are the rules unrolled",
                ["scripts/generate-terminal-columns.py"],
            ),
            (
                "Run the generator in the gate.",
                "a drifted table fails the gate",
                ["scripts/test-pty-sweep.sh"],
            ),
        ],
    },
    "TSK23": {
        "title": "The Needs you age says under a minute",
        "outcome": "A wait shorter than a minute is said in words, not as zero.",
        "problem": "A row that just entered its status shows an age of zero minutes, which reads as no wait at all.",
        "in_scope": [
            "the age vocabulary",
            "the Needs you row",
        ],
        "out_of_scope": [
            "the Path row",
            "the Overview",
        ],
        "criteria": [
            (
                "A wait under a minute says so.",
                "cargo test -p coding-crew-core age",
            ),
            (
                "The same words are used by both tools.",
                "cargo test -p coding-crew-core age",
            ),
        ],
        "steps": [
            (
                "Share the age words from core.",
                "one vocabulary, two readers",
                ["crates/core/src/clock.rs"],
            ),
            (
                "Draw the under-a-minute case on the row.",
                "a fresh wait is not a zero",
                ["crates/raul/src/needs.rs"],
            ),
        ],
    },
    "TSK24": {
        "title": "The Path tab forks a blocked task under its blocker",
        "outcome": "A blocked task hangs under the first dependency that is not done, and names every dependency it waits on.",
        "problem": "A flat blocked list hides which task is holding the work.",
        "in_scope": [
            "the Path's blocked branch",
            "the fork parent",
        ],
        "out_of_scope": [
            "the overlay and claim cases",
            "the dependency graph",
        ],
        "criteria": [
            (
                "A task with two open dependencies forks under the first.",
                "cargo test -p coding-crew-raul path",
            ),
            (
                "The row names both dependencies.",
                "cargo test -p coding-crew-raul path",
            ),
        ],
        "steps": [
            (
                "Find the first dependency not done.",
                "the fork parent is the first open one",
                ["crates/raul/src/path.rs"],
            ),
            (
                "Group the blocked rows by their fork parent.",
                "each row hangs under its blocker",
                ["crates/raul/src/path.rs"],
            ),
        ],
    },
    "TSK25": {
        "title": "The settings announcement names its hidden groups",
        "outcome": "A short Settings pane names every group it hides.",
        "problem": "The announcement names the groups in view and leaves the hidden ones as a bare count.",
        "in_scope": [
            "the Settings announcement",
            "the group rows",
        ],
        "out_of_scope": [
            "the Backlog announcement",
            "the theme list",
        ],
        "criteria": [
            (
                "Every hidden group is named with its count.",
                "cargo test -p coding-crew-raul settings",
            ),
            (
                "A wholly shown group is not announced as hidden.",
                "cargo test -p coding-crew-raul settings",
            ),
        ],
        "steps": [
            (
                "Collect the groups the window leaves out.",
                "the hidden rows have a count",
                ["crates/raul/src/settings.rs"],
            ),
            (
                "Name them in the same words the Backlog uses.",
                "one announcement vocabulary",
                ["crates/raul/src/settings.rs"],
            ),
        ],
    },
    "TSK26": {
        "title": "The store skips a broken file and names it",
        "outcome": "One unreadable file is skipped, named, and never hides the rest of the folder.",
        "problem": "A reader that stops on the first broken file hides a whole folder behind one bad byte.",
        "in_scope": [
            "the store's listing",
            "the unreadable marker",
        ],
        "out_of_scope": [
            "the JSON shape",
            "the duplicate-id rule",
        ],
        "criteria": [
            (
                "A broken file does not hide the readable tasks.",
                "cargo test -p coding-crew-core store",
            ),
            (
                "The broken file is named with its parse error.",
                "cargo test -p coding-crew-core store",
            ),
        ],
        "steps": [
            (
                "Read each file alone and keep the error beside the list.",
                "a broken file becomes an error row",
                ["crates/core/src/store.rs"],
            ),
            (
                "Draw the error row after the tasks.",
                "the folder is never hidden",
                ["crates/raul/src/overview.rs"],
            ),
        ],
    },
    "TSK27": {
        "title": "The candidate row cuts a wide title",
        "kind": "bug",
        "outcome": "The candidate row cuts a wide title without splitting a grapheme.",
        "problem": "A wide title overflows the row and pushes the state off the edge.",
        "in_scope": [
            "the candidate row",
        ],
        "out_of_scope": [
            "the Backlog rows",
        ],
        "criteria": [
            (
                "A wide title is cut at the row's width.",
                "",
            ),
        ],
        "steps": [],
    },
    "TSK28": {
        "title": "The review nests a round with no findings",
    },
    "TSK29": {
        "title": "The fixtures keep their gates",
        "outcome": "Every fixture satisfies the gates of the phase it is in, and the set tells one consistent history.",
        "problem": "A fixture that breaks a gate teaches the screens a state the model forbids.",
        "in_scope": [
            "the fixture gates",
            "the dependency graph",
        ],
        "out_of_scope": [
            "the model's own gates",
            "the screens",
        ],
        "criteria": [
            (
                "Every fixture passes the gate of its phase.",
                "cargo test -p coding-crew-core mock_data",
            ),
            (
                "The dependency graph is acyclic and closed under done work.",
                "cargo test -p coding-crew-core mock_data",
            ),
        ],
        "steps": [
            (
                "Check every phase's gate in one test.",
                "a fixture that breaks its gate fails the pin",
                ["crates/core/tests/mock_data.rs"],
            ),
            (
                "Check the graph's order and closure.",
                "a claim before its dependency fails the pin",
                ["crates/core/tests/mock_data.rs"],
            ),
        ],
    },
    "TSK30": {
        "title": "The review shows a finding that belongs to no round",
    },
}

PRIORITIES = ["urgent", "high", "normal", "low"]
EFFORTS = ["S", "M", "L"]
FOCI = ["spec-compliance", "edge-cases", "security", "empirical", "design"]

# Real observations a reviewer can raise. The index is the finding's number.
FINDING_TEXT = [
    "At forty columns the round's notes wrap into the next column and the reader loses the verdict.",
    "The row cuts after the age and the finding count disappears from the row.",
    "The document paints a level-two heading as the first level, so the hierarchy is flat.",
    "A finding no round names is dropped from the Review section instead of shown after the rounds.",
    "The queue's count says one and a narrow pane draws none of the rows.",
    "The cut leaves a combining mark behind and the row is one column wider than the pane.",
    "The configuration's error is drawn on the dim slot, so it reads as a hint rather than a failure.",
    "The fenced block's gutter numbers the wrapped rows, so one line has three numbers.",
    "The mark is rendered but no slot paints it, so two equals signs sit in the sentence.",
    "The pool of external rounds is counted twice when the panel is redrawn.",
]

# What each review round was looking at, in the order the rounds run.
FOCUS_BY_ROUND = ["spec-compliance", "edge-cases", "security", "empirical", "design"]

# The fixed instant the fixtures are read with, as in
# crates/core/tests/mock_data.rs: it decides whether a claim's lease is
# still live.
FIXTURE_NOW = "2026-11-19T08:00:00Z"

DONE_IDS = [f"TSK{number:02d}" for number in range(22, 27)]


def brief(task_id):
    return BRIEFS[task_id]


def slugify(title):
    out = []
    dash = False
    for ch in title.lower():
        if ch.isalnum():
            if dash and out:
                out.append("-")
            dash = False
            out.append(ch)
        else:
            dash = True
    return "".join(out)


def pick(seq, index):
    return seq[index % len(seq)]


def ts(day, hour, minute=0):
    return f"2026-11-{day:02d}T{hour:02d}:{minute:02d}Z"


def plus_minutes(at, minutes):
    moment = datetime.strptime(at, "%Y-%m-%dT%H:%MZ") + timedelta(minutes=minutes)
    return moment.strftime("%Y-%m-%dT%H:%MZ")


def pass_evidence(command, by, session, day, hour, minute=0):
    return {
        "command": command,
        "exit_code": 0,
        "summary": "12 passed; 0 failed",
        "at": ts(day, hour, minute),
        "by": by,
        "session": session,
    }


def fail_evidence(command, by, session, day, hour, minute=0):
    return {
        "command": command,
        "exit_code": 1,
        "summary": "1 failed; 11 passed",
        "at": ts(day, hour, minute),
        "by": by,
        "session": session,
    }


def criterion(
    cid,
    description,
    verification,
    status="pending",
    evidence=None,
    deferral=None,
    failure_reason="",
):
    return {
        "id": cid,
        "description": description,
        "verification": verification,
        "status": status,
        "evidence": evidence or [],
        "deferral": deferral,
        "failure_reason": failure_reason,
    }


def step(
    order,
    action,
    done_when,
    files,
    covers,
    status="pending",
    depends_on=None,
    deviation="",
):
    return {
        "id": f"STP-{order:02d}",
        "order": order,
        "action": action,
        "done_when": done_when,
        "files": files,
        "status": status,
        "covers": covers,
        "depends_on": depends_on or [],
        "deviation": deviation,
    }


def comment(cid, author, body, at):
    return {"id": cid, "author": author, "body": body, "at": at}


def finding(
    number,
    created,
    severity="medium",
    status="open",
    description=None,
    resolved="",
    resolution="",
    comments=None,
):
    return {
        "id": f"FIND-{number:02d}",
        "severity": severity,
        "status": status,
        "description": description or pick(FINDING_TEXT, number),
        "created": created,
        "resolved": resolved,
        "resolution": resolution,
        "comments": comments or [],
    }


def round_(n, kind, focus, reviewer, verdict, at, notes, findings=None):
    return {
        "n": n,
        "kind": kind,
        "focus": focus,
        "reviewer": reviewer,
        "verdict": verdict,
        "at": at,
        "notes": notes,
        "findings": findings or [],
    }


def note(note_id, author, body, at):
    return {"id": note_id, "author": author, "body": body, "at": at}


def question(
    qid,
    text,
    status,
    answer,
    raised_by,
    raised_at,
    resolved_by,
    resolved_at,
):
    return {
        "id": qid,
        "question": text,
        "status": status,
        "answer": answer,
        "raised_by": raised_by,
        "raised_at": raised_at,
        "resolved_by": resolved_by,
        "resolved_at": resolved_at,
    }


def transition_note(index, author, body, at, note_id="NOTE-01"):
    """A note dated at the moment the task entered its status.

    The model has no separate event log, so a status transition records
    its moment in a note.
    """
    return note(note_id, author, body, at)


def transition_moment(item):
    """The moment the fixture entered the status it is in."""
    status = item["status"]
    if status == "draft":
        return item["created"]
    if status in ("groomed", "approved"):
        prefix = "Groomed by" if status == "groomed" else "Approved by"
        for candidate in reversed(item["notes"]):
            if candidate["body"].startswith(prefix):
                return candidate["at"]
        raise ValueError(f"{item['id']} has no {prefix} transition note")
    if status == "executing":
        return item["claim"]["at"]
    # Every other status is entered by its last review round.
    return item["review"]["rounds"][-1]["at"]


def event_stamps(item):
    """Every dated event in the fixture's history."""
    stamps = [item["created"], item["status_since"]]
    claim = item.get("claim")
    if claim:
        stamps.append(claim["at"])
    for overlay in item.get("overlays", {}).values():
        if overlay:
            stamps.append(overlay["at"])
    for criterion in item.get("acceptance_criteria", []):
        for evidence in criterion.get("evidence", []):
            stamps.append(evidence["at"])
    review = item.get("review", {})
    for finding in review.get("findings", []):
        stamps.append(finding["created"])
        if finding["resolved"]:
            stamps.append(finding["resolved"])
        for comment in finding.get("comments", []):
            stamps.append(comment["at"])
    for round in review.get("rounds", []):
        stamps.append(round["at"])
    for question in item.get("questions", []):
        stamps.append(question["raised_at"])
        if question["resolved_at"]:
            stamps.append(question["resolved_at"])
    for note in item.get("notes", []):
        stamps.append(note["at"])
    return stamps


def task(task_id, title, status, index, **overrides):
    # `created` and `updated` are the two ends of the fixture's timeline:
    # `updated` is moved to the latest event in `finalize` below.
    created = ts(2 + index % 6, 9 + index % 8)
    item = {
        "schema_version": 1,
        "id": task_id,
        "title": title,
        "slug": slugify(title),
        "parent": None,
        "children": [],
        "created": created,
        "updated": "",
        "kind": "task",
        "promoted_from": None,
        "status": status,
        "status_since": "",
        "overlays": {},
        "claim": None,
        "planned_by": None,
        "meta": {
            "priority": pick(PRIORITIES, index),
            "effort": pick(EFFORTS, index),
            "tags": ["mock", "raul"],
        },
        "intent": {"outcome": "", "problem": ""},
        "scope": {"in_scope": [], "out_of_scope": []},
        "acceptance_criteria": [],
        "steps": [],
        "review": {"max_rounds": 5, "rounds": [], "verdict": None, "findings": []},
        "questions": [],
        "notes": [],
        "depends_on": [],
    }
    item.update(overrides)
    return item


def planned(task_id, index, status, **overrides):
    """A task with its brief's intent, scope, criteria and planned steps."""
    b = brief(task_id)
    criteria = [
        criterion(f"ACCEPT-{n:02d}", description, verification)
        for n, (description, verification) in enumerate(b["criteria"], start=1)
    ]
    steps = [
        step(
            n,
            action,
            done_when,
            files,
            [f"ACCEPT-{n:02d}"],
            depends_on=[f"STP-{n - 1:02d}"] if n > 1 else [],
        )
        for n, (action, done_when, files) in enumerate(b["steps"], start=1)
    ]
    item = task(
        task_id,
        b["title"],
        status,
        index,
        intent={"outcome": b["outcome"], "problem": b["problem"]},
        scope={
            "in_scope": list(b["in_scope"]),
            "out_of_scope": list(b["out_of_scope"]),
        },
        acceptance_criteria=criteria,
        steps=steps,
    )
    if "kind" in b:
        item["kind"] = b["kind"]
    item.update(overrides)
    return item


def finalize(item):
    """Stamp the status from the history and move the last write with it."""
    item["status_since"] = transition_moment(item)
    # The last write is the latest event in the fixture's history.
    item["updated"] = max(event_stamps(item))
    assert item["created"] <= item["status_since"] <= item["updated"], item["id"]
    return item


def check_claims_follow_dependencies(tasks):
    """Every task past approved was claimed after its dependencies finished.

    `dependencies.satisfied` gates `start`: the task must be claimed and every
    dependency done. A fixture past approved therefore tells a chronology the
    model allows only if its claim sits after the completion of the done work
    it waits on.
    """
    by_id = {item["id"]: item for item in tasks}
    past_approved = {"executing", "code-review", "remediating", "needs-human", "done"}
    for item in tasks:
        if item["status"] not in past_approved:
            continue
        for dependency in item["depends_on"]:
            done = by_id[dependency]
            assert done["status"] == "done", (item["id"], dependency, done["status"])
            assert item["claim"]["at"] > done["status_since"], (
                item["id"],
                item["claim"]["at"],
                dependency,
                done["status_since"],
            )


# The journal's event kinds, in the order one task's history has them when
# they share an instant. The kinds are provisional: the writer settles them.
EVENT_KINDS = [
    "created",
    "approved",
    "claimed",
    "review_requested",
    "finding_raised",
    "remediated",
    "commented",
    "escalated",
    "completed",
]
EVENT_RANK = {kind: rank for rank, kind in enumerate(EVENT_KINDS)}


def stamp(text):
    return datetime.strptime(text, "%Y-%m-%dT%H:%MZ")


def at_minute(moment):
    return moment.replace(second=0, microsecond=0).strftime("%Y-%m-%dT%H:%MZ")


def minutes_before(text, minutes):
    return at_minute(stamp(text) - timedelta(minutes=minutes))


def between(first, second):
    """The whole minute midway between two events."""
    return at_minute(stamp(first) + (stamp(second) - stamp(first)) / 2)


def event(at, kind, task_id, by, note=""):
    record = {"at": at, "kind": kind, "task": task_id, "by": by}
    if note:
        record["note"] = note
    return record


def approval_at(item):
    """When the task left the human lane, from its own history."""
    for note in reversed(item["notes"]):
        if note["body"].startswith("Approved by"):
            return note["at"]
    if item["status"] == "approved":
        return item["status_since"]
    if item.get("claim"):
        # An older fixture records the claim but not the approval: the
        # claim is the next event, and the approval is shortly before it.
        return minutes_before(item["claim"]["at"], 30)
    return None


def activity_events(tasks):
    """The journal of the tasks' own histories, oldest first."""
    events = []
    for item in tasks:
        task_id = item["id"]
        events.append(event(item["created"], "created", task_id, "human"))
        approved = approval_at(item)
        if approved:
            events.append(
                event(approved, "approved", task_id, "human", "left the human lane")
            )
        claim = item.get("claim")
        if claim:
            events.append(
                event(claim["at"], "claimed", task_id, "runner", f"claimed by {claim['by']}")
            )
        rounds = item.get("review", {}).get("rounds", [])
        if rounds:
            events.append(
                event(rounds[0]["at"], "review_requested", task_id, "runner", "sent to review")
            )
        for finding in item.get("review", {}).get("findings", []):
            events.append(
                event(
                    finding["created"],
                    "finding_raised",
                    task_id,
                    "reviewer",
                    f"raised {finding['id']}",
                )
            )
        # A re-review is the runner answering a finding and sending the work
        # back: it sits between two external rounds.
        external = [round for round in rounds if round["kind"] == "external"]
        for first, second in zip(external, external[1:]):
            events.append(
                event(
                    between(first["at"], second["at"]),
                    "remediated",
                    task_id,
                    "runner",
                    "answered the findings",
                )
            )
        if item["status"] == "needs-human":
            events.append(
                event(
                    item["status_since"],
                    "escalated",
                    task_id,
                    "reviewer",
                    "the review budget is spent",
                )
            )
        if item["status"] == "done":
            events.append(
                event(item["status_since"], "completed", task_id, "reviewer", "reviewed clean")
            )
        for candidate in item.get("notes", []):
            if candidate["body"].startswith("Groomed by") or candidate["body"].startswith(
                "Approved by"
            ):
                continue
            # The journal's note is one line, so a multi-line note gives its
            # first line.
            body = candidate["body"].splitlines()[0] if candidate["body"] else ""
            author = candidate["author"]
            if author.startswith("runner"):
                by = "runner"
            elif author.startswith("reviewer"):
                by = "reviewer"
            else:
                by = "human"
            events.append(event(candidate["at"], "commented", task_id, by, body))
    events.sort(key=lambda record: (record["at"], EVENT_RANK[record["kind"]], record["task"]))
    return events


def check_activity(tasks, events):
    """The journal tells the chronology the fixtures already tell."""
    by_id = {item["id"]: item for item in tasks}
    for record in events:
        assert record["kind"] in EVENT_RANK, record
        assert record["task"] in by_id, record
        item = by_id[record["task"]]
        assert record["at"] >= item["created"], (record, item["created"])
    for task_id, item in by_id.items():
        task_events = [record for record in events if record["task"] == task_id]
        assert task_events, f"{task_id} has no activity"
        assert task_events[0]["kind"] == "created", (task_id, task_events[0])
        stamps = [record["at"] for record in task_events]
        assert stamps == sorted(stamps), (task_id, stamps)
        kinds = [record["kind"] for record in task_events]
        if "claimed" in kinds:
            assert "approved" in kinds, task_id
            assert kinds.index("approved") < kinds.index("claimed"), task_id
            if "review_requested" in kinds:
                assert kinds.index("claimed") < kinds.index("review_requested"), task_id
        if item["status"] == "done":
            assert "completed" in kinds, task_id
        if item["status"] == "needs-human":
            assert "escalated" in kinds, task_id
    # The dependency rule holds in time as it does in the fixtures: a claim
    # comes after the completion of every done task it waits on.
    for record in events:
        if record["kind"] != "claimed":
            continue
        item = by_id[record["task"]]
        for dependency in item["depends_on"]:
            completed = [
                done["at"]
                for done in events
                if done["task"] == dependency and done["kind"] == "completed"
            ]
            assert completed, (record["task"], dependency, "not completed")
            assert record["at"] > completed[0], (
                record["task"],
                record["at"],
                dependency,
                completed[0],
            )
    # The block shows the last few: the source has more than it shows.
    assert len(events) > 5, len(events)


def lease_is_live(claim):
    """Whether a claim still holds its task at the fixtures' clock."""
    if not claim:
        return False
    lease = claim.get("lease_expires_at") or ""
    if not lease:
        return True
    try:
        expires = datetime.fromisoformat(lease.replace("Z", "+00:00"))
    except ValueError:
        return False
    return expires > datetime.fromisoformat(FIXTURE_NOW.replace("Z", "+00:00"))


def claimable(item, by_id):
    """Whether an agent could claim the task at the fixtures' clock.

    The claim gate is the model's: approved, no overlay, every dependency
    done, and no live claim held elsewhere. It is the queue's one rule.
    """
    if item["status"] != "approved":
        return False
    if any(item["overlays"].get(kind) for kind in ("blocked", "deferred", "cancelled")):
        return False
    for dependency in item["depends_on"]:
        if by_id[dependency]["status"] != "done":
            return False
    return not lease_is_live(item.get("claim"))


# The run the runtime record tells: a queue of five tasks, in order. Two are
# done, one is in flight and two wait. The flow runs one task at a time, so
# the builder and the reviewer both hold the task in flight: the reviewer is
# on its second external round, and the builder waits for the verdict.
RUN_QUEUE = ["TSK22", "TSK25", "TSK14", "TSK29", "TSK07"]
IN_FLIGHT = "TSK14"
# Round 1 of the task in flight raised three findings, one high and two
# medium. Two are resolved, so the one high is still open: the task's own
# fixture holds that open finding, and the record's count is checked against it.
FIRST_ROUND_RAISED = 3
OPEN_SEVERITIES = {"high": 1, "medium": 0, "low": 0}


def runtime_record(tasks):
    """The flow's runtime, told as one run over the fixtures' own tasks.

    The queue is the run's tasks in order, each with the status its fixture
    holds. The crew holds the task in flight. The round is the reviewer's
    second external round, after one round that raised findings. The budget
    is the rounds the task in flight has used of its own limit, and the
    findings are its open ones.
    """
    by_id = {item["id"]: item for item in tasks}
    flight = by_id[IN_FLIGHT]
    return {
        "started_at": minutes_before(by_id[RUN_QUEUE[0]]["claim"]["at"], 30),
        "crew": [
            {
                "role": "orchestrator",
                "model": ORCHESTRATOR_MODEL,
                "state": "idle",
            },
            {
                "role": "builder",
                "model": RUNNER_MODEL,
                "holds": IN_FLIGHT,
                "since": flight["claim"]["at"],
                "state": "idle",
            },
            {
                "role": "reviewer",
                "model": REVIEWER_MODEL,
                "holds": IN_FLIGHT,
                "since": flight["status_since"],
                "state": "reviewing",
            },
        ],
        "queue": [
            {
                "id": task_id,
                "status": by_id[task_id]["status"],
                "claimable": claimable(by_id[task_id], by_id),
            }
            for task_id in RUN_QUEUE
        ],
        "round": {
            "task": IN_FLIGHT,
            "stage": "external",
            "number": 2,
            "earlier": [{"number": 1, "raised": FIRST_ROUND_RAISED}],
        },
        "budget": {
            "spent": 2,
            "total": flight["review"]["max_rounds"],
        },
        "findings": sum(OPEN_SEVERITIES.values()),
        "severities": dict(OPEN_SEVERITIES),
        "stamp": {"runs": True},
    }


def check_runtime(record, tasks):
    """The runtime record tells the run the fixtures can hold."""
    by_id = {item["id"]: item for item in tasks}

    # The queue is the run, in order, with the statuses the fixtures hold.
    queued = [item["id"] for item in record["queue"]]
    assert queued == RUN_QUEUE, queued
    for item in record["queue"]:
        assert item["status"] == by_id[item["id"]]["status"], item
        # The flag is the claim rule's answer, so a pick offers what an agent
        # could take right now and nothing else.
        assert item["claimable"] == claimable(by_id[item["id"]], by_id), item

    # Two done, one in flight and held by the crew, two approved and waiting.
    # One of the waiting is claimable; the other waits on work not yet done.
    done = [item["id"] for item in record["queue"] if item["status"] == "done"]
    assert len(done) == 2, done
    held = [member["holds"] for member in record["crew"] if member.get("holds")]
    assert held == [IN_FLIGHT, IN_FLIGHT], held
    waiting = [
        item["id"]
        for item in record["queue"]
        if item["id"] not in held and item["status"] != "done"
    ]
    assert len(waiting) == 2, waiting
    for task_id in waiting:
        item = by_id[task_id]
        assert item["status"] == "approved" and not item["overlays"], item["id"]
    claimable_now = [item["id"] for item in record["queue"] if item["claimable"]]
    assert claimable_now == ["TSK29"], claimable_now

    # The stamp runs, and a record that stops says why.
    if record["stamp"]["runs"]:
        assert "why" not in record["stamp"], record["stamp"]
    else:
        assert record["stamp"].get("why"), record["stamp"]

    # The round is the task in flight's: the next external round, after the
    # rounds it has had. The budget counts those rounds of its own limit.
    # TSK14 reaches round 2 through a re_review the fixture cannot stamp: the
    # model has no re-review record yet (talita's design adds one). So its
    # open high and the fix times are the runtime's story, not the fixture's.
    round_ = record["round"]
    assert round_["task"] == IN_FLIGHT, round_
    assert by_id[IN_FLIGHT]["status"] == "code-review", IN_FLIGHT
    assert round_["number"] == len(round_["earlier"]) + 1, round_
    assert record["budget"] == {
        "spent": round_["number"],
        "total": by_id[IN_FLIGHT]["review"]["max_rounds"],
    }, record["budget"]
    reviewer = next(m for m in record["crew"] if m["role"] == "reviewer")
    assert reviewer["state"] == "reviewing", reviewer

    # The findings are the open ones of the task in flight, by severity.
    assert record["findings"] == sum(record["severities"].values()), record
    assert set(record["severities"]) == {"high", "medium", "low"}, record
    assert record["severities"] == OPEN_SEVERITIES, record["severities"]
    # The first round's findings are the task's own: the record's count is
    # what that round raised, in the fixture's external round 1.
    first = next(r for r in by_id[IN_FLIGHT]["review"]["rounds"] if r["n"] == 1)
    assert first["kind"] == "external", first
    assert record["round"]["earlier"] == [{"number": 1, "raised": FIRST_ROUND_RAISED}], record["round"]
    assert len(first["findings"]) == FIRST_ROUND_RAISED, first
    open_here = [f for f in by_id[IN_FLIGHT]["review"]["findings"] if f["status"] == "open"]
    assert record["findings"] == len(open_here), (record["findings"], open_here)
    for severity in ("high", "medium", "low"):
        assert record["severities"][severity] == sum(
            1 for f in open_here if f["severity"] == severity
        ), severity

    # The crew is one per role, the builder and reviewer on the task in flight.
    roles = [member["role"] for member in record["crew"]]
    assert sorted(roles) == ["builder", "orchestrator", "reviewer"], roles
    builder = next(m for m in record["crew"] if m["role"] == "builder")
    assert builder["since"] == by_id[IN_FLIGHT]["claim"]["at"], builder

    # The run began before the first hold.
    started = stamp(record["started_at"])
    for member in record["crew"]:
        if member.get("since"):
            assert started < stamp(member["since"]), member


def build():
    tasks = []
    counter = 0

    def next_id():
        nonlocal counter
        counter += 1
        return f"TSK{counter:02d}"

    ids = {}

    # draft — a title alone. Two carry an overlay: one parked on purpose,
    # one cancelled, so every overlay slot has a fixture.
    for i in range(3):
        tid = next_id()
        ids.setdefault("draft", []).append(tid)
        item = task(tid, brief(tid)["title"], "draft", i)
        if tid == "TSK02":
            item["overlays"] = {
                "cancelled": {
                    "reason": "Parked until the row tabs share one announcement.",
                    "by": "human",
                    "at": ts(7, 11),
                }
            }
        if tid == "TSK03":
            item["overlays"] = {
                "deferred": {
                    "reason": "Waits on the orchestrator's model pick in hugo.",
                    "by": "human",
                    "at": ts(9, 10),
                }
            }
        tasks.append(item)

    # groomed — intent, scope, criteria; one of them blocked, one carries an
    # open question, one a resolved question.
    for i in range(3):
        tid = next_id()
        ids.setdefault("groomed", []).append(tid)
        groom_at = ts(6 + 2 * i, 9 + i)
        creator = f"session-creator-{i + 1:02d}"
        b = brief(tid)
        notes = [
            transition_note(
                i,
                creator,
                f"Groomed by {creator}: intent, scope and criteria written.",
                groom_at,
            )
        ]
        if tid == "TSK05":
            notes.append(
                note(
                    "NOTE-02",
                    creator,
                    "The blocked reason names the design question the human has to settle first.",
                    ts(8, 14),
                )
            )
        questions = []
        if tid == "TSK04":
            questions.append(
                question(
                    "QST-01",
                    "Does the review state fit beside the age at forty columns?",
                    "open",
                    "",
                    "human",
                    ts(6, 10),
                    "",
                    "",
                )
            )
        if tid == "TSK06":
            questions.append(
                question(
                    "QST-01",
                    "How much of the body does the row carry?",
                    "resolved",
                    "The first words that fit; the cut marker names the rest.",
                    "human",
                    ts(10, 12),
                    "session-creator-03",
                    ts(10, 13),
                )
            )
        item = planned(
            tid,
            i + 3,
            "groomed",
            planned_by=creator,
            notes=notes,
            questions=questions,
        )
        # The brief supplies the criteria; the steps are planned once the
        # task is approved.
        item["steps"] = []
        if tid == "TSK05":
            item["overlays"] = {
                "blocked": {
                    "reason": "The engine's viewport ownership is still a design question.",
                    "by": "human",
                    "at": ts(8, 14),
                }
            }
        tasks.append(item)

    # approved — criteria covered by steps with done_when.
    for i in range(3):
        tid = next_id()
        ids.setdefault("approved", []).append(tid)
        creator = f"session-creator-{i + 1:02d}"
        questions = []
        if tid == "TSK07":
            questions.append(
                question(
                    "QST-01",
                    "Does a round with no findings get an empty line or no line?",
                    "resolved",
                    "An empty line, so the reader knows the round looked.",
                    "human",
                    ts(4, 10),
                    creator,
                    ts(5, 15),
                )
            )
        item = planned(
            tid,
            i + 6,
            "approved",
            planned_by=creator,
            notes=[
                transition_note(
                    i,
                    creator,
                    f"Groomed by {creator}: intent, scope and criteria written.",
                    ts(4 + i, 9),
                ),
                transition_note(
                    i + 3,
                    creator,
                    f"Approved by {creator}: criteria covered, steps planned.",
                    ts(5 + i, 16 + i),
                    note_id="NOTE-02",
                ),
            ],
            questions=questions,
            depends_on=(
                [ids["groomed"][0], ids["groomed"][1]]
                if i == 0
                else [DONE_IDS[i - 1]]
            ),
        )
        if tid == "TSK08":
            item["overlays"] = {
                "blocked": {
                    "reason": "The palette table waits for the human's pick of eighteen roles.",
                    "by": "creator-02",
                    "at": ts(13, 15),
                }
            }
        if tid == "TSK09":
            item["claim"] = {
                "by": "runner-09",
                "session": "session-runner-09",
                "at": ts(14, 9),
                "lease_expires_at": "2099-12-31T00:00:00Z",
            }
        tasks.append(item)

    # executing — claimed, evidence on the first criterion; one defers a
    # criterion, one leaves one unproven, one asks a question.
    for i in range(3):
        tid = next_id()
        ids.setdefault("executing", []).append(tid)
        claim_day = 9 + i
        runner = f"runner-{i + 1:02d}"
        session = f"session-runner-{i + 1:02d}"
        b = brief(tid)
        first_verification = b["criteria"][0][1]
        criteria = [
            criterion(
                "ACCEPT-01",
                b["criteria"][0][0],
                first_verification,
                status="passed",
                evidence=[
                    pass_evidence(first_verification, runner, session, claim_day, 12)
                ],
            )
        ]
        if tid == "TSK11":
            criteria.append(
                criterion(
                    "ACCEPT-02",
                    b["criteria"][1][0],
                    b["criteria"][1][1],
                    status="unproven",
                )
            )
        elif tid == "TSK12":
            criteria.append(
                criterion(
                    "ACCEPT-02",
                    b["criteria"][1][0],
                    b["criteria"][1][1],
                    status="deferred",
                    deferral={
                        "reason": "The queue's count reads the same window the rows do; that belongs to the row tabs.",
                        "follow_up": ids["draft"][0],
                    },
                )
            )
        else:
            criteria.append(
                criterion(
                    "ACCEPT-02",
                    b["criteria"][1][0],
                    b["criteria"][1][1],
                )
            )
        steps = [
            step(
                1,
                b["steps"][0][0],
                b["steps"][0][1],
                b["steps"][0][2],
                ["ACCEPT-01"],
                status="in-progress" if tid == "TSK10" else "done",
                deviation=(
                    "The count moved into the existing panel rather than a new one."
                    if tid == "TSK10"
                    else ""
                ),
            ),
            step(
                2,
                b["steps"][1][0],
                b["steps"][1][1],
                b["steps"][1][2],
                ["ACCEPT-02"],
                depends_on=["STP-01"],
            ),
        ]
        questions = []
        if tid == "TSK11":
            questions.append(
                question(
                    "QST-01",
                    "Should a dropped branch be announced above the trunk or at the foot?",
                    "open",
                    "",
                    runner,
                    ts(claim_day, 13, 30),
                    "",
                    "",
                )
            )
        item = task(
            tid,
            b["title"],
            "executing",
            i + 9,
            kind=b.get("kind", "task"),
            planned_by=f"session-creator-{i + 1:02d}",
            claim={
                "by": runner,
                "session": session,
                "at": ts(claim_day, 9),
                "lease_expires_at": ts(claim_day, 17),
            },
            intent={"outcome": b["outcome"], "problem": b["problem"]},
            scope={
                "in_scope": list(b["in_scope"]),
                "out_of_scope": list(b["out_of_scope"]),
            },
            acceptance_criteria=criteria,
            steps=steps,
            notes=[
                note(
                    "NOTE-01",
                    runner,
                    "The first criterion is proven. The second is waiting on the fixture the next unit writes.",
                    ts(claim_day, 13),
                )
            ],
            questions=questions,
            depends_on=[DONE_IDS[i]],
        )
        tasks.append(item)

    # code-review — steps settled, evidence, self round, findings: one open,
    # one resolved with a comment, and one finding no round names.
    for i in range(3):
        tid = next_id()
        ids.setdefault("code-review", []).append(tid)
        review_day = 10 + i
        claim_at = ts(review_day, 9)
        round_at = ts(review_day, 16)
        # TSK14's findings were raised by its first external round, after the
        # self pre-screen had found nothing; every other task raised its own at
        # the self round.
        raised_at = ts(review_day, 16, 40) if tid == "TSK14" else round_at
        runner = f"runner-{i + 1:02d}"
        session = f"session-runner-{i + 1:02d}"
        b = brief(tid)
        findings = [
            finding(
                1,
                raised_at,
                severity="high" if tid == "TSK14" else "medium",
                description=(
                    FINDING_TEXT[1] if tid == "TSK14" else pick(FINDING_TEXT, i)
                ),
                comments=(
                    [
                        comment(
                            "CMT-01",
                            f"reviewer-{i + 1:02d}",
                            "The row keeps the count when the cut moves it.",
                            ts(review_day, 17),
                        )
                    ]
                    if tid == "TSK14"
                    else []
                ),
            )
        ]
        if tid == "TSK13":
            findings.append(
                finding(
                    2,
                    round_at,
                    severity="low",
                    status="fixed",
                    description="The mark's background says nothing about the selection.",
                    resolved=ts(review_day, 16, 30),
                    resolution="The mark now reads the selection's two slots, foreground and background.",
                    comments=[
                        comment(
                            "CMT-01",
                            "runner-01",
                            "Took the slots from the pane rather than inventing a third colour.",
                            ts(review_day, 16, 10),
                        )
                    ],
                )
            )
        elif tid == "TSK15":
            findings.append(
                finding(
                    2,
                    round_at,
                    severity="high",
                    description=pick(FINDING_TEXT, 7),
                )
            )
        elif tid == "TSK14":
            findings.extend(
                [
                    finding(
                        2,
                        raised_at,
                        severity="medium",
                        status="fixed",
                        description=FINDING_TEXT[5],
                        resolved=ts(review_day, 17, 20),
                        resolution="The cut goes through the shared helper, so the row keeps whole graphemes.",
                    ),
                    finding(
                        3,
                        raised_at,
                        severity="medium",
                        status="fixed",
                        description="The pin passes on a title with no combining mark, so it does not catch a scalar cut.",
                        resolved=ts(review_day, 17, 25),
                        resolution="The pin now builds its title from combining marks.",
                    ),
                ]
            )
        item = planned(
            tid,
            i + 12,
            "code-review",
            planned_by=f"session-creator-{i + 1:02d}",
            claim={
                "by": runner,
                "session": session,
                "at": claim_at,
                "lease_expires_at": ts(review_day, 18),
            },
            notes=[
                note(
                    "NOTE-01",
                    runner,
                    "The first criterion is proven and the second follows the same command.",
                    ts(review_day + 1, 9),
                )
            ],
            depends_on=[DONE_IDS[i], DONE_IDS[(i + 1) % len(DONE_IDS)]],
        )
        # Both criteria carry evidence: a code-review task has finished
        # executing, so every criterion is proven or failed.
        for number, acceptance in enumerate(item["acceptance_criteria"], start=1):
            acceptance["status"] = "passed"
            acceptance["evidence"] = [
                pass_evidence(
                    acceptance["verification"],
                    runner,
                    session,
                    review_day,
                    14,
                    (number - 1) * 5,
                )
            ]
        for planned_step in item["steps"]:
            planned_step["status"] = "done"
        if tid == "TSK13":
            # The detail pane's fixture. Its note carries a ==mark==, so the
            # document paints text.selected on free text; it carries a fenced
            # code block, so the document paints syntect's code tokens; and
            # its criterion's verification carries a shell pipe, so the
            # criteria table holds a code span with an escaped pipe.
            item["notes"][0]["body"] += (
                "\n\nThe ==key finding== is marked for the next round.\n\n"
                "```rust\n"
                "let answer = 42;\n"
                "```"
            )
        item["review"] = {
            "max_rounds": 5,
            "rounds": [
                round_(
                    0,
                    "self",
                    "spec-compliance",
                    runner,
                    "changes-needed",
                    round_at,
                    "The pre-screen found the row and the mark before the external round spent budget.",
                    [finding["id"] for finding in findings if finding["created"] == round_at],
                )
            ],
            "verdict": None,
            "findings": findings,
        }
        if tid == "TSK14":
            # The first external round raised the three findings and sent the
            # work back, and the self round before it found nothing. The high
            # finding is still open, so the review is round 2's, in flight.
            self_round = item["review"]["rounds"][0]
            self_round["verdict"] = "ok"
            self_round["notes"] = "The pre-screen found nothing to fix before the external round."
            self_round["findings"] = []
            item["review"]["rounds"].append(
                round_(
                    1,
                    "external",
                    pick(FOCI, i + 1),
                    f"reviewer-{i + 1:02d}",
                    "changes-needed",
                    raised_at,
                    "The count is lost when the cut moves, and the pin misses the cut.",
                    [finding["id"] for finding in findings],
                )
            )
        tasks.append(item)

    # remediating — external round with changes-needed: one finding fixed,
    # one open, and on the third fixture one dismissed.
    for i in range(3):
        tid = next_id()
        ids.setdefault("remediating", []).append(tid)
        claim_day = 13 + i // 2
        claim_hour = 9 + i % 2
        external_at = ts(claim_day, claim_hour + 8)
        runner = f"runner-{i + 2:02d}"
        session = f"session-runner-{i + 2:02d}"
        b = brief(tid)
        findings = [
            finding(
                1,
                external_at,
                severity="medium",
                status="fixed" if tid == "TSK16" else "open",
                description=pick(FINDING_TEXT, i + 1),
                resolved=ts(claim_day, claim_hour + 9) if tid == "TSK16" else "",
                resolution=(
                    "The round now draws as blocks; the table is gone."
                    if tid == "TSK16"
                    else ""
                ),
                comments=(
                    [
                        comment(
                            "CMT-01",
                            runner,
                            "Fixed by moving the round out of the table.",
                            ts(claim_day, claim_hour + 8, 30),
                        )
                    ]
                    if tid == "TSK16"
                    else []
                ),
            )
        ]
        second = finding(
            2,
            external_at,
            severity="high",
            description=pick(FINDING_TEXT, i + 4),
        )
        if tid == "TSK18":
            second["status"] = "dismissed"
            second["resolved"] = ts(claim_day, claim_hour + 9)
            second["resolution"] = (
                "The file is already left byte-identical; the notice only needed a name."
            )
            second["comments"] = [
                comment(
                    "CMT-01",
                    runner,
                    "The bytes are untouched; only the wording changed.",
                    ts(claim_day, claim_hour + 8, 45),
                )
            ]
        findings.append(second)
        item = planned(
            tid,
            i + 15,
            "remediating",
            planned_by=f"session-creator-{i + 2:02d}",
            claim={
                "by": runner,
                "session": session,
                "at": ts(claim_day, claim_hour),
                "lease_expires_at": ts(claim_day, claim_hour + 8),
            },
            depends_on=[DONE_IDS[(i + 1) % len(DONE_IDS)]],
        )
        for number, acceptance in enumerate(item["acceptance_criteria"], start=1):
            acceptance["status"] = "passed"
            acceptance["evidence"] = [
                pass_evidence(
                    acceptance["verification"],
                    runner,
                    session,
                    claim_day,
                    claim_hour + 4,
                    (number - 1) * 5,
                )
            ]
        for planned_step in item["steps"]:
            planned_step["status"] = "done"
        item["review"] = {
            "max_rounds": 5,
            "rounds": [
                round_(
                    0,
                    "self",
                    "spec-compliance",
                    runner,
                    "ok",
                    ts(claim_day, claim_hour + 6),
                    "The pre-screen found nothing to fix before the external round.",
                ),
                round_(
                    1,
                    "external",
                    pick(FOCI, i + 1),
                    f"reviewer-{i + 1:02d}",
                    "changes-needed",
                    external_at,
                    "The round is readable, but the second finding needs a decision.",
                    ["FIND-01", "FIND-02"],
                ),
            ],
            "verdict": "changes-needed",
            "findings": findings,
        }
        tasks.append(item)

    # needs-human — five external rounds exhausted. The findings carry the
    # history of the loop: fixed, dismissed and still open.
    for i in range(3):
        tid = next_id()
        ids.setdefault("needs-human", []).append(tid)
        claim_day = 14 + i
        runner = f"runner-{i + 3:02d}"
        session = f"session-runner-{i + 3:02d}"
        b = brief(tid)
        rounds = [
            round_(
                0,
                "self",
                "spec-compliance",
                runner,
                "ok",
                ts(claim_day, 5),
                "The pre-screen is clean; the work is ready for an external round.",
            )
        ]
        findings = []
        for n in range(1, 6):
            at = ts(claim_day, 5 + n)
            status = "open"
            resolved = ""
            resolution = ""
            comments = []
            if tid == "TSK19" and n in (1, 3):
                status = "fixed"
                resolved = ts(claim_day, 5 + n, 30)
                resolution = "The heading now takes its own level's role."
                comments = [
                    comment(
                        f"CMT-{n:02d}",
                        runner,
                        "The role follows the level rather than the document.",
                        ts(claim_day, 5 + n, 15),
                    )
                ]
            if tid == "TSK19" and n == 2:
                status = "dismissed"
                resolved = ts(claim_day, 7, 30)
                resolution = "The alert role is already distinct; the report named the body colour."
                comments = [
                    comment(
                        "CMT-02",
                        runner,
                        "Checked against the palette table; the alert has its own role.",
                        ts(claim_day, 7, 15),
                    )
                ]
            if tid == "TSK20" and n in (1, 2, 3):
                status = "fixed"
                resolved = ts(claim_day, 5 + n, 30)
                resolution = "The finding is nested under the round that raised it."
            if tid == "TSK21" and n in (1,):
                comments = [
                    comment(
                        "CMT-01",
                        f"reviewer-{n:02d}",
                        "The gutter is drawn but the wrap still takes a number.",
                        ts(claim_day, 5 + n, 20),
                    )
                ]
            findings.append(
                finding(
                    n,
                    at,
                    severity=["medium", "high", "low", "medium", "high"][n - 1],
                    status=status,
                    description=pick(FINDING_TEXT, i + n),
                    resolved=resolved,
                    resolution=resolution,
                    comments=comments,
                )
            )
            rounds.append(
                round_(
                    n,
                    "external",
                    pick(FOCUS_BY_ROUND, n),
                    f"reviewer-{n:02d}",
                    "changes-needed",
                    at,
                    "The round found the next defect and the budget moved one round closer to spent.",
                    [f"FIND-{n:02d}"],
                )
            )
        criteria = []
        for number, (description, verification) in enumerate(b["criteria"], start=1):
            if tid == "TSK21" and number == 2:
                criteria.append(
                    criterion(
                        "ACCEPT-02",
                        description,
                        verification,
                        status="failed",
                        evidence=[
                            fail_evidence(
                                verification, runner, session, claim_day, 3, 30
                            )
                        ],
                        failure_reason="A wrapped line still takes a second number in the gutter.",
                    )
                )
            else:
                criteria.append(
                    criterion(
                        f"ACCEPT-{number:02d}",
                        description,
                        verification,
                        status="passed",
                        evidence=[
                            pass_evidence(
                                verification,
                                runner,
                                session,
                                claim_day,
                                3,
                                (number - 1) * 10,
                            )
                        ],
                    )
                )
        steps = [
            step(
                n,
                action,
                done_when,
                files,
                [f"ACCEPT-{n:02d}"],
                status="failed" if tid == "TSK21" and n == 2 else "done",
                depends_on=[f"STP-{n - 1:02d}"] if n > 1 else [],
                deviation=(
                    "The wrap number is still drawn; the fix belongs to the gutter unit."
                    if tid == "TSK21" and n == 2
                    else ""
                ),
            )
            for n, (action, done_when, files) in enumerate(b["steps"], start=1)
        ]
        questions = []
        if tid == "TSK20":
            questions.append(
                question(
                    "QST-01",
                    "Does an orphan finding go before or after the unclaimed findings?",
                    "resolved",
                    "After the rounds, in its own block.",
                    "human",
                    ts(claim_day - 1, 12),
                    "session-creator-04",
                    ts(claim_day - 1, 13),
                )
            )
        item = task(
            tid,
            b["title"],
            "needs-human",
            i + 18,
            planned_by=f"session-creator-{i + 3:02d}",
            claim={
                "by": runner,
                "session": session,
                "at": ts(claim_day, 1),
                "lease_expires_at": ts(claim_day, 11),
            },
            intent={"outcome": b["outcome"], "problem": b["problem"]},
            scope={
                "in_scope": list(b["in_scope"]),
                "out_of_scope": list(b["out_of_scope"]),
            },
            acceptance_criteria=criteria,
            steps=steps,
            review={
                "max_rounds": 5,
                "rounds": rounds,
                "verdict": "changes-needed",
                "findings": findings,
            },
            notes=[
                note(
                    "NOTE-01",
                    f"reviewer-05",
                    "The budget is spent and the loop stops here: a human decides the next move.",
                    ts(claim_day, 12),
                )
            ],
            questions=questions,
            depends_on=[DONE_IDS[i], DONE_IDS[(i + 2) % len(DONE_IDS)]],
        )
        # The notes that carry the document's constructs: alerts, a nested
        # list three deep, a table, a mermaid block, a long link, inline code,
        # and fenced code in python and toml. Each appends to the note the
        # journal already reads, so its first line is unchanged.
        if tid == "TSK19":
            item["notes"][0]["body"] += (
                "\n\nThe roles the level work adds, by the field that paints them:\n\n"
                "| field | role | paints |\n"
                "|---|---|---|\n"
                "| `heading_1` | `HeadingSection` | the first-level heading |\n"
                "| `heading_2` | `HeadingItem` | the second-level heading |\n"
                "| `heading_3` | `HeadingFinding` | the third level and every level below |\n"
                "| `alert_note` | `AlertNote` | a `[!NOTE]` alert |\n"
                "| `alert_warning` | `AlertWarning` | a `[!WARNING]` alert |\n\n"
                "The open choice is the depth. The options are:\n\n"
                "- keep one role below level three\n"
                "  - the palette stays at eighteen roles\n"
                "  - a level-four heading reads like a level-three one\n"
                "    - the nested list in a finding is the only place it shows\n"
                "- give level four its own role\n"
                "  - the palette grows to nineteen\n\n"
                "> [!NOTE]\n"
                "> The alert and link roles were already in the palette; this unit adds only the headings.\n\n"
                "The upstream presets are in [leaf's `presets.rs`]"
                "(https://github.com/rivolink/leaf/blob/29f6ad480d95a4240c63f63d390e9909afdad9e8/src/theme/presets.rs)"
                " at the pinned commit.\n\n"
                "> [!WARNING]\n"
                "> The terminal preset stays colourless, so the roles above are invisible there:"
                " check the level split in a coloured preset before the level-four decision.\n"
            )
        if tid == "TSK20":
            item["notes"][0]["body"] += (
                "\n\nA finding keeps its id, such as `FIND-01`, across the rounds that name it."
                " It travels the loop the Review section draws:\n\n"
                "```mermaid\n"
                "flowchart TD\n"
                "    A[Round raises a finding] --> B{Fixed in the next round?}\n"
                "    B -->|yes| C[Resolved, listed under its round]\n"
                "    B -->|no| D[Carried into the next round]\n"
                "    D --> A\n"
                "```\n"
            )
        if tid == "TSK21":
            item["notes"][0]["body"] += (
                "\n\n> [!WARNING]\n"
                "> The gutter counts a wrapped row as a line, so a block's numbers stop matching"
                " the source past the first wrap.\n\n"
                "The width the gutter takes is the digits of the last line, plus two cells of padding:\n\n"
                "```python\n"
                "def gutter_width(lines: int) -> int:\n"
                "    return len(str(lines)) + 2\n"
                "```\n\n"
                "The engine's syntax set is a workspace dependency, so the gutter inherits it:\n\n"
                "```toml\n"
                "[dependencies]\n"
                "syntect.workspace = true\n"
                "```\n"
            )
        tasks.append(item)

    # done — clean verdict, no open findings. Five of them, chained only
    # among themselves: the done set is closed under its waits-on edges.
    done_dependencies = [
        [],
        [DONE_IDS[0]],
        [DONE_IDS[0], DONE_IDS[1]],
        [DONE_IDS[2]],
        [DONE_IDS[1], DONE_IDS[2], DONE_IDS[3]],
    ]
    done_schedule = [
        (ts(5, 15), (5, 16, 0), ts(5, 17), ts(5, 18)),
        (ts(6, 16), (6, 17, 0), ts(6, 18), ts(6, 19)),
        (ts(7, 17), (7, 17, 30), ts(7, 17, 45), ts(7, 18, 45)),
        (ts(7, 20, 30), (7, 20, 40), ts(7, 20, 50), ts(7, 21)),
        (ts(8, 9), (8, 10, 0), ts(8, 11), ts(8, 12)),
    ]
    for i in range(5):
        tid = next_id()
        assert tid == DONE_IDS[i], (tid, DONE_IDS[i])
        ids.setdefault("done", []).append(tid)
        claim_at, (evidence_day, evidence_hour, evidence_minute), self_at, external_at = done_schedule[i]
        runner = f"runner-{i + 4:02d}"
        session = f"session-runner-{i + 4:02d}"
        b = brief(tid)
        findings = []
        rounds = [
            round_(
                0,
                "self",
                "spec-compliance",
                runner,
                "ok",
                self_at,
                "The pre-screen is clean; the work leaves for the external round.",
            )
        ]
        if tid == "TSK23":
            findings.append(
                finding(
                    1,
                    ts(6, 19),
                    severity="medium",
                    status="fixed",
                    description="A fresh wait reads as zero minutes instead of words.",
                    resolved=ts(6, 19, 30),
                    resolution="The shared age vocabulary says under a minute.",
                    comments=[
                        comment(
                            "CMT-01",
                            runner,
                            "Both readers now share the words.",
                            ts(6, 19, 15),
                        )
                    ],
                )
            )
            rounds.append(
                round_(
                    1,
                    "external",
                    pick(FOCUS_BY_ROUND, 1),
                    f"reviewer-{i + 5:02d}",
                    "changes-needed",
                    ts(6, 19),
                    "The under-a-minute case was not exercised.",
                    ["FIND-01"],
                )
            )
            rounds.append(
                round_(
                    2,
                    "external",
                    pick(FOCUS_BY_ROUND, 2),
                    f"reviewer-{i + 5:02d}",
                    "ok",
                    ts(6, 20),
                    "The words and the row agree.",
                )
            )
        elif tid == "TSK24":
            for n, minute in enumerate((0, 15, 30), start=1):
                findings.append(
                    finding(
                        n,
                        ts(7, 18, minute),
                        severity=["medium", "high", "low"][n - 1],
                        status="fixed",
                        description=pick(FINDING_TEXT, n + 2),
                        resolved=ts(7, 18, minute + 5),
                        resolution="The fork parent is found from the first open dependency.",
                    )
                )
                rounds.append(
                    round_(
                        n,
                        "external",
                        pick(FOCUS_BY_ROUND, n),
                        f"reviewer-{i + 5:02d}",
                        "changes-needed",
                        ts(7, 18, minute),
                        "The fork parent moved when a dependency was added.",
                        [f"FIND-{n:02d}"],
                    )
                )
            rounds.append(
                round_(
                    4,
                    "external",
                    pick(FOCUS_BY_ROUND, 4),
                    f"reviewer-{i + 5:02d}",
                    "ok",
                    external_at,
                    "The fork is stable and the row names both dependencies.",
                )
            )
        elif tid == "TSK26":
            findings.append(
                finding(
                    1,
                    ts(8, 9, 30),
                    severity="low",
                    status="dismissed",
                    description="The broken file's name is cut at a narrow pane.",
                    resolved=ts(8, 9, 45),
                    resolution="The name is the row's own; the cut is the pane's, and the marker says so.",
                    comments=[
                        comment(
                            "CMT-01",
                            f"reviewer-{i + 5:02d}",
                            "A cut name is still a name; the marker carries the rest.",
                            ts(8, 9, 40),
                        )
                    ],
                )
            )
            rounds.append(
                round_(
                    1,
                    "external",
                    pick(FOCUS_BY_ROUND, 1),
                    f"reviewer-{i + 5:02d}",
                    "ok",
                    external_at,
                    "The folder is never hidden and the error is named.",
                )
            )
        else:
            rounds.append(
                round_(
                    1,
                    "external",
                    pick(FOCUS_BY_ROUND, i),
                    f"reviewer-{i + 5:02d}",
                    "ok",
                    external_at,
                    "The work is reviewed clean and the criteria are proven.",
                )
            )
        questions = []
        if tid in ("TSK22", "TSK25"):
            questions.append(
                question(
                    "QST-01",
                    "Does the row carry the count before or after the label?",
                    "resolved",
                    "After the label, so the count reads as the label's.",
                    "human",
                    ts(4, 10 + i),
                    f"session-creator-{i + 4:02d}",
                    ts(5, 14),
                )
            )
        notes = [
            note(
                "NOTE-01",
                runner,
                "The work is done and the evidence is attached; the review is closed.",
                ts(evidence_day, evidence_hour, evidence_minute + 5),
            )
        ]
        # The last note sits after the last round: a done task is written to
        # when the review closes, so it is stamped from the round that closed
        # it, not from the schedule's nominal external round.
        notes.append(
            note(
                "NOTE-02",
                f"reviewer-{i + 5:02d}",
                "Reviewed clean: the criteria are proven and no finding is open.",
                plus_minutes(rounds[-1]["at"], 10),
            )
        )
        # The fixture whose note carries a fenced bash block: the gate that
        # keeps the column table honest is the command the note names.
        if tid == "TSK22":
            notes[0]["body"] += (
                "\n\nThe gate regenerates the column table and fails on any drift:\n\n"
                "```bash\n"
                "make columns\n"
                "```\n\n"
                "The target runs the generator with `--check`, so a stale table fails"
                " `make check` rather than drifting quietly.\n"
            )
        item = task(
            tid,
            b["title"],
            "done",
            i + 21,
            planned_by=f"session-creator-{i + 4:02d}",
            claim={
                "by": runner,
                "session": session,
                "at": claim_at,
                "lease_expires_at": external_at,
            },
            intent={"outcome": b["outcome"], "problem": b["problem"]},
            scope={
                "in_scope": list(b["in_scope"]),
                "out_of_scope": list(b["out_of_scope"]),
            },
            acceptance_criteria=[
                criterion(
                    f"ACCEPT-{number:02d}",
                    description,
                    verification,
                    status="passed",
                    evidence=[
                        pass_evidence(
                            verification,
                            runner,
                            session,
                            evidence_day,
                            evidence_hour,
                            evidence_minute + (number - 1) * 5,
                        )
                    ],
                )
                for number, (description, verification) in enumerate(
                    b["criteria"], start=1
                )
            ],
            steps=[
                step(
                    n,
                    action,
                    done_when,
                    files,
                    [f"ACCEPT-{n:02d}"],
                    status=(
                        "skipped"
                        if tid == "TSK24" and n == 2
                        else "done"
                    ),
                    depends_on=[f"STP-{n - 1:02d}"] if n > 1 else [],
                    deviation=(
                        "The dependency fixture already covered this case; the step was not run."
                        if tid == "TSK24" and n == 2
                        else ""
                    ),
                )
                for n, (action, done_when, files) in enumerate(b["steps"], start=1)
            ],
            review={
                "max_rounds": 5,
                "rounds": rounds,
                "verdict": "ok",
                "findings": findings,
            },
            questions=questions,
            notes=notes,
            depends_on=done_dependencies[i],
        )
        tasks.append(item)

    done_ids = ids["done"]

    # smaller tasks — ideas, bugs and small features, each with a body and
    # a scope that say what the item is worth.
    def item(item_id, title, kind, status, body, scope, promoted_to=None,
             dismissed_reason=""):
        return {
            "schema_version": 1,
            "id": item_id,
            "title": title,
            "slug": slugify(title),
            "kind": kind,
            "status": status,
            "body": body,
            "scope": scope,
            "tags": ["mock"],
            "created": ts(2, 8),
            "updated": ts(6, 15),
            "promoted_to": promoted_to,
            "dismissed_reason": dismissed_reason,
        }

    items = [
        item(
            "ITM01",
            "The Overview says how long the run has been going",
            "idea",
            "open",
            "The activity block says what happened; it does not say when the run began.",
            ["the Overview's activity block"],
        ),
        item(
            "ITM02",
            "The Path row names the round it waits on",
            "feature",
            "open",
            "A task in review waits on a round number the Path row does not show.",
            ["the Path rows", "the review state"],
        ),
        item(
            "ITM03",
            "hugo says its configuration home could not be made",
            "bug",
            "open",
            "A home that cannot be created blanks the sidebar instead of naming the path.",
            ["hugo's sidebar", "the configuration home"],
        ),
        item(
            "ITM04",
            "The light theme families",
            "feature",
            "dismissed",
            "Catppuccin Latte, Nord Light and Dracula Alucard would finish the families.",
            ["Catppuccin Latte", "Nord Light", "Dracula Alucard"],
            dismissed_reason="Deferred by the human: the light themes wait until the dark presets are settled.",
        ),
    ]

    # A bug that was promoted: the task it became is a small task.
    promoted_title = brief("TSK27")["title"]
    promoted_id = next_id()
    assert promoted_id == "TSK27", promoted_id
    promoted_task = task(promoted_id, promoted_title, "draft", 4)
    promoted_task["kind"] = "bug"
    promoted_task["promoted_from"] = "ITM05"
    promoted_task["intent"] = {
        "outcome": brief("TSK27")["outcome"],
        "problem": brief("TSK27")["problem"],
    }
    promoted_task["scope"] = {
        "in_scope": list(brief("TSK27")["in_scope"]),
        "out_of_scope": list(brief("TSK27")["out_of_scope"]),
    }
    promoted_task["acceptance_criteria"] = [
        criterion("ACCEPT-01", brief("TSK27")["criteria"][0][0], "")
    ]
    tasks.append(promoted_task)
    items.append(
        item(
            "ITM05",
            promoted_title,
            "bug",
            "promoted",
            "The candidate row overflows when the title is wider than the row.",
            ["the candidate row"],
            promoted_to=promoted_id,
        )
    )

    # parent / child: the first approved task owns two draft children. The
    # first refines the round's shape, the second adds a finding no round
    # names.
    child_one = next_id()
    assert child_one == "TSK28", child_one
    ids.setdefault("draft", []).append(child_one)
    child_task = task(child_one, brief(child_one)["title"], "draft", 2)
    child_task["parent"] = ids["approved"][0]
    tasks.append(child_task)

    # approved after the last cycle finished: its dependencies are done, so
    # an agent can claim it right now. It is generated before the second
    # child, so the claimable fixture keeps its id.
    tid = next_id()
    assert tid == "TSK29", tid
    ids.setdefault("approved", []).append(tid)
    b = brief(tid)
    tasks.append(
        task(
            tid,
            b["title"],
            "approved",
            6,
            planned_by="session-creator-05",
            intent={"outcome": b["outcome"], "problem": b["problem"]},
            scope={
                "in_scope": list(b["in_scope"]),
                "out_of_scope": list(b["out_of_scope"]),
            },
            acceptance_criteria=[
                criterion(f"ACCEPT-{n:02d}", description, verification)
                for n, (description, verification) in enumerate(b["criteria"], start=1)
            ],
            steps=[
                step(
                    n,
                    action,
                    done_when,
                    files,
                    [f"ACCEPT-{n:02d}"],
                    depends_on=[f"STP-{n - 1:02d}"] if n > 1 else [],
                )
                for n, (action, done_when, files) in enumerate(b["steps"], start=1)
            ],
            notes=[
                transition_note(
                    5,
                    "session-creator-05",
                    "Groomed by session-creator-05: intent, scope and criteria written.",
                    ts(13, 10),
                ),
                transition_note(
                    6,
                    "session-creator-05",
                    "Approved by session-creator-05: criteria covered, steps planned.",
                    # After the dependency it waits for is done (TSK22).
                    ts(16, 12),
                    note_id="NOTE-02",
                ),
            ],
            depends_on=[done_ids[0]],
        )
    )

    child_two = next_id()
    assert child_two == "TSK30", child_two
    ids.setdefault("draft", []).append(child_two)
    second_child = task(child_two, brief(child_two)["title"], "draft", 3)
    second_child["parent"] = ids["approved"][0]
    tasks.append(second_child)

    for item in tasks:
        if item["id"] == ids["approved"][0]:
            item["children"] = [child_one, child_two]

    for task_item in tasks:
        finalize(task_item)
    check_claims_follow_dependencies(tasks)

    return tasks, items


def main():
    if TASKS.exists():
        shutil.rmtree(TASKS)
    TASKS.mkdir(parents=True)
    tasks, items = build()
    for record in tasks + items:
        path = TASKS / f"{record['id']}-{record['slug']}.json"
        path.write_text(json.dumps(record, indent=2, ensure_ascii=False) + "\n")
        print(path.relative_to(ROOT))
    events = activity_events(tasks)
    check_activity(tasks, events)
    ACTIVITY.write_text(
        json.dumps({"events": events}, indent=2, ensure_ascii=False) + "\n"
    )
    print(ACTIVITY.relative_to(ROOT))
    runtime = runtime_record(tasks)
    check_runtime(runtime, tasks)
    RUNTIME.write_text(json.dumps(runtime, indent=2, ensure_ascii=False) + "\n")
    print(RUNTIME.relative_to(ROOT))


if __name__ == "__main__":
    main()
