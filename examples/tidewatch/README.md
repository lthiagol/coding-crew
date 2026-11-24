# Tidewatch

Tidewatch watches the tide gauges along the harbour and sends alerts to the
harbour masters when the water rises past a limit: a tide over the high
mark, or a storm surge.

## What it is made of

- `src/tidewatch/` — the thresholds each harbour sets, and the alert that
  goes to its master.
- `data/harbours.json` — the harbours, their masters and their limits.
- `data/gauges.csv` — the gauges, the harbour each one serves and when it was
  last read.

## Running it

    python3 -m tidewatch.alerts --check    # run one check over the gauges

## How the team works

An alert is sent only to a number that has confirmed it. A change to a
threshold is reviewed by the harbour office before it reaches the sea.
